//! Text shaping against an in-memory font compiled from the document.
//!
//! One compiled font is cached per document and master. The cache entry records
//! the document generation it was built from; a request for a newer generation
//! is served from the previous font with [`ShapedRun::stale`] set while a
//! debounced background thread recompiles. Advances of glyphs whose model width
//! changed since that compile are corrected from the model, so metrics editing
//! stays responsive without waiting for a recompile.
//!
//! Callers that hold a document lock must use the two-phase API — [`Shaper::plan`]
//! under the lock, [`Shaper::shape_planned`] outside it — because compiling and
//! shaping take far longer than any document mutation may be blocked for. Only
//! [`ShapePlan::needs_font`] plans require a copy of the model, so a burst of
//! keystrokes copies the font at most once per scheduled rebuild.

use std::{
    collections::{HashMap, HashSet},
    panic::{catch_unwind, AssertUnwindSafe},
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, MutexGuard, PoisonError,
    },
    thread,
    time::{Duration, Instant},
};

use fc_compile::{horizontal_kerning_warnings, preserve_merge, ExportOptions, ShapingCompile};
use fc_model::{FeaturesSource, Font, GlyphName, MasterId};
use harfrust::{Direction, Feature, Language, Script, ShapeOptions, ShaperData, UnicodeBuffer};
use read_fonts::{
    types::{GlyphId, Tag},
    FontRef, TableProvider,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Error;

/// Delay before a stale shaping font is recompiled in the background.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(300);

/// Warning code prefix used when a background rebuild failed.
pub const COMPILE_FAILED: &str = "compile-failed";

/// How long a request waits for another caller's first compile to land.
const FIRST_COMPILE_DEADLINE: Duration = Duration::from_secs(300);

/// How often a waiting request rechecks the entry; `notify_all` is the fast path.
const WAIT_POLL: Duration = Duration::from_millis(50);

/// Model advance widths of the glyphs one shaping request needs.
pub type ModelWidths = HashMap<GlyphName, f64>;

/// Identifies the compiled shaping font of one document and master.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontKey {
    /// Session document identifier.
    pub doc: u32,
    /// Master the font is compiled for.
    pub master: MasterId,
}

/// One shaping request in document terms.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShapeRequest {
    /// Text to shape.
    pub text: String,
    /// Feature tags to force on or off.
    pub features: Vec<(String, bool)>,
    /// ISO 15924 script code, or automatic detection when absent.
    pub script: Option<String>,
    /// BCP 47 language tag, or the shaper default when absent.
    pub language: Option<String>,
    /// `ltr`, `rtl`, `ttb`, or `btt`; automatic detection when absent.
    pub direction: Option<String>,
    /// Rendering size; positions are scaled from font units when present.
    pub size: Option<f64>,
}

/// One glyph of a shaped run.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ShapedGlyph {
    /// Glyph identifier in the compiled shaping font.
    pub gid: u32,
    /// Zero-based index of the first input byte of the cluster.
    pub cluster: u32,
    /// Horizontal advance.
    pub x_advance: f64,
    /// Vertical advance.
    pub y_advance: f64,
    /// Horizontal placement offset.
    pub x_offset: f64,
    /// Vertical placement offset.
    pub y_offset: f64,
}

/// A shaped run and the diagnostics collected while shaping it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct ShapedRun {
    /// Shaped glyphs in visual order.
    pub glyphs: Vec<ShapedGlyph>,
    /// Glyph names of `glyphs`, in the same order, when the font supplies them.
    pub names: Vec<Option<String>>,
    /// Non-fatal shaping diagnostics.
    ///
    /// A background rebuild that failed is reported as
    /// `compile-failed: <detail>` and keeps being reported until a rebuild
    /// succeeds, so the run stays explicably stale instead of silently so.
    pub warnings: Vec<String>,
    /// Whether the shaping font is older than the requested document generation.
    pub stale: bool,
    /// Units per em of the shaping font.
    pub units_per_em: u16,
    /// Size the positions are expressed in, or font units when absent.
    pub size: Option<f64>,
}

/// What one shaping request needs before it can be answered.
///
/// Produced by [`Shaper::plan`] while the caller still holds its document and
/// consumed by [`Shaper::shape_planned`] once the lock is released.
pub struct ShapePlan {
    entry: Arc<Entry>,
    generation: u64,
    kind: Option<PlanKind>,
    /// Whether this plan still owns the entry's build claim.
    claimed: bool,
}

impl ShapePlan {
    /// Whether [`Shaper::shape_planned`] needs a copy of the document font.
    ///
    /// This is false whenever the cached font already matches, whenever a
    /// rebuild for this generation or a newer one is already scheduled, and
    /// whenever another caller has claimed the build — so two concurrent
    /// requests for the same generation copy the font at most once between them.
    #[must_use]
    pub fn needs_font(&self) -> bool {
        matches!(
            self.kind,
            Some(PlanKind::Rebuild(_)) | Some(PlanKind::First)
        )
    }
}

impl Drop for ShapePlan {
    fn drop(&mut self) {
        if !self.claimed {
            return;
        }
        // The claim was never converted into a build; release it so the next
        // request can take it instead of waiting for a build nobody is running.
        let mut state = lock(&self.entry.state);
        state.release_claim(self.generation);
        drop(state);
        self.entry.published.notify_all();
    }
}

enum PlanKind {
    /// The cached font was built from the requested generation.
    Current(Arc<CompiledFont>),
    /// The cached font is older and a rebuild is already scheduled or hopeless.
    Stale(Arc<CompiledFont>),
    /// The cached font is older and this caller must schedule the rebuild.
    Rebuild(Arc<CompiledFont>),
    /// Nothing is cached; this caller must compile synchronously.
    First,
    /// Nothing is cached and another caller is already compiling it.
    Wait,
}

/// A cache of compiled shaping fonts shared by every shaping request.
pub struct Shaper {
    entries: Mutex<HashMap<FontKey, Arc<Entry>>>,
    debounce: Duration,
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Shaper {
    fn drop(&mut self) {
        // Background workers outlive this cache by up to one debounce plus one
        // compile; cancelling makes them exit before starting that compile.
        for entry in lock(&self.entries).values() {
            entry.cancel();
        }
    }
}

impl Shaper {
    /// Creates an empty cache using the default recompile debounce.
    #[must_use]
    pub fn new() -> Self {
        Self::with_debounce(DEFAULT_DEBOUNCE)
    }

    /// Creates an empty cache with an explicit recompile debounce.
    #[must_use]
    pub fn with_debounce(debounce: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            debounce,
        }
    }

    /// Decides what a shaping request for `generation` needs.
    ///
    /// This touches only the entry, so it is safe to call while a document lock
    /// is held. When [`ShapePlan::needs_font`] is true the caller must copy the
    /// document font and hand it to [`Shaper::shape_planned`]; the plan claims
    /// the build on the entry, so a concurrent request for the same generation
    /// waits for this one instead of copying and compiling a second time. The
    /// claim is released when the plan is consumed or dropped.
    #[must_use]
    pub fn plan(&self, key: &FontKey, generation: u64) -> ShapePlan {
        let entry = self.entry(key);
        let mut claimed = false;
        let kind = {
            let mut state = lock(&entry.state);
            match state.compiled.clone() {
                Some(compiled) if compiled.generation == generation => PlanKind::Current(compiled),
                Some(compiled)
                    if state.scheduled_for(generation)
                        || state.failed_for(generation)
                        || state.claimed_for(generation) =>
                {
                    PlanKind::Stale(compiled)
                }
                Some(compiled) => {
                    claimed = state.claim(generation);
                    PlanKind::Rebuild(compiled)
                }
                None if state.claimed_for(generation) => PlanKind::Wait,
                None => {
                    claimed = state.claim(generation);
                    PlanKind::First
                }
            }
        };
        ShapePlan {
            entry,
            generation,
            kind: Some(kind),
            claimed,
        }
    }

    /// Completes `plan` and shapes `request` against the selected font.
    ///
    /// `font` must be a copy of the document font whenever
    /// [`ShapePlan::needs_font`] was true; `widths` supplies the model advances
    /// used to correct the compiled ones, and is collected by
    /// [`widths_for_text`] while the document is still held. Neither compiling
    /// nor shaping touches the document, so this must run outside its lock.
    ///
    /// # Errors
    ///
    /// Returns an error when the plan needed a font and none was supplied, when
    /// a synchronous first compile fails, or when the compiled font cannot be
    /// parsed back.
    pub fn shape_planned(
        &self,
        mut plan: ShapePlan,
        font: Option<Font>,
        widths: &ModelWidths,
        request: &ShapeRequest,
    ) -> Result<ShapedRun, Error> {
        let entry = Arc::clone(&plan.entry);
        let generation = plan.generation;
        // A plan is consumed by value exactly once, so the kind is always here.
        let Some(kind) = plan.kind.take() else {
            return Err(Error::AbandonedCompile);
        };
        let (compiled, stale) = match kind {
            PlanKind::Current(compiled) => (compiled, false),
            PlanKind::Stale(compiled) => (compiled, true),
            PlanKind::Wait => {
                let compiled = await_publish(&entry)?;
                let stale = compiled.generation != generation;
                (compiled, stale)
            }
            PlanKind::Rebuild(compiled) => {
                let Some(font) = font else {
                    return Err(Error::MissingFont);
                };
                self.schedule(&entry, font, generation);
                plan.claimed = false;
                (compiled, true)
            }
            PlanKind::First => {
                let Some(font) = font else {
                    return Err(Error::MissingFont);
                };
                let compiled = Arc::new(CompiledFont::build(&font, &entry.key.master, generation)?);
                let mut state = lock(&entry.state);
                state.publish(&compiled);
                state.release_claim(generation);
                drop(state);
                plan.claimed = false;
                entry.published.notify_all();
                (compiled, false)
            }
        };
        let failure = lock(&entry.state)
            .failure
            .as_ref()
            .map(|failure| format!("{COMPILE_FAILED}: {}", failure.message));
        shape_run(&compiled, widths, request, stale, failure)
    }

    /// Shapes `request` with the font compiled for `key` at `generation`.
    ///
    /// This is the single-shot form for callers that already own `font` outside
    /// any lock, such as the command line and the tests. Handlers that hold a
    /// session lock must use [`Shaper::plan`] and [`Shaper::shape_planned`].
    ///
    /// # Errors
    ///
    /// Returns an error when the master is missing, when compilation fails, or
    /// when the compiled font cannot be parsed back.
    pub fn shape(
        &self,
        font: &Font,
        key: &FontKey,
        generation: u64,
        request: &ShapeRequest,
    ) -> Result<ShapedRun, Error> {
        if font.master(&key.master).is_none() {
            return Err(Error::MissingMaster(key.master.clone()));
        }
        let plan = self.plan(key, generation);
        let copy = plan.needs_font().then(|| font.clone());
        let widths = widths_for_text(font, &key.master, &request.text);
        self.shape_planned(plan, copy, &widths, request)
    }

    /// Returns whether a cached font matches `generation` exactly.
    #[must_use]
    pub fn is_current(&self, key: &FontKey, generation: u64) -> bool {
        let entries = lock(&self.entries);
        entries.get(key).is_some_and(|entry| {
            lock(&entry.state)
                .compiled
                .as_ref()
                .is_some_and(|compiled| compiled.generation == generation)
        })
    }

    /// Whether the cached shaping font for this key reused the previous
    /// outline, metric and `cmap` tables and rebuilt only the layout tables.
    ///
    /// The shaper's output never depends on this; it is the observable record
    /// that live editing takes the incremental layout path.
    #[must_use]
    pub fn used_incremental_layout(&self, key: &FontKey) -> Option<bool> {
        let entries = lock(&self.entries);
        let entry = entries.get(key)?;
        let state = lock(&entry.state);
        let compiled = state.compiled.as_ref()?;
        compiled.source.as_ref().map(|source| source.incremental)
    }

    /// Drops every compiled font belonging to one document.
    ///
    /// A rebuild that was already scheduled for the closed document is
    /// cancelled, so closing a large font does not leave a worker compiling it.
    pub fn forget(&self, doc: u32) {
        lock(&self.entries).retain(|key, entry| {
            if key.doc == doc {
                entry.cancel();
                false
            } else {
                true
            }
        });
    }

    fn entry(&self, key: &FontKey) -> Arc<Entry> {
        let mut entries = lock(&self.entries);
        Arc::clone(
            entries
                .entry(key.clone())
                .or_insert_with(|| Arc::new(Entry::new(key.clone()))),
        )
    }

    fn schedule(&self, entry: &Arc<Entry>, font: Font, generation: u64) {
        let mut state = lock(&entry.state);
        state.pending = Some(PendingBuild {
            generation,
            font: Box::new(font),
        });
        state.release_claim(generation);
        if state.worker.is_none() {
            let token = state.take_token();
            if self.spawn_worker(entry, token) {
                state.worker = Some(token);
            } else {
                state.pending = None;
            }
        }
    }

    fn spawn_worker(&self, entry: &Arc<Entry>, token: u64) -> bool {
        let entry = Arc::clone(entry);
        let debounce = self.debounce;
        match thread::Builder::new()
            .name("fontcarver-shaping".to_owned())
            .spawn(move || run_worker(&entry, token, debounce))
        {
            Ok(_handle) => true,
            Err(error) => {
                tracing::error!(%error, "could not start the shaping recompile worker");
                false
            }
        }
    }
}

/// Collects the model widths of the glyphs `text` needs.
///
/// [`Shaper::shape_planned`] corrects compiled advances with these, so a width
/// edit is visible while the rebuild is still running. The map covers only the
/// glyphs encoded by `text`, which keeps it cheap enough to build under a
/// document read lock.
#[must_use]
pub fn widths_for_text(font: &Font, master: &MasterId, text: &str) -> ModelWidths {
    let scalars: HashSet<u32> = text.chars().map(u32::from).collect();
    font.glyphs
        .iter()
        .filter(|(_, glyph)| {
            glyph
                .unicodes
                .iter()
                .any(|unicode| scalars.contains(unicode))
        })
        .filter_map(|(name, glyph)| {
            glyph
                .master_layer(master)
                .map(|layer| (name.clone(), layer.width))
        })
        .collect()
}

/// Collect current advances through a session's maintained reverse character map.
/// Work is proportional to text and matching glyphs, rather than font size.
#[must_use]
pub fn widths_for_text_indexed(
    font: &Font,
    index: &fc_model::UnicodeIndex,
    master: &MasterId,
    text: &str,
) -> ModelWidths {
    let scalars: HashSet<_> = text.chars().map(u32::from).collect();
    scalars
        .into_iter()
        .flat_map(|scalar| index.glyphs(scalar))
        .filter_map(|name| {
            font.glyph(name)?
                .master_layer(master)
                .map(|layer| (name.clone(), layer.width))
        })
        .collect()
}

struct Entry {
    key: FontKey,
    state: Mutex<EntryState>,
    /// Signalled whenever a compile lands or a build claim is released.
    published: Condvar,
    cancelled: AtomicBool,
    #[cfg(test)]
    live_workers: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    peak_workers: std::sync::atomic::AtomicUsize,
}

impl Entry {
    fn new(key: FontKey) -> Self {
        Self {
            key,
            state: Mutex::new(EntryState::default()),
            published: Condvar::new(),
            cancelled: AtomicBool::new(false),
            #[cfg(test)]
            live_workers: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            peak_workers: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.published.notify_all();
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Default)]
struct EntryState {
    compiled: Option<Arc<CompiledFont>>,
    pending: Option<PendingBuild>,
    building: Option<u64>,
    /// Token of the live worker, when one exists.
    worker: Option<u64>,
    /// Next worker token to hand out.
    next_token: u64,
    /// Newest generation a caller has taken responsibility for building.
    claimed: Option<u64>,
    failure: Option<BuildFailure>,
}

impl EntryState {
    /// Hands out the next worker token.
    ///
    /// A worker only ever clears its own token, so a worker that is exiting
    /// cannot clear the flag of the successor that replaced it.
    fn take_token(&mut self) -> u64 {
        let token = self.next_token;
        self.next_token = self.next_token.wrapping_add(1);
        token
    }

    /// Records that a caller will build `generation`, unless a newer claim holds.
    fn claim(&mut self, generation: u64) -> bool {
        if self.claimed_for(generation) {
            return false;
        }
        self.claimed = Some(generation);
        true
    }

    /// Drops a claim, but only the one `generation` took.
    fn release_claim(&mut self, generation: u64) {
        if self.claimed == Some(generation) {
            self.claimed = None;
        }
    }

    /// Whether a caller has claimed `generation` or something newer.
    fn claimed_for(&self, generation: u64) -> bool {
        self.claimed.is_some_and(|claimed| claimed >= generation)
    }

    /// Publishes `compiled` unless an equally new or newer font is published.
    ///
    /// Two compiles can be in flight at once — a background rebuild and a first
    /// compile, or two first compiles — and they can finish out of order, so an
    /// older result must never replace a newer one.
    fn publish(&mut self, compiled: &Arc<CompiledFont>) -> bool {
        if self
            .compiled
            .as_ref()
            .is_some_and(|current| current.generation >= compiled.generation)
        {
            return false;
        }
        self.compiled = Some(Arc::clone(compiled));
        self.failure = None;
        true
    }

    /// Whether a live worker will produce `generation` or something newer.
    fn scheduled_for(&self, generation: u64) -> bool {
        self.worker.is_some()
            && (self
                .pending
                .as_ref()
                .is_some_and(|pending| pending.generation >= generation)
                || self.building.is_some_and(|building| building >= generation))
    }

    /// Whether a build at least as new as `generation` already failed.
    ///
    /// Retrying the same broken generation on every keystroke would burn a core
    /// for nothing; the next edit bumps the generation and retries.
    fn failed_for(&self, generation: u64) -> bool {
        self.failure
            .as_ref()
            .is_some_and(|failure| failure.generation >= generation)
    }
}

struct PendingBuild {
    generation: u64,
    font: Box<Font>,
}

struct BuildFailure {
    generation: u64,
    message: String,
}

/// Clears an entry's worker bookkeeping even when the worker unwinds.
///
/// The guard runs after the worker has already released the flag in its loop, so
/// a successor may have taken over in between; clearing is therefore conditional
/// on still owning the token, which keeps "at most one worker per entry" true.
struct WorkerGuard<'a> {
    entry: &'a Arc<Entry>,
    token: u64,
}

impl Drop for WorkerGuard<'_> {
    fn drop(&mut self) {
        let mut state = lock(&self.entry.state);
        if state.worker == Some(self.token) {
            state.worker = None;
            state.building = None;
        }
    }
}

/// Counts the workers running for one entry so tests can assert the invariant.
#[cfg(test)]
struct WorkerCensus<'a> {
    entry: &'a Arc<Entry>,
}

#[cfg(test)]
impl<'a> WorkerCensus<'a> {
    fn new(entry: &'a Arc<Entry>) -> Self {
        let live = entry.live_workers.fetch_add(1, Ordering::AcqRel) + 1;
        entry.peak_workers.fetch_max(live, Ordering::AcqRel);
        Self { entry }
    }
}

#[cfg(test)]
impl Drop for WorkerCensus<'_> {
    fn drop(&mut self) {
        self.entry.live_workers.fetch_sub(1, Ordering::AcqRel);
    }
}

struct CompiledFont {
    bytes: Arc<[u8]>,
    generation: u64,
    names: Vec<Option<GlyphName>>,
    advances: Vec<f64>,
    units_per_em: u16,
    warnings: Vec<String>,
    source: Option<Arc<ShapingCompile>>,
}

impl CompiledFont {
    fn build(font: &Font, master: &MasterId, generation: u64) -> Result<Self, Error> {
        Self::rebuild(font, master, generation, None)
    }

    fn rebuild(
        font: &Font,
        master: &MasterId,
        generation: u64,
        previous: Option<&Self>,
    ) -> Result<Self, Error> {
        if font.master(master).is_none() {
            return Err(Error::MissingMaster(master.clone()));
        }
        let compiled = Arc::new(ShapingCompile::build(
            font,
            master,
            previous.and_then(|previous| previous.source.as_deref()),
        )?);
        let mut bytes = Arc::clone(&compiled.bytes);
        let mut warnings = horizontal_kerning_warnings(font, master);
        if font.features.source_of_truth == FeaturesSource::ImportedTables {
            // Shaping shares export's glyph-order-verified layout preservation.
            // Keep newly compiled metrics/outlines: preview has no need to
            // restore original glyph programs and must reflect current edits.
            let options = ExportOptions {
                master: Some(master.clone()),
                keep_tt_instructions: false,
                ..ExportOptions::default()
            };
            let (preserved_bytes, preserved) =
                preserve_merge(&compiled.bytes, font, &HashSet::new(), &options)?;
            bytes = preserved_bytes.into();
            warnings.extend(preserved);
        }
        let mut result = Self {
            advances: glyph_advances(&bytes)?,
            names: compiled.glyph_order.iter().cloned().map(Some).collect(),
            bytes,
            generation,
            units_per_em: font.info.units_per_em,
            warnings: Vec::new(),
            source: Some(compiled),
        };
        result.warnings = warnings
            .into_iter()
            .map(|warning| format!("{}: {}", warning.code, warning.message))
            .collect();
        Ok(result)
    }

    #[cfg(test)]
    fn from_static(
        compiled: fc_compile::StaticCompile,
        generation: u64,
        units_per_em: u16,
    ) -> Result<Self, Error> {
        let advances = glyph_advances(&compiled.bytes)?;
        let names = compiled.glyph_order.into_iter().map(Some).collect();
        Ok(Self {
            bytes: compiled.bytes.into(),
            generation,
            names,
            advances,
            units_per_em,
            warnings: Vec::new(),
            source: None,
        })
    }
}

fn run_worker(entry: &Arc<Entry>, token: u64, debounce: Duration) {
    #[cfg(test)]
    let _census = WorkerCensus::new(entry);
    let _guard = WorkerGuard { entry, token };
    loop {
        thread::sleep(debounce);
        if entry.is_cancelled() {
            return;
        }
        let pending = {
            let mut state = lock(&entry.state);
            match state.pending.take() {
                Some(pending) => {
                    state.building = Some(pending.generation);
                    Some(pending)
                }
                None => {
                    if state.worker == Some(token) {
                        state.worker = None;
                    }
                    None
                }
            }
        };
        let Some(pending) = pending else {
            return;
        };
        if entry.is_cancelled() {
            return;
        }
        // fontc and fea-rs are third-party code running on untrusted models;
        // a panic there must not wedge this entry into permanent staleness.
        let built = catch_unwind(AssertUnwindSafe(|| {
            let previous = lock(&entry.state).compiled.clone();
            CompiledFont::rebuild(
                &pending.font,
                &entry.key.master,
                pending.generation,
                previous.as_deref(),
            )
        }));
        if entry.is_cancelled() {
            return;
        }
        let mut state = lock(&entry.state);
        state.building = None;
        let mut published = false;
        match built {
            Ok(Ok(compiled)) => {
                published = state.publish(&Arc::new(compiled));
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, doc = entry.key.doc, "could not recompile the shaping font");
                state.failure = Some(BuildFailure {
                    generation: pending.generation,
                    message: error.to_string(),
                });
            }
            Err(payload) => {
                let message = panic_message(payload.as_ref());
                tracing::error!(message, doc = entry.key.doc, "the shaping compile panicked");
                state.failure = Some(BuildFailure {
                    generation: pending.generation,
                    message,
                });
            }
        }
        drop(state);
        if published {
            entry.published.notify_all();
        }
    }
}

/// Waits for the caller that claimed the first compile of an entry to publish it.
fn await_publish(entry: &Arc<Entry>) -> Result<Arc<CompiledFont>, Error> {
    let deadline = Instant::now() + FIRST_COMPILE_DEADLINE;
    let mut state = lock(&entry.state);
    loop {
        if let Some(compiled) = state.compiled.clone() {
            return Ok(compiled);
        }
        if state.claimed.is_none() || entry.is_cancelled() {
            return Err(Error::AbandonedCompile);
        }
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(Error::AbandonedCompile);
        };
        let (guard, _timeout) = entry
            .published
            .wait_timeout(state, remaining.min(WAIT_POLL))
            .unwrap_or_else(PoisonError::into_inner);
        state = guard;
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the shaping compile panicked".to_owned())
}

fn glyph_advances(bytes: &[u8]) -> Result<Vec<f64>, Error> {
    let face = FontRef::new(bytes).map_err(|error| Error::Parse(error.to_string()))?;
    let count = face
        .maxp()
        .map_err(|error| Error::Parse(error.to_string()))?
        .num_glyphs();
    let hmtx = face.hmtx().ok();
    let mut advances = Vec::with_capacity(usize::from(count));
    for gid in 0..count {
        advances.push(
            hmtx.as_ref()
                .and_then(|hmtx| hmtx.advance(GlyphId::new(u32::from(gid))))
                .map_or(0.0, f64::from),
        );
    }
    Ok(advances)
}

fn shape_run(
    compiled: &CompiledFont,
    widths: &ModelWidths,
    request: &ShapeRequest,
    stale: bool,
    failure: Option<String>,
) -> Result<ShapedRun, Error> {
    let mut warnings = compiled.warnings.clone();
    warnings.extend(failure);
    let face =
        FontRef::from_index(&compiled.bytes, 0).map_err(|error| Error::Parse(error.to_string()))?;
    let data = ShaperData::new(&face);
    let shaper = data.shaper(&face).build();

    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(&request.text);
    if let Some(direction) = request.direction.as_deref() {
        match Direction::from_str(direction) {
            Ok(direction) => buffer.set_direction(direction),
            Err(_) => warnings.push(format!("unknown direction `{direction}`")),
        }
    }
    if let Some(script) = request.script.as_deref() {
        match Script::from_str(script) {
            Ok(script) => buffer.set_script(script),
            Err(_) => warnings.push(format!("unknown script `{script}`")),
        }
    }
    if let Some(language) = request.language.as_deref() {
        match Language::new(language) {
            Some(language) => buffer.set_language(language),
            None => warnings.push(format!("unknown language `{language}`")),
        }
    }
    buffer.guess_segment_properties();

    let features = features(&request.features, &mut warnings);
    let shaped = shaper.shape(buffer, ShapeOptions::new().features(&features));

    let scale = request
        .size
        .filter(|size| *size > 0.0)
        .map(|size| size / f64::from(compiled.units_per_em.max(1)));
    let mut glyphs = Vec::with_capacity(shaped.glyph_infos().len());
    let mut names = Vec::with_capacity(shaped.glyph_infos().len());
    let mut undefined = 0_usize;
    for (info, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        let index = info.glyph_id as usize;
        let name = compiled.names.get(index).cloned().flatten();
        if info.glyph_id == 0 {
            undefined = undefined.saturating_add(1);
        }
        let x_advance = f64::from(position.x_advance)
            + width_correction(widths, name.as_ref(), compiled.advances.get(index).copied());
        let scale = scale.unwrap_or(1.0);
        glyphs.push(ShapedGlyph {
            gid: info.glyph_id,
            cluster: info.cluster,
            x_advance: x_advance * scale,
            y_advance: f64::from(position.y_advance) * scale,
            x_offset: f64::from(position.x_offset) * scale,
            y_offset: f64::from(position.y_offset) * scale,
        });
        names.push(name.map(|name| name.to_string()));
    }
    if undefined != 0 {
        warnings.push(format!("{undefined} characters have no glyph in this font"));
    }
    Ok(ShapedRun {
        glyphs,
        names,
        warnings,
        stale,
        units_per_em: compiled.units_per_em,
        size: request.size.filter(|size| *size > 0.0),
    })
}

/// Returns the advance delta between the model width and the compiled advance.
///
/// Width edits are visible immediately this way, even while the shaping font is
/// still being recompiled; adding a delta keeps positioning adjustments such as
/// kerning that the shaper already applied.
fn width_correction(
    widths: &ModelWidths,
    name: Option<&GlyphName>,
    compiled_advance: Option<f64>,
) -> f64 {
    let (Some(name), Some(compiled_advance)) = (name, compiled_advance) else {
        return 0.0;
    };
    let Some(width) = widths.get(name).copied() else {
        return 0.0;
    };
    if (width - compiled_advance).abs() > 0.5 {
        width - compiled_advance
    } else {
        0.0
    }
}

fn features(requested: &[(String, bool)], warnings: &mut Vec<String>) -> Vec<Feature> {
    let mut features = Vec::with_capacity(requested.len());
    for (tag, enabled) in requested {
        match Tag::new_checked(tag.as_bytes()) {
            Ok(tag) => features.push(Feature::new(tag, u32::from(*enabled), ..)),
            Err(_) => warnings.push(format!("invalid feature tag `{tag}`")),
        }
    }
    features
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::{atomic::Ordering, Arc};

    use fc_compile::{compile_static_with_metadata, CompileOptions};
    use fc_model::{FeatureBlock, Font, Glyph, Layer, MasterId};
    use read_fonts::{types::Tag, FontRef, TableProvider};

    use super::{
        lock, run_worker, shape_run, CompiledFont, Entry, FontKey, ModelWidths, PendingBuild,
        ShapeRequest, Shaper, WorkerGuard,
    };

    fn entry() -> Arc<Entry> {
        Arc::new(Entry::new(FontKey {
            doc: 1,
            master: MasterId("m".into()),
        }))
    }

    fn compiled(generation: u64) -> Arc<CompiledFont> {
        Arc::new(CompiledFont {
            bytes: Vec::new().into(),
            generation,
            names: Vec::new(),
            advances: Vec::new(),
            units_per_em: 1_000,
            warnings: Vec::new(),
            source: None,
        })
    }

    #[test]
    fn retained_names_survive_when_compiled_post_has_no_names() {
        let mut font = Font::new_empty("Post Three Shaping", 1_000);
        let master = font.default_master().id.clone();
        for (name, unicode) in [("f", Some(0x66)), ("i", Some(0x69)), ("f_i", None)] {
            let mut glyph = Glyph::new(name);
            glyph.unicodes.extend(unicode);
            glyph.layers.push(Layer::new_master(master.clone()));
            font.insert_glyph(glyph).expect("insert glyph");
        }
        font.features.features.push(FeatureBlock {
            tag: "liga".to_owned(),
            code: "sub f i by f_i;".to_owned(),
            ..FeatureBlock::default()
        });
        let mut output = compile_static_with_metadata(&font, &master, &CompileOptions::default())
            .expect("compile font");
        let post_offset = {
            let face = FontRef::new(&output.bytes).expect("parse font");
            face.table_directory()
                .table_records()
                .iter()
                .find(|record| record.tag() == Tag::new(b"post"))
                .map(|record| record.offset() as usize)
                .expect("post record")
        };
        output.bytes[post_offset..post_offset + 4].copy_from_slice(&0x0003_0000_u32.to_be_bytes());
        let face = FontRef::new(&output.bytes).expect("parse post 3 font");
        assert!(face
            .post()
            .expect("post")
            .glyph_name(read_fonts::types::GlyphId16::new(1))
            .is_none());

        let compiled = CompiledFont::from_static(output, 1, font.info.units_per_em)
            .expect("load compiled metadata");
        assert_eq!(
            compiled.names,
            [
                Some(".notdef".into()),
                Some("f".into()),
                Some("i".into()),
                Some("f_i".into()),
            ]
        );
        let run = shape_run(
            &compiled,
            &ModelWidths::new(),
            &ShapeRequest {
                text: "fi".to_owned(),
                ..ShapeRequest::default()
            },
            false,
            None,
        )
        .expect("shape post 3 font");
        assert_eq!(run.names, [Some("f_i".to_owned())]);
    }

    #[test]
    fn the_worker_guard_clears_the_worker_flag_when_the_worker_panics() {
        let entry = entry();
        {
            let mut state = lock(&entry.state);
            state.worker = Some(7);
            state.building = Some(3);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = WorkerGuard {
                entry: &entry,
                token: 7,
            };
            panic!("the compile died");
        }));

        assert!(result.is_err());
        let state = lock(&entry.state);
        assert!(
            state.worker.is_none(),
            "a dead worker must not block later rebuilds"
        );
        assert!(state.building.is_none());
    }

    #[test]
    fn a_worker_guard_never_clears_a_successor_workers_token() {
        let entry = entry();
        lock(&entry.state).worker = Some(1);

        {
            let guard = WorkerGuard {
                entry: &entry,
                token: 1,
            };
            // The window: the exiting worker released the flag inside its loop
            // and a schedule spawned a successor before the guard ran.
            lock(&entry.state).worker = Some(2);
            drop(guard);
        }

        assert_eq!(
            lock(&entry.state).worker,
            Some(2),
            "the exiting worker clobbered its successor"
        );
    }

    #[test]
    fn concurrent_scheduling_never_runs_two_workers_for_one_entry() {
        let shaper = Arc::new(Shaper::with_debounce(std::time::Duration::from_millis(1)));
        let key = FontKey {
            doc: 9,
            master: MasterId("m".into()),
        };
        let entry = shaper.entry(&key);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(400);

        let threads: Vec<_> = (0..4_u64)
            .map(|thread| {
                let shaper = Arc::clone(&shaper);
                let entry = Arc::clone(&entry);
                std::thread::spawn(move || {
                    // An empty font fails to compile immediately, so the worker
                    // lifecycle churns without paying for a real compile.
                    let mut generation = thread * 1_000_000;
                    while std::time::Instant::now() < deadline {
                        generation += 1;
                        shaper.schedule(&entry, Font::new_empty("Stress", 1_000), generation);
                        std::thread::sleep(std::time::Duration::from_micros(200));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().expect("join a scheduling thread");
        }
        let drained = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while lock(&entry.state).worker.is_some() && std::time::Instant::now() < drained {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(
            entry.peak_workers.load(Ordering::Acquire) <= 1,
            "{} workers ran at once",
            entry.peak_workers.load(Ordering::Acquire)
        );
        assert_eq!(entry.live_workers.load(Ordering::Acquire), 0);
    }

    #[test]
    fn publishing_never_replaces_a_newer_font_with_an_older_one() {
        let entry = entry();
        let mut state = lock(&entry.state);

        assert!(state.publish(&compiled(5)));
        assert!(!state.publish(&compiled(3)), "an older compile landed");
        assert!(
            !state.publish(&compiled(5)),
            "the same generation landed twice"
        );
        assert!(state.publish(&compiled(6)));

        assert_eq!(
            state.compiled.as_ref().map(|compiled| compiled.generation),
            Some(6)
        );
    }

    #[test]
    fn a_cancelled_worker_exits_without_compiling() {
        let entry = entry();
        {
            let mut state = lock(&entry.state);
            state.worker = Some(0);
            // The master does not exist, so a compile would record a failure.
            state.pending = Some(PendingBuild {
                generation: 2,
                font: Box::new(Font::new_empty("Cancelled", 1_000)),
            });
        }
        entry.cancel();

        run_worker(&entry, 0, std::time::Duration::ZERO);

        let state = lock(&entry.state);
        assert!(state.failure.is_none(), "the worker compiled anyway");
        assert!(state.compiled.is_none());
        assert!(state.worker.is_none());
    }

    #[test]
    fn forgetting_a_document_cancels_its_entries() {
        let shaper = Shaper::new();
        let key = FontKey {
            doc: 4,
            master: MasterId("m".into()),
        };
        let entry = shaper.entry(&key);

        shaper.forget(4);

        assert!(entry.is_cancelled());
    }

    #[test]
    fn dropping_the_shaper_cancels_every_entry() {
        let shaper = Shaper::new();
        let entry = shaper.entry(&FontKey {
            doc: 5,
            master: MasterId("m".into()),
        });

        drop(shaper);

        assert!(entry.is_cancelled());
    }
}

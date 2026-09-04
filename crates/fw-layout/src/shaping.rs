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
        Arc, Mutex, MutexGuard, PoisonError,
    },
    thread,
    time::Duration,
};

use fw_compile::{compile_static, CompileOptions};
use fw_model::{Font, GlyphName, MasterId};
use harfrust::{Direction, Feature, Language, Script, ShapeOptions, ShaperData, UnicodeBuffer};
use read_fonts::{
    types::{GlyphId, GlyphId16, Tag},
    FontRef, TableProvider,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Error;

/// Delay before a stale shaping font is recompiled in the background.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(300);

/// Warning code prefix used when a background rebuild failed.
pub const COMPILE_FAILED: &str = "compile-failed";

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
    kind: PlanKind,
}

impl ShapePlan {
    /// Whether [`Shaper::shape_planned`] needs a copy of the document font.
    ///
    /// This is false whenever the cached font already matches, and whenever a
    /// rebuild for this generation or a newer one is already scheduled.
    #[must_use]
    pub fn needs_font(&self) -> bool {
        matches!(self.kind, PlanKind::Rebuild(_) | PlanKind::First)
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
    /// This only reads the cache, so it is safe to call while a document lock is
    /// held. When [`ShapePlan::needs_font`] is true the caller must copy the
    /// document font and hand it to [`Shaper::shape_planned`].
    #[must_use]
    pub fn plan(&self, key: &FontKey, generation: u64) -> ShapePlan {
        let entry = self.entry(key);
        let kind = {
            let state = lock(&entry.state);
            match state.compiled.clone() {
                Some(compiled) if compiled.generation == generation => PlanKind::Current(compiled),
                Some(compiled)
                    if state.scheduled_for(generation) || state.failed_for(generation) =>
                {
                    PlanKind::Stale(compiled)
                }
                Some(compiled) => PlanKind::Rebuild(compiled),
                None => PlanKind::First,
            }
        };
        ShapePlan {
            entry,
            generation,
            kind,
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
        plan: ShapePlan,
        font: Option<Font>,
        widths: &ModelWidths,
        request: &ShapeRequest,
    ) -> Result<ShapedRun, Error> {
        let ShapePlan {
            entry,
            generation,
            kind,
        } = plan;
        let (compiled, stale) = match kind {
            PlanKind::Current(compiled) => (compiled, false),
            PlanKind::Stale(compiled) => (compiled, true),
            PlanKind::Rebuild(compiled) => {
                let Some(font) = font else {
                    return Err(Error::MissingFont);
                };
                self.schedule(&entry, font, generation);
                (compiled, true)
            }
            PlanKind::First => {
                let Some(font) = font else {
                    return Err(Error::MissingFont);
                };
                let compiled = Arc::new(CompiledFont::build(&font, &entry.key.master, generation)?);
                let mut state = lock(&entry.state);
                state.compiled = Some(Arc::clone(&compiled));
                state.failure = None;
                drop(state);
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
        Arc::clone(entries.entry(key.clone()).or_insert_with(|| {
            Arc::new(Entry {
                key: key.clone(),
                state: Mutex::new(EntryState::default()),
                cancelled: AtomicBool::new(false),
            })
        }))
    }

    fn schedule(&self, entry: &Arc<Entry>, font: Font, generation: u64) {
        let mut state = lock(&entry.state);
        state.pending = Some(PendingBuild {
            generation,
            font: Box::new(font),
        });
        if !state.worker {
            if self.spawn_worker(entry) {
                state.worker = true;
            } else {
                state.pending = None;
            }
        }
    }

    fn spawn_worker(&self, entry: &Arc<Entry>) -> bool {
        let entry = Arc::clone(entry);
        let debounce = self.debounce;
        match thread::Builder::new()
            .name("fontworkshop-shaping".to_owned())
            .spawn(move || run_worker(&entry, debounce))
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

struct Entry {
    key: FontKey,
    state: Mutex<EntryState>,
    cancelled: AtomicBool,
}

impl Entry {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
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
    worker: bool,
    failure: Option<BuildFailure>,
}

impl EntryState {
    /// Whether a live worker will produce `generation` or something newer.
    fn scheduled_for(&self, generation: u64) -> bool {
        self.worker
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
struct WorkerGuard<'a> {
    entry: &'a Arc<Entry>,
}

impl Drop for WorkerGuard<'_> {
    fn drop(&mut self) {
        let mut state = lock(&self.entry.state);
        state.worker = false;
        state.building = None;
    }
}

struct CompiledFont {
    bytes: Vec<u8>,
    generation: u64,
    names: Vec<Option<GlyphName>>,
    advances: Vec<f64>,
    units_per_em: u16,
}

impl CompiledFont {
    fn build(font: &Font, master: &MasterId, generation: u64) -> Result<Self, Error> {
        if font.master(master).is_none() {
            return Err(Error::MissingMaster(master.clone()));
        }
        let bytes = compile_static(font, master, &CompileOptions::default())?;
        let (names, advances) = glyph_table(&bytes)?;
        Ok(Self {
            bytes,
            generation,
            names,
            advances,
            units_per_em: font.info.units_per_em,
        })
    }
}

fn run_worker(entry: &Arc<Entry>, debounce: Duration) {
    let _guard = WorkerGuard { entry };
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
                    state.worker = false;
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
            CompiledFont::build(&pending.font, &entry.key.master, pending.generation)
        }));
        if entry.is_cancelled() {
            return;
        }
        let mut state = lock(&entry.state);
        state.building = None;
        match built {
            Ok(Ok(compiled)) => {
                state.compiled = Some(Arc::new(compiled));
                state.failure = None;
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
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "the shaping compile panicked".to_owned())
}

fn glyph_table(bytes: &[u8]) -> Result<(Vec<Option<GlyphName>>, Vec<f64>), Error> {
    let face = FontRef::new(bytes).map_err(|error| Error::Parse(error.to_string()))?;
    let count = face
        .maxp()
        .map_err(|error| Error::Parse(error.to_string()))?
        .num_glyphs();
    let post = face.post().ok();
    let hmtx = face.hmtx().ok();
    let mut names = Vec::with_capacity(usize::from(count));
    let mut advances = Vec::with_capacity(usize::from(count));
    for gid in 0..count {
        names.push(
            post.as_ref()
                .and_then(|post| post.glyph_name(GlyphId16::new(gid)))
                .map(GlyphName::from),
        );
        advances.push(
            hmtx.as_ref()
                .and_then(|hmtx| hmtx.advance(GlyphId::new(u32::from(gid))))
                .map_or(0.0, f64::from),
        );
    }
    Ok((names, advances))
}

fn shape_run(
    compiled: &CompiledFont,
    widths: &ModelWidths,
    request: &ShapeRequest,
    stale: bool,
    failure: Option<String>,
) -> Result<ShapedRun, Error> {
    let mut warnings = Vec::new();
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
    use std::sync::{atomic::AtomicBool, Arc, Mutex};

    use fw_model::{Font, MasterId};

    use super::{lock, run_worker, Entry, EntryState, FontKey, PendingBuild, Shaper, WorkerGuard};

    fn entry() -> Arc<Entry> {
        Arc::new(Entry {
            key: FontKey {
                doc: 1,
                master: MasterId("m".into()),
            },
            state: Mutex::new(EntryState::default()),
            cancelled: AtomicBool::new(false),
        })
    }

    #[test]
    fn the_worker_guard_clears_the_worker_flag_when_the_worker_panics() {
        let entry = entry();
        {
            let mut state = lock(&entry.state);
            state.worker = true;
            state.building = Some(3);
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = WorkerGuard { entry: &entry };
            panic!("the compile died");
        }));

        assert!(result.is_err());
        let state = lock(&entry.state);
        assert!(!state.worker, "a dead worker must not block later rebuilds");
        assert!(state.building.is_none());
    }

    #[test]
    fn a_cancelled_worker_exits_without_compiling() {
        let entry = entry();
        {
            let mut state = lock(&entry.state);
            state.worker = true;
            // The master does not exist, so a compile would record a failure.
            state.pending = Some(PendingBuild {
                generation: 2,
                font: Box::new(Font::new_empty("Cancelled", 1_000)),
            });
        }
        entry.cancel();

        run_worker(&entry, std::time::Duration::ZERO);

        let state = lock(&entry.state);
        assert!(state.failure.is_none(), "the worker compiled anyway");
        assert!(state.compiled.is_none());
        assert!(!state.worker);
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

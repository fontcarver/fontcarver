//! Text shaping against an in-memory font compiled from the document.
//!
//! One compiled font is cached per document and master. The cache entry records
//! the document generation it was built from; a request for a newer generation
//! is served from the previous font with [`ShapedRun::stale`] set while a
//! debounced background thread recompiles. Advances of glyphs whose model width
//! changed since that compile are corrected from the model, so metrics editing
//! stays responsive without waiting for a recompile.

use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
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
    pub warnings: Vec<String>,
    /// Whether the shaping font is older than the requested document generation.
    pub stale: bool,
    /// Units per em of the shaping font.
    pub units_per_em: u16,
    /// Size the positions are expressed in, or font units when absent.
    pub size: Option<f64>,
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

    /// Shapes `request` with the font compiled for `key` at `generation`.
    ///
    /// The first request for a key compiles synchronously. Later requests for a
    /// newer generation are answered from the cached font with
    /// [`ShapedRun::stale`] set while the recompile runs in the background.
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
        let entry = self.entry(key);
        let (compiled, stale) = self.compiled_font(&entry, font, generation)?;
        shape_run(&compiled, font, &key.master, request, stale)
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
    pub fn forget(&self, doc: u32) {
        lock(&self.entries).retain(|key, _| key.doc != doc);
    }

    fn entry(&self, key: &FontKey) -> Arc<Entry> {
        let mut entries = lock(&self.entries);
        Arc::clone(entries.entry(key.clone()).or_insert_with(|| {
            Arc::new(Entry {
                key: key.clone(),
                state: Mutex::new(EntryState::default()),
            })
        }))
    }

    fn compiled_font(
        &self,
        entry: &Arc<Entry>,
        font: &Font,
        generation: u64,
    ) -> Result<(Arc<CompiledFont>, bool), Error> {
        let mut state = lock(&entry.state);
        match state.compiled.clone() {
            Some(compiled) if compiled.generation == generation => Ok((compiled, false)),
            Some(compiled) => {
                state.pending = Some(PendingBuild {
                    generation,
                    font: Box::new(font.clone()),
                });
                if !state.rebuilding {
                    state.rebuilding = self.spawn_worker(entry);
                    if !state.rebuilding {
                        state.pending = None;
                    }
                }
                Ok((compiled, true))
            }
            None => {
                let compiled = Arc::new(CompiledFont::build(font, &entry.key.master, generation)?);
                state.compiled = Some(Arc::clone(&compiled));
                Ok((compiled, false))
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

struct Entry {
    key: FontKey,
    state: Mutex<EntryState>,
}

#[derive(Default)]
struct EntryState {
    compiled: Option<Arc<CompiledFont>>,
    pending: Option<PendingBuild>,
    rebuilding: bool,
}

struct PendingBuild {
    generation: u64,
    font: Box<Font>,
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
    loop {
        thread::sleep(debounce);
        let pending = {
            let mut state = lock(&entry.state);
            match state.pending.take() {
                Some(pending) => Some(pending),
                None => {
                    state.rebuilding = false;
                    None
                }
            }
        };
        let Some(pending) = pending else {
            return;
        };
        match CompiledFont::build(&pending.font, &entry.key.master, pending.generation) {
            Ok(compiled) => lock(&entry.state).compiled = Some(Arc::new(compiled)),
            Err(error) => {
                tracing::warn!(%error, doc = entry.key.doc, "could not recompile the shaping font");
            }
        }
    }
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
    font: &Font,
    master: &MasterId,
    request: &ShapeRequest,
    stale: bool,
) -> Result<ShapedRun, Error> {
    let mut warnings = Vec::new();
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
            + width_correction(
                font,
                master,
                name.as_ref(),
                compiled.advances.get(index).copied(),
            );
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
    font: &Font,
    master: &MasterId,
    name: Option<&GlyphName>,
    compiled_advance: Option<f64>,
) -> f64 {
    let (Some(name), Some(compiled_advance)) = (name, compiled_advance) else {
        return 0.0;
    };
    let Some(width) = font
        .glyph(name)
        .and_then(|glyph| glyph.master_layer(master))
        .map(|layer| layer.width)
    else {
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

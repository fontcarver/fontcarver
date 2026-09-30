mod cff;
mod cmap;
mod container;
mod glyf;
mod hinting;
mod metrics;
mod names;
mod os2;
mod post;
mod raw;
mod variations;
mod vertical;

use std::{path::Path, sync::Arc};

use fc_model::{FeaturesSource, Font, SourceProvenance, Tag};
use read_fonts::{FontRef, TableProvider};
use sha2::{Digest, Sha256};

pub use container::{sniff, Container};

use crate::Error;

/// Options controlling sfnt import.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadOptions {
    /// Selected TTC face, or all faces when absent.
    pub font_index: Option<u32>,
    /// Keep decoded source bytes for preservation-based export.
    pub keep_original_bytes: bool,
    /// Decode editable embedded bitmap strikes; false keeps their original tables opaque.
    pub decode_bitmaps: bool,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            font_index: None,
            keep_original_bytes: true,
            decode_bitmaps: true,
        }
    }
}

/// A non-fatal issue encountered while importing a font.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    /// Stable machine-readable warning identifier.
    pub code: &'static str,
    /// Human-readable warning detail.
    pub message: String,
}

/// Fonts and non-fatal diagnostics produced by an import.
#[derive(Debug)]
pub struct ReadOutcome {
    /// Imported faces in collection order.
    pub fonts: Vec<Font>,
    /// Non-fatal diagnostics across all imported faces.
    pub warnings: Vec<Warning>,
}

/// Reads every selected face from in-memory container bytes.
pub fn read_bytes(bytes: &[u8], opts: &ReadOptions) -> Result<ReadOutcome, Error> {
    read_impl(bytes, None, opts)
}

/// Reads every selected face from a filesystem path.
pub fn read_path(path: &Path, opts: &ReadOptions) -> Result<ReadOutcome, Error> {
    let bytes = container::read_file(path)?;
    read_impl(&bytes, Some(path), opts)
}

/// Returns the number of faces in an sfnt or web-font container.
pub fn face_count(bytes: &[u8]) -> Result<u32, Error> {
    let (_, decoded) = container::decode(bytes)?;
    container::decoded_face_count(&decoded)
}

/// Returns the warning shown when a collection defaults to its first face.
///
/// Invalid or non-collection bytes return `None`; normal import reports their
/// decoding errors separately.
#[must_use]
pub fn collection_warning(bytes: &[u8]) -> Option<Warning> {
    if sniff(bytes) != Some(Container::Ttc) {
        return None;
    }
    let count = face_count(bytes).ok()?;
    (count > 1).then(|| Warning {
        code: "collection-face-opened",
        message: format!("collection has {count} faces; opened face 0"),
    })
}

/// Returns the best full name for every face in an sfnt or web-font container.
pub fn face_names(bytes: &[u8]) -> Result<Vec<String>, Error> {
    let (_, decoded) = container::decode(bytes)?;
    let count = container::decoded_face_count(&decoded)?;
    (0..count)
        .map(|index| {
            let face = FontRef::from_index(&decoded, index)
                .map_err(|error| Error::parse("sfnt", error))?;
            Ok(names::best_name(&face, 4).unwrap_or_else(|| format!("Face {}", index + 1)))
        })
        .collect()
}

fn read_impl(bytes: &[u8], path: Option<&Path>, opts: &ReadOptions) -> Result<ReadOutcome, Error> {
    let (container, decoded) = container::decode(bytes)?;
    let count = container::decoded_face_count(&decoded)?;
    let indexes: Box<dyn Iterator<Item = u32>> = match opts.font_index {
        Some(index) if index < count => Box::new(std::iter::once(index)),
        Some(index) => {
            return Err(Error::Parse {
                table: "TTC",
                reason: format!("face index {index} is out of range for {count} faces"),
            })
        }
        None => Box::new(0..count),
    };
    let source_bytes = Arc::new(decoded.into_owned());
    let hash = format!("{:x}", Sha256::digest(source_bytes.as_slice()));
    let mut warnings = Vec::new();
    let mut fonts = Vec::with_capacity(indexes.size_hint().0);

    for index in indexes {
        let face = FontRef::from_index(source_bytes.as_slice(), index)
            .map_err(|error| Error::parse("sfnt", error))?;
        let mut font = read_face(&face, &mut warnings)?;
        if opts.decode_bitmaps {
            font.bitmap_strikes = crate::bitmap::read_strikes(&face, &font)?;
            font.bitmap_tables_owned = [b"EBLC", b"CBLC", b"sbix"]
                .iter()
                .any(|tag| face.table_data(read_fonts::types::Tag::new(tag)).is_some());
        }
        let format = source_format(container, &face);
        let glyph_order = font.glyph_order().cloned().collect();
        font.extra.source = Some(SourceProvenance {
            path: path.map(|path| path.to_string_lossy().into_owned()),
            format: format.to_owned(),
            sha256: hash.clone(),
            table_tags: raw::table_tags(&face),
            glyph_order,
            original_bytes: opts.keep_original_bytes.then(|| Arc::clone(&source_bytes)),
        });
        fonts.push(font);
    }
    Ok(ReadOutcome { fonts, warnings })
}

fn source_format(container: Container, face: &FontRef<'_>) -> &'static str {
    match container {
        Container::Ttc => "ttc",
        Container::Woff => "woff",
        Container::Woff2 => "woff2",
        Container::Sfnt => {
            if face
                .table_data(read_fonts::types::Tag::new(b"CFF "))
                .is_some()
                || face
                    .table_data(read_fonts::types::Tag::new(b"CFF2"))
                    .is_some()
            {
                "otf"
            } else {
                "ttf"
            }
        }
    }
}

fn read_face(face: &FontRef<'_>, warnings: &mut Vec<Warning>) -> Result<Font, Error> {
    let head = face.head().map_err(|error| Error::parse("head", error))?;
    let maxp = face.maxp().map_err(|error| Error::parse("maxp", error))?;
    let hmtx = face.hmtx().map_err(|error| Error::parse("hmtx", error))?;
    let glyph_count = maxp.num_glyphs();

    let mut font = Font {
        info: names::font_info(face, &head)?,
        ..Font::default()
    };
    os2::apply(face, &mut font.info)?;
    post::apply(face, &mut font.info)?;
    metrics::apply_global(face, &mut font.info)?;
    variations::apply(face, &mut font)?;

    let glyph_data = cmap::glyph_data(face, glyph_count)?;
    let glyph_names = names::glyph_names(face, glyph_count, &glyph_data.unicodes, warnings)?;
    let mut master = metrics::master(face, &font.info)?;
    master.name = font.info.style_name.clone();
    master.location = variations::default_location(&font.axes);
    hinting::apply(face, &maxp, &mut master.tt)?;
    let private_dict_has_hints = cff::apply_private(face, &mut font.info, &mut master, warnings)?;
    let master_id = master.id.clone();
    font.masters = vec![master];
    font.cid = cff::cid_info(face, glyph_count, &glyph_names)?;

    let vertical = vertical::VerticalMetrics::new(face)?;
    let glyf_source = glyf::GlyfSource::new(face)?;
    let cff_source = cff::CffSource::new(face, private_dict_has_hints)?;

    for gid in 0..glyph_count {
        let mut glyph = fc_model::Glyph::new(glyph_names[usize::from(gid)].clone());
        glyph.unicodes = glyph_data.unicodes[usize::from(gid)].clone();
        glyph.variation_selectors = glyph_data.variation_selectors[usize::from(gid)].clone();
        let mut layer = fc_model::Layer::new_master(master_id.clone());
        layer.width = f64::from(
            hmtx.advance(read_fonts::types::GlyphId::new(u32::from(gid)))
                .ok_or_else(|| Error::Parse {
                    table: "hmtx",
                    reason: format!("missing advance for glyph {gid}"),
                })?,
        );
        vertical.apply(gid, &mut layer);
        if let Some(source) = &glyf_source {
            source.apply(gid, &glyph_names, &mut layer)?;
        } else if let Some(source) = &cff_source {
            source.apply(gid, &mut layer)?;
        }
        infer_smooth_points(&mut layer.contours);
        glyph.layers.push(layer);
        font.glyphs.insert(glyph.name.clone(), glyph);
    }
    if let Some(source) = &glyf_source {
        source.normalize_origins(face, &mut font)?;
    }
    metrics::apply_height_fallbacks(&mut font);
    if let Err(error) = crate::tt::imported_bindings(face, &mut font) {
        warnings.push(Warning {
            code: "tt-instruction-bindings-partial",
            message: format!(
                "Some imported TrueType programs could not be bound for safe editing: {error}"
            ),
        });
    }
    if cff_source.as_ref().is_some_and(cff::CffSource::lost_hints) {
        warnings.push(Warning {
            code: "cff-hints-not-imported",
            message: "Some CFF stem data could not be represented after font-matrix or seac transformation"
                .to_owned(),
        });
    }
    if cff_source
        .as_ref()
        .is_some_and(cff::CffSource::unpreserved_private_dicts)
    {
        warnings.push(Warning {
            code: "cff-private-dicts-flattened",
            message: "The source has multiple CFF2 private dictionaries; only the default dictionary is imported as master parameters. Per-glyph stems are retained, but variable private dictionaries are not preserved.".into(),
        });
    }
    font.extra.raw_tables = raw::raw_tables(face);
    decode_math(&mut font, &glyph_names, warnings);
    decode_color(&mut font, &glyph_names, warnings);
    decode_svg(&mut font, &glyph_names, warnings);
    if [
        *b"GSUB", *b"GPOS", *b"GDEF", *b"BASE", *b"kern", *b"kerx", *b"morx", *b"mort", *b"JSTF",
    ]
    .iter()
    .any(|tag| font.extra.raw_tables.contains_key(&Tag(*tag)))
    {
        font.features.source_of_truth = FeaturesSource::ImportedTables;
    }
    Ok(font)
}

/// Replace raw MATH with owned editable data when the whole table decodes.
///
/// Decoding is atomic: malformed or unsupported input keeps the raw bytes for
/// the existing preservation policy and never produces partial editable data.
fn decode_math(font: &mut Font, glyph_names: &[fc_model::GlyphName], warnings: &mut Vec<Warning>) {
    let tag = Tag(*b"MATH");
    let Some(bytes) = font.extra.raw_tables.get(&tag) else {
        return;
    };
    match crate::math::read_table(bytes, glyph_names) {
        Ok(math) => {
            font.math = Some(math);
            font.math_table_owned = true;
            font.extra.raw_tables.remove(&tag);
        }
        Err(error) => warnings.push(Warning {
            code: "math-not-editable",
            message: format!(
                "The MATH table was kept as imported bytes and is not editable: {error}"
            ),
        }),
    }
}

fn decode_color(font: &mut Font, order: &[fc_model::GlyphName], warnings: &mut Vec<Warning>) {
    let colr = font.extra.raw_tables.get(&Tag(*b"COLR"));
    let cpal = font.extra.raw_tables.get(&Tag(*b"CPAL"));
    if colr.is_none() && cpal.is_none() {
        return;
    }
    let result = cpal
        .ok_or_else(|| crate::Error::parse("COLR/CPAL", "missing CPAL table"))
        .and_then(|bytes| crate::color::read_tables(colr.map(Vec::as_slice), bytes, order));
    match result {
        Ok(color) => {
            font.color = Some(color);
            font.color_tables_owned = true;
            font.extra.raw_tables.remove(&Tag(*b"COLR"));
            font.extra.raw_tables.remove(&Tag(*b"CPAL"));
        }
        Err(error) => warnings.push(Warning {
            code: "color-not-editable",
            message: format!("COLR/CPAL were kept as imported bytes and are not editable: {error}"),
        }),
    }
}

fn infer_smooth_points(contours: &mut [fc_model::Contour]) {
    const SIN_ONE_DEGREE_SQUARED: f64 = 0.000_304_586_490_452_134_93;

    for contour in contours {
        let len = contour.points.len();
        let smooth: Vec<_> = (0..len)
            .map(|index| {
                let point = &contour.points[index];
                if point.kind == fc_model::PointKind::OffCurve {
                    return false;
                }
                let previous = index
                    .checked_sub(1)
                    .or_else(|| contour.closed.then(|| len.saturating_sub(1)));
                let next = (index + 1 < len)
                    .then_some(index + 1)
                    .or_else(|| contour.closed.then_some(0));
                let (Some(previous), Some(next)) = (previous, next) else {
                    return false;
                };
                let previous = &contour.points[previous];
                let next = &contour.points[next];
                if previous.kind != fc_model::PointKind::OffCurve
                    || next.kind != fc_model::PointKind::OffCurve
                {
                    return false;
                }

                let incoming = (previous.x - point.x, previous.y - point.y);
                let outgoing = (next.x - point.x, next.y - point.y);
                let incoming_len_squared = incoming.0 * incoming.0 + incoming.1 * incoming.1;
                let outgoing_len_squared = outgoing.0 * outgoing.0 + outgoing.1 * outgoing.1;
                if incoming_len_squared <= f64::EPSILON || outgoing_len_squared <= f64::EPSILON {
                    return false;
                }
                let dot = incoming.0 * outgoing.0 + incoming.1 * outgoing.1;
                let cross = incoming.0 * outgoing.1 - incoming.1 * outgoing.0;
                dot < 0.0
                    && cross * cross
                        <= SIN_ONE_DEGREE_SQUARED * incoming_len_squared * outgoing_len_squared
            })
            .collect();
        for (point, smooth) in contour.points.iter_mut().zip(smooth) {
            point.smooth = smooth;
        }
    }
}

fn decode_svg(font: &mut Font, order: &[fc_model::GlyphName], warnings: &mut Vec<Warning>) {
    let tag = fc_model::Tag(*b"SVG ");
    let Some(bytes) = font.extra.raw_tables.get(&tag) else {
        return;
    };
    match crate::svg::read_table(bytes, order) {
        Ok(svg) => {
            font.svg = Some(svg);
            font.svg_table_owned = true;
            font.extra.raw_tables.remove(&tag);
        }
        Err(error) => warnings.push(Warning {
            code: "svg-not-editable",
            message: format!("SVG retained as imported bytes: {error}"),
        }),
    }
}

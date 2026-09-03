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

use fw_model::{FeaturesSource, Font, SourceProvenance, Tag};
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
    /// Decode embedded bitmap strikes. P0 leaves them as raw tables.
    pub decode_bitmaps: bool,
}

impl Default for ReadOptions {
    fn default() -> Self {
        Self {
            font_index: None,
            keep_original_bytes: true,
            decode_bitmaps: false,
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
        let format = source_format(container, &face);
        font.extra.source = Some(SourceProvenance {
            path: path.map(|path| path.to_string_lossy().into_owned()),
            format: format.to_owned(),
            sha256: hash.clone(),
            table_tags: raw::table_tags(&face),
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
    let glyph_names = names::glyph_names(face, glyph_count, warnings)?;
    let mut master = metrics::master(face, &font.info)?;
    master.name = font.info.style_name.clone();
    master.location = variations::default_location(&font.axes);
    hinting::apply(face, &maxp, &mut master.tt)?;
    let private_dict_has_hints =
        cff::apply_private(face, &mut font.info, &mut master.private_dict)?;
    let master_id = master.id.clone();
    font.masters = vec![master];
    font.cid = cff::cid_info(face, glyph_count, &glyph_names)?;

    let vertical = vertical::VerticalMetrics::new(face)?;
    let glyf_source = glyf::GlyfSource::new(face)?;
    let cff_source = cff::CffSource::new(face, private_dict_has_hints)?;

    for gid in 0..glyph_count {
        let mut glyph = fw_model::Glyph::new(glyph_names[usize::from(gid)].clone());
        glyph.unicodes = glyph_data.unicodes[usize::from(gid)].clone();
        glyph.variation_selectors = glyph_data.variation_selectors[usize::from(gid)].clone();
        let mut layer = fw_model::Layer::new_master(master_id.clone());
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
        glyph.layers.push(layer);
        font.glyphs.insert(glyph.name.clone(), glyph);
    }
    metrics::apply_height_fallbacks(&mut font);
    if cff_source.as_ref().is_some_and(cff::CffSource::has_hints) {
        warnings.push(Warning {
            code: "cff-hints-not-imported",
            message: "CFF outlines were imported without per-glyph stem and hint-mask data"
                .to_owned(),
        });
    }
    font.extra.raw_tables = raw::raw_tables(face);
    if [Tag(*b"GSUB"), Tag(*b"GPOS"), Tag(*b"GDEF")]
        .iter()
        .any(|tag| font.extra.raw_tables.contains_key(tag))
    {
        font.features.source_of_truth = FeaturesSource::ImportedTables;
    }
    Ok(font)
}

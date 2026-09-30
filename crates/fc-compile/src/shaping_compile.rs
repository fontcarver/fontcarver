//! Retain outline/metric tables while refreshing the static layout work graph.

use std::sync::Arc;

use fc_model::{FeaturesSource, Font, GlyphName, MasterId};
use read_fonts::{types::Tag, FontRef};
use write_fonts::FontBuilder;

use crate::{compile_static_with_metadata, CompileOptions, Error};

const LAYOUT_TABLES: [Tag; 4] = [
    Tag::new(b"GSUB"),
    Tag::new(b"GPOS"),
    Tag::new(b"GDEF"),
    Tag::new(b"BASE"),
];

/// A compiled shaping font with the dependency snapshot needed for layout reuse.
///
/// Initial compilation and edits outside layout use the normal font compiler.
/// Subsequent layout edits retain outlines: HarfRust uses their extents for
/// fallback mark placement and their points for contour-index attachments.
#[derive(Clone)]
pub struct ShapingCompile {
    /// Complete sfnt bytes, including original non-layout tables.
    pub bytes: Arc<[u8]>,
    /// Exact model identities in glyph-ID order.
    pub glyph_order: Arc<[GlyphName]>,
    /// Whether this build used only the layout work graph.
    pub incremental: bool,
    master: MasterId,
    basis: Font,
    reusable: bool,
}

impl ShapingCompile {
    /// Build a shaping font, reusing `previous` only when every non-layout
    /// dependency matches exactly. Imported tables retain the normal verified
    /// preservation path in the caller and never qualify for layout reuse.
    pub fn build(font: &Font, master: &MasterId, previous: Option<&Self>) -> Result<Self, Error> {
        let basis = non_layout_basis(font);
        let reusable = reusable_source(font);
        if let Some(previous) = previous.filter(|previous| {
            reusable
                && previous.reusable
                && previous.master == *master
                && previous.basis == basis
                // IndexMap equality does not include its insertion order.
                && previous.basis.glyph_order().eq(basis.glyph_order())
        }) {
            match crate::layout_compile::compile_layout(font, master, &previous.glyph_order) {
                Ok(tables) => {
                    let face = FontRef::new(&previous.bytes)
                        .map_err(|error| Error::parse("incremental shaping font", error))?;
                    let mut builder = FontBuilder::new();
                    for record in face.table_directory().table_records() {
                        let tag = record.tag();
                        if !LAYOUT_TABLES.contains(&tag) {
                            let data = face
                                .table_data(tag)
                                .ok_or_else(|| Error::parse("incremental shaping table", tag))?;
                            builder.add_raw(tag, data.as_bytes());
                        }
                    }
                    for (tag, bytes) in &tables {
                        builder.add_raw(*tag, bytes);
                    }
                    return Ok(Self {
                        bytes: builder.build().into(),
                        glyph_order: Arc::clone(&previous.glyph_order),
                        incremental: true,
                        master: master.clone(),
                        basis,
                        reusable,
                    });
                }
                // Metadata feature blocks need the normal compiler's table
                // overlays. Its validation also reports the authoritative
                // diagnostic for invalid layout source.
                Err(error) => {
                    tracing::debug!(%error, "layout refresh requires a full shaping build")
                }
            }
        }
        let compiled = compile_static_with_metadata(font, master, &CompileOptions::default())?;
        Ok(Self {
            bytes: compiled.bytes.into(),
            glyph_order: compiled.glyph_order.into(),
            incremental: false,
            master: master.clone(),
            basis,
            reusable,
        })
    }
}

fn non_layout_basis(font: &Font) -> Font {
    let mut basis = font.clone();
    basis.features = Default::default();
    basis.feature_settings = Default::default();
    basis.feature_draft = None;
    basis.feature_revision = 0;
    basis.kerning_groups = Default::default();
    for master in &mut basis.masters {
        master.kerning_h = Default::default();
        master.kerning_v = Default::default();
    }
    for glyph in basis.glyphs.values_mut() {
        for layer in &mut glyph.layers {
            layer.anchors.clear();
        }
    }
    basis
}

fn reusable_source(font: &Font) -> bool {
    if font.features.source_of_truth != FeaturesSource::Fea {
        return false;
    }
    let source = font.features.to_fea();
    // Conservatively exclude all table declarations and constructs which can
    // synthesize name records. This also protects removal of a previous table
    // overlay; examining only the newly compiled output cannot detect that.
    !["table", "featureNames", "cvParameters", "sizemenuname"]
        .iter()
        .any(|keyword| source.contains(keyword))
}

//! Establish layout ownership before offering horizontal kerning edits.

use std::{path::Path, sync::Arc};

use fc_model::{FeaturesSource, Font};
use fea_rs::{
    parse::{parse_root, SourceLoadError},
    GlyphMap, Kind, NodeOrToken,
};

/// Why stored horizontal kerning cannot currently be edited.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    thiserror::Error,
    serde::Serialize,
    serde::Deserialize,
    ts_rs::TS,
)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalKerningEditBlock {
    /// Imported layout tables remain authoritative until converted to source.
    #[error("the imported font's spacing rules cannot yet be edited")]
    ImportedLayout,
    /// Active source or its writer policy reserves kerning from model editing.
    #[error("kerning is controlled by feature source")]
    SourceOwnedKerning,
    /// Source syntax or includes prevent reliable ownership inspection.
    #[error("feature source could not be parsed to establish kerning ownership")]
    InvalidFeatureSource,
}

/// Check whether source ownership permits stored horizontal kerning edits.
///
/// Uses the same assembled FEA as compilation. Active `kern` feature blocks
/// and variations reserve kerning for source, including blocks referencing
/// named lookups. Disabling the model kerning writer also reserves ownership.
/// Disabled blocks, comments, and unrelated names do not.
/// Parse errors and unresolved includes fail closed; no files are read.
///
/// This pure check does not validate pair/group data, compile feature source,
/// or establish a shaped advance. Future authoring queries and commands must
/// both call it against their current font, before exposing or mutating pairs.
pub fn check_horizontal_editing(font: &Font) -> Result<(), HorizontalKerningEditBlock> {
    check_editing(font, "kern")
}

/// Check whether active source permits model-owned vertical kerning.
pub fn check_vertical_editing(font: &Font) -> Result<(), HorizontalKerningEditBlock> {
    check_editing(font, "vkrn")
}

fn check_editing(font: &Font, owned_tag: &str) -> Result<(), HorizontalKerningEditBlock> {
    use HorizontalKerningEditBlock as Block;

    if font.features.source_of_truth == FeaturesSource::ImportedTables {
        return Err(Block::ImportedLayout);
    }
    if !font.feature_settings.writer_policy.kerning {
        return Err(Block::SourceOwnedKerning);
    }
    let source: Arc<str> = font.features.to_fea().into();
    if source.trim().is_empty() {
        return Ok(());
    }
    let glyphs = GlyphMap::new(font.glyph_order().map(|name| name.as_ref()))
        .map_err(|_| Block::InvalidFeatureSource)?;
    const ROOT: &str = "fontcarver-kerning.fea";
    let (tree, diagnostics) = parse_root(
        ROOT.into(),
        Some(&glyphs),
        Box::new(move |path: &Path| {
            if path == Path::new(ROOT) {
                Ok(Arc::clone(&source))
            } else {
                Err(SourceLoadError::new(
                    path.to_owned(),
                    "external feature includes are unavailable for in-memory source",
                ))
            }
        }),
    )
    .map_err(|_| Block::InvalidFeatureSource)?;
    if diagnostics.has_errors() {
        return Err(Block::InvalidFeatureSource);
    }
    for statement in tree.root().iter_children() {
        if !matches!(statement.kind(), Kind::FeatureNode | Kind::VariationNode) {
            continue;
        }
        let tag = statement.as_node().and_then(|node| {
            node.iter_children()
                .find(|child| child.kind() == Kind::Tag)
                .and_then(NodeOrToken::token_text)
        });
        match tag {
            Some(tag) if tag == owned_tag => return Err(Block::SourceOwnedKerning),
            None => return Err(Block::InvalidFeatureSource),
            _ => {}
        }
    }
    Ok(())
}

//! Per-entry scope labels for the document history surface.
//!
//! The editing path must not pay for text the user only sees when the History
//! panel is open, so a scope is derived on demand from the snapshots an entry
//! already stores. Layer names come from comparing the entry's before and after
//! glyph values, so a layer is named only when that entry really changes it.
//!
//! Nothing here produces user-facing prose. A layer the font left unnamed is
//! reported as its [`LayerRole`], and the interface translates that; a label the
//! kernel spelled in English would be printed verbatim in every language.

use std::collections::BTreeMap;

use fc_model::{Glyph, GlyphName, LayerId, LayerKind};

use super::{Snapshot, UndoEntry};

/// What an unnamed layer is for, so the interface can name it in its language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerRole {
    /// The main layer of a design-space master.
    Master,
    /// The background layer of a master.
    Background,
    /// An intermediate source at an explicit design location.
    Intermediate,
    /// A conditional alternate layer.
    Alternate,
    /// A local smart-component source.
    Smart,
    /// A timestamped backup of a master layer.
    Backup,
    /// An application- or format-defined layer with no name of its own.
    Other,
}

impl LayerRole {
    /// Stable identifier for the interface's translation key.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Master => "master",
            Self::Background => "background",
            Self::Intermediate => "intermediate",
            Self::Alternate => "alternate",
            Self::Smart => "smart",
            Self::Backup => "backup",
            Self::Other => "other",
        }
    }
}

/// One layer a history entry changes: its own name, or its role when unnamed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerScope {
    /// The layer's display name, exactly as the font spells it.
    Named(String),
    /// The layer has no name, so only its role can be reported.
    Role(LayerRole),
}

/// Which part of a document one history entry restores.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntryScope {
    /// Glyphs the entry restores, in snapshot order.
    pub glyphs: Vec<GlyphName>,
    /// The layers the entry changes, when it identifies them.
    pub layers: Vec<LayerScope>,
    /// Bitmap strikes the entry restores, as `(ppem, depth)`.
    pub strikes: Vec<(u16, u8)>,
    /// Whether font-wide data is restored: metadata, design space, layout,
    /// kerning, glyph order, font guides, palettes or embedded tables.
    pub font_level: bool,
}

impl EntryScope {
    /// Whether the entry restores nothing the user can be pointed at.
    ///
    /// An empty scope is possible for an entry whose snapshots were all shared
    /// away by coalescing, and the panel then falls back to the plain label.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
            && self.layers.is_empty()
            && self.strikes.is_empty()
            && !self.font_level
    }
}

pub(super) fn scope(entry: &UndoEntry) -> EntryScope {
    let mut scope = EntryScope::default();
    for snapshot in entry.inverse.iter().chain(&entry.forward) {
        match snapshot {
            Snapshot::Glyph(name, _) => push_glyph(&mut scope.glyphs, name),
            Snapshot::FeatureAnchors(state) => {
                push_glyph(&mut scope.glyphs, &state.glyph);
                let label = layer_label(glyph_value(entry, &state.glyph), &state.layer);
                push_layer(&mut scope.layers, label);
            }
            Snapshot::BitmapGlyph(ppem, depth, name, _) => {
                push_glyph(&mut scope.glyphs, name);
                if !scope.strikes.contains(&(*ppem, *depth)) {
                    scope.strikes.push((*ppem, *depth));
                }
            }
            Snapshot::Svg(..)
            | Snapshot::Math(..)
            | Snapshot::Color(..)
            | Snapshot::Cid(_)
            | Snapshot::DesignSpace(_)
            | Snapshot::BitmapStrikes(..)
            | Snapshot::Features(_)
            | Snapshot::Kerning(_)
            | Snapshot::FontInfo(_)
            | Snapshot::Master(..)
            | Snapshot::GlyphOrder(_)
            | Snapshot::FontGuides(_) => scope.font_level = true,
        }
    }
    // COLR, SVG and MATH are font-wide tables keyed by glyph name, so an edit
    // to one glyph's color, SVG or math record would otherwise read as nothing
    // but "font-level". The keys that actually differ name the glyphs it is.
    for name in changed_table_glyphs(entry) {
        push_glyph(&mut scope.glyphs, &name);
    }
    for label in changed_layers(entry) {
        push_layer(&mut scope.layers, label);
    }
    scope
}

/// Glyph names whose color, SVG or math record differs across the entry.
fn changed_table_glyphs(entry: &UndoEntry) -> Vec<GlyphName> {
    let mut names = Vec::new();
    for (before, after) in [
        (color_records(&entry.inverse), color_records(&entry.forward)),
        (svg_records(&entry.inverse), svg_records(&entry.forward)),
        (math_records(&entry.inverse), math_records(&entry.forward)),
    ] {
        let (Some(before), Some(after)) = (before, after) else {
            continue;
        };
        for name in before.keys().chain(after.keys()) {
            if before.get(name) != after.get(name) && !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    names
}

/// Per-glyph COLR records: the v1 root paint, its clip box and the v0 layers.
fn color_records(snapshots: &[Snapshot]) -> Option<BTreeMap<GlyphName, String>> {
    snapshots.iter().find_map(|snapshot| match snapshot {
        Snapshot::Color(value, _) => Some(value.as_ref().map_or_else(BTreeMap::new, |table| {
            let mut records: BTreeMap<GlyphName, String> = BTreeMap::new();
            for (name, layers) in &table.glyphs {
                records.insert(name.clone(), format!("{layers:?}"));
            }
            if let Some(graph) = &table.graph {
                for (name, root) in &graph.glyphs {
                    let clip = graph.clips.get(name);
                    records
                        .entry(name.clone())
                        .or_default()
                        .push_str(&format!("|{root}|{clip:?}"));
                }
            }
            records
        })),
        _ => None,
    })
}

/// Per-glyph SVG bindings: the document source and the element the glyph uses.
fn svg_records(snapshots: &[Snapshot]) -> Option<BTreeMap<GlyphName, String>> {
    snapshots.iter().find_map(|snapshot| match snapshot {
        Snapshot::Svg(value, _) => Some(value.as_ref().map_or_else(BTreeMap::new, |table| {
            let mut records = BTreeMap::new();
            for document in &table.documents {
                for (name, element) in &document.glyphs {
                    records.insert(name.clone(), format!("{element}|{}", document.source.len()));
                }
            }
            records
        })),
        _ => None,
    })
}

/// Per-glyph MATH records.
fn math_records(snapshots: &[Snapshot]) -> Option<BTreeMap<GlyphName, String>> {
    snapshots.iter().find_map(|snapshot| match snapshot {
        Snapshot::Math(value, _) => Some(value.as_ref().map_or_else(BTreeMap::new, |table| {
            table
                .glyphs
                .iter()
                .map(|(name, info)| (name.clone(), format!("{info:?}")))
                .collect()
        })),
        _ => None,
    })
}

fn push_glyph(names: &mut Vec<GlyphName>, name: &GlyphName) {
    if !names.contains(name) {
        names.push(name.clone());
    }
}

fn push_layer(layers: &mut Vec<LayerScope>, label: Option<LayerScope>) {
    if let Some(label) = label {
        if !layers.contains(&label) {
            layers.push(label);
        }
    }
}

/// The glyph value stored for `name`, preferring the state before the edit.
fn glyph_value<'a>(entry: &'a UndoEntry, name: &GlyphName) -> Option<&'a Glyph> {
    find_glyph(&entry.inverse, name).or_else(|| find_glyph(&entry.forward, name))
}

fn find_glyph<'a>(snapshots: &'a [Snapshot], name: &GlyphName) -> Option<&'a Glyph> {
    snapshots.iter().find_map(|snapshot| match snapshot {
        Snapshot::Glyph(stored, value) if stored == name => value.as_deref(),
        _ => None,
    })
}

/// A layer's display name, falling back to its role.
///
/// Many fonts leave a master layer unnamed, and a persistent layer identifier is
/// a UUID: showing one would tell the reader nothing. The role is what the reader
/// can act on, and the interface spells it in their language.
fn layer_label(glyph: Option<&Glyph>, id: &LayerId) -> Option<LayerScope> {
    let layer = glyph.and_then(|glyph| glyph.layers.iter().find(|layer| &layer.id == id))?;
    if !layer.name.is_empty() {
        return Some(LayerScope::Named(layer.name.clone()));
    }
    Some(match &layer.kind {
        LayerKind::Master(_) => LayerScope::Role(LayerRole::Master),
        LayerKind::Background(_) => LayerScope::Role(LayerRole::Background),
        LayerKind::Brace { .. } => LayerScope::Role(LayerRole::Intermediate),
        LayerKind::Bracket { .. } => LayerScope::Role(LayerRole::Alternate),
        LayerKind::Smart { .. } => LayerScope::Role(LayerRole::Smart),
        LayerKind::Backup { .. } => LayerScope::Role(LayerRole::Backup),
        LayerKind::Custom(name) if !name.is_empty() => LayerScope::Named(name.clone()),
        LayerKind::Custom(_) => LayerScope::Role(LayerRole::Other),
    })
}

/// Layers whose content differs between the entry's before and after glyphs.
///
/// A glyph that is created or deleted by the entry has no layer scope: the
/// glyph itself is the scope, and listing every one of its layers would say
/// less, not more.
fn changed_layers(entry: &UndoEntry) -> Vec<Option<LayerScope>> {
    let mut labels = Vec::new();
    for snapshot in &entry.inverse {
        let Snapshot::Glyph(name, Some(before)) = snapshot else {
            continue;
        };
        let Some(after) = find_glyph(&entry.forward, name) else {
            continue;
        };
        for layer in &before.layers {
            let changed = match after.layers.iter().find(|other| other.id == layer.id) {
                Some(other) => other != layer,
                None => true,
            };
            if changed {
                labels.push(layer_label(Some(before), &layer.id));
            }
        }
        for layer in &after.layers {
            if !before.layers.iter().any(|other| other.id == layer.id) {
                labels.push(layer_label(Some(after), &layer.id));
            }
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fc_model::{Glyph, GlyphName, Layer, LayerKind};

    use super::*;
    use crate::{undo::DesignSpaceSnapshot, Change};

    fn entry(inverse: Vec<Snapshot>, forward: Vec<Snapshot>) -> UndoEntry {
        UndoEntry::new(
            "Edit".to_owned(),
            inverse,
            forward,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Change::default(),
            false,
            0,
            1,
        )
    }

    fn glyph(name: &str, layers: Vec<Layer>) -> Glyph {
        let mut glyph = Glyph::new(GlyphName::from(name));
        glyph.layers = layers;
        glyph
    }

    fn layer(id: &str, name: &str, width: f64) -> Layer {
        Layer {
            id: LayerId(id.into()),
            kind: LayerKind::Master(fc_model::MasterId("regular".into())),
            name: name.to_owned(),
            width,
            ..Layer::default()
        }
    }

    #[test]
    fn a_glyph_edit_names_the_glyph_and_only_the_changed_layer() {
        let before = glyph(
            "A",
            vec![
                layer("m1", "Regular", 500.0),
                layer("bg", "Background", 0.0),
            ],
        );
        let mut after = before.clone();
        after.layers[0].width = 540.0;
        let entry = entry(
            vec![Snapshot::Glyph(
                GlyphName::from("A"),
                Some(Arc::new(before)),
            )],
            vec![Snapshot::Glyph(GlyphName::from("A"), Some(Arc::new(after)))],
        );

        let scope = entry.scope();
        assert_eq!(scope.glyphs, vec![GlyphName::from("A")]);
        assert_eq!(scope.layers, vec![LayerScope::Named("Regular".to_owned())]);
        assert!(!scope.font_level);
        assert!(!scope.is_empty());
    }

    #[test]
    fn creating_a_glyph_reports_the_glyph_without_layer_noise() {
        let after = glyph("B", vec![layer("m1", "Regular", 500.0)]);
        let entry = entry(
            vec![Snapshot::Glyph(GlyphName::from("B"), None)],
            vec![Snapshot::Glyph(GlyphName::from("B"), Some(Arc::new(after)))],
        );

        let scope = entry.scope();
        assert_eq!(scope.glyphs, vec![GlyphName::from("B")]);
        assert!(scope.layers.is_empty());
    }

    #[test]
    fn an_unnamed_layer_is_labelled_by_its_role_not_its_identifier() {
        let mut before = glyph("F", vec![layer("m1", "", 500.0)]);
        before.layers.push(Layer {
            id: LayerId("bg".into()),
            kind: LayerKind::Background(fc_model::MasterId("regular".into())),
            name: String::new(),
            ..Layer::default()
        });
        let mut after = before.clone();
        after.layers[0].width = 540.0;
        after.layers[1].width = 1.0;
        let entry = entry(
            vec![Snapshot::Glyph(
                GlyphName::from("F"),
                Some(Arc::new(before)),
            )],
            vec![Snapshot::Glyph(GlyphName::from("F"), Some(Arc::new(after)))],
        );

        let scope = entry.scope();
        assert_eq!(
            scope.layers,
            vec![
                LayerScope::Role(LayerRole::Master),
                LayerScope::Role(LayerRole::Background)
            ]
        );
        // A persistent identifier never reaches the interface.
        assert!(
            !scope.layers.iter().any(|label| matches!(
                label,
                LayerScope::Named(name) if name.contains("m1")
            )),
            "{:?}",
            scope.layers
        );
        assert_eq!(LayerRole::Master.as_str(), "master");
    }

    #[test]
    fn an_added_layer_is_named_from_the_new_value() {
        let before = glyph("C", vec![layer("m1", "Regular", 500.0)]);
        let mut after = before.clone();
        after.layers.push(layer("bg", "Background", 0.0));
        let entry = entry(
            vec![Snapshot::Glyph(
                GlyphName::from("C"),
                Some(Arc::new(before)),
            )],
            vec![Snapshot::Glyph(GlyphName::from("C"), Some(Arc::new(after)))],
        );

        assert_eq!(
            entry.scope().layers,
            vec![LayerScope::Named("Background".to_owned())]
        );
    }

    #[test]
    fn font_wide_snapshots_are_font_level() {
        let entry = entry(
            vec![Snapshot::DesignSpace(Arc::new(
                DesignSpaceSnapshot::default(),
            ))],
            vec![Snapshot::DesignSpace(Arc::new(
                DesignSpaceSnapshot::default(),
            ))],
        );
        let scope = entry.scope();
        assert!(scope.font_level);
        assert!(scope.glyphs.is_empty());
    }

    #[test]
    fn a_bitmap_edit_reports_its_glyph_and_strike() {
        let entry = entry(
            vec![Snapshot::BitmapGlyph(16, 1, GlyphName::from("D"), None)],
            vec![Snapshot::BitmapGlyph(16, 1, GlyphName::from("D"), None)],
        );
        let scope = entry.scope();
        assert_eq!(scope.glyphs, vec![GlyphName::from("D")]);
        assert_eq!(scope.strikes, vec![(16, 1)]);
    }

    #[test]
    fn an_anchor_edit_names_its_layer_even_without_a_glyph_diff() {
        let value = glyph("E", vec![layer("m1", "Regular", 500.0)]);
        let entry = entry(
            vec![
                Snapshot::Glyph(GlyphName::from("E"), Some(Arc::new(value.clone()))),
                Snapshot::FeatureAnchors(Arc::new(fc_model::FeatureAnchorState {
                    glyph: GlyphName::from("E"),
                    layer: LayerId("m1".into()),
                    anchors: Vec::new(),
                })),
            ],
            vec![Snapshot::Glyph(GlyphName::from("E"), Some(Arc::new(value)))],
        );

        let scope = entry.scope();
        assert_eq!(scope.glyphs, vec![GlyphName::from("E")]);
        assert_eq!(scope.layers, vec![LayerScope::Named("Regular".to_owned())]);
    }

    #[test]
    fn a_per_glyph_color_edit_names_its_glyph_as_well_as_the_font() {
        let table = |root: u32| {
            let mut graph = fc_model::ColrGraph::default();
            graph.glyphs.insert(GlyphName::from("G"), root);
            graph.glyphs.insert(GlyphName::from("H"), 9);
            fc_model::ColorTable {
                graph: Some(graph),
                palettes: Vec::new(),
                ..Default::default()
            }
        };
        let entry = entry(
            vec![Snapshot::Color(Some(Arc::new(table(1))), true)],
            vec![Snapshot::Color(Some(Arc::new(table(2))), true)],
        );

        let scope = entry.scope();
        // The table is font-wide, but only one glyph's record moved.
        assert!(scope.font_level);
        assert_eq!(scope.glyphs, vec![GlyphName::from("G")]);
    }

    #[test]
    fn a_per_glyph_svg_or_math_edit_names_its_glyph() {
        let svg = |element: &str| fc_model::SvgTable {
            documents: vec![fc_model::SvgDocument {
                source: "<svg/>".to_owned(),
                glyphs: [(GlyphName::from("I"), element.to_owned())]
                    .into_iter()
                    .collect(),
            }],
        };
        let bound = entry(
            vec![Snapshot::Svg(Some(Arc::new(svg("a"))), true)],
            vec![Snapshot::Svg(Some(Arc::new(svg("b"))), true)],
        );
        assert_eq!(bound.scope().glyphs, vec![GlyphName::from("I")]);

        let math = |advance: f64| {
            let mut table = fc_model::MathTable::default();
            table.glyphs.insert(
                GlyphName::from("J"),
                fc_model::MathGlyphInfo {
                    italic_correction: Some(advance.into()),
                    ..Default::default()
                },
            );
            table
        };
        let measured = entry(
            vec![Snapshot::Math(Some(Arc::new(math(1.0))), true)],
            vec![Snapshot::Math(Some(Arc::new(math(2.0))), true)],
        );
        assert_eq!(measured.scope().glyphs, vec![GlyphName::from("J")]);
    }

    #[test]
    fn an_unchanged_table_names_no_glyph() {
        let mut table = fc_model::MathTable::default();
        table
            .glyphs
            .insert(GlyphName::from("K"), fc_model::MathGlyphInfo::default());
        let value = Arc::new(table);
        let entry = entry(
            vec![Snapshot::Math(Some(Arc::clone(&value)), true)],
            vec![Snapshot::Math(Some(value), true)],
        );
        assert!(entry.scope().glyphs.is_empty());
        assert!(entry.scope().font_level);
    }

    #[test]
    fn an_empty_entry_has_an_empty_scope() {
        assert!(entry(Vec::new(), Vec::new()).scope().is_empty());
    }
}

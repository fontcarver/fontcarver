//! Top-level font document and mutation invariants.

use std::{
    collections::{BTreeMap, HashSet},
    sync::LazyLock,
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    BitmapStrike, Error, Extra, Features, FontInfo, Glyph, GlyphName, Guide, KernSide, Kerning,
    KerningGroups, Layer, Master, MasterId, Tag,
};

/// A location in normalized or design coordinates, keyed by axis tag.
pub type DesignLocation = BTreeMap<Tag, f64>;

/// One variation axis.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Axis {
    /// Four-byte axis tag.
    #[serde(default)]
    pub tag: Tag,
    /// Localized default axis name.
    #[serde(default)]
    pub name: String,
    /// Minimum user-space coordinate.
    #[serde(default)]
    pub min: f64,
    /// Default user-space coordinate.
    #[serde(default)]
    pub default: f64,
    /// Maximum user-space coordinate.
    #[serde(default)]
    pub max: f64,
    /// Whether interfaces should hide the axis.
    #[serde(default)]
    pub hidden: bool,
    /// User-to-design coordinate mapping.
    #[serde(default)]
    pub map: Vec<(f64, f64)>,
}

/// A named export instance.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Instance {
    /// Instance display name.
    #[serde(default)]
    pub name: String,
    /// Design-space location.
    #[serde(default, with = "design_location_serde")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number>"))]
    pub location: DesignLocation,
    /// Exported style name.
    #[serde(default)]
    pub style_name: String,
    /// Optional family-name override.
    #[serde(default)]
    pub family_name: Option<String>,
    /// Whether this instance participates in export.
    #[serde(default)]
    pub active: bool,
    /// Whether this is a named instance in the variable font.
    #[serde(default)]
    pub is_variable_named_instance: bool,
    /// Format-specific instance metadata.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub user_data: serde_json::Value,
}

pub(crate) mod design_location_serde {
    use std::{collections::BTreeMap, str::FromStr};

    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    use crate::{DesignLocation, Tag};

    pub(crate) fn serialize<S>(location: &DesignLocation, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        location
            .iter()
            .map(|(tag, value)| (tag.to_string(), value))
            .collect::<BTreeMap<_, _>>()
            .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<DesignLocation, D::Error>
    where
        D: Deserializer<'de>,
    {
        BTreeMap::<String, f64>::deserialize(deserializer)?
            .into_iter()
            .map(|(tag, value)| {
                Tag::from_str(&tag)
                    .map(|tag| (tag, value))
                    .map_err(D::Error::custom)
            })
            .collect()
    }
}

/// CID-keyed font metadata and CID-to-glyph mapping.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CidInfo {
    /// CID registry.
    #[serde(default)]
    pub registry: String,
    /// CID ordering.
    #[serde(default)]
    pub ordering: String,
    /// CID supplement number.
    #[serde(default)]
    pub supplement: u32,
    /// Mapping from CIDs to glyph names.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<number, string>"))]
    pub cid_map: BTreeMap<u32, GlyphName>,
}

/// A complete editable font document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Font {
    /// Font-wide names and metadata.
    #[serde(default)]
    pub info: FontInfo,
    /// Variation axes; empty for a static font.
    #[serde(default)]
    pub axes: Vec<Axis>,
    /// Interpolation masters.
    #[serde(default = "default_masters", deserialize_with = "deserialize_masters")]
    pub masters: Vec<Master>,
    /// Named export instances.
    #[serde(default)]
    pub instances: Vec<Instance>,
    /// Glyphs in persistent display/export order.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, Glyph>"))]
    pub glyphs: IndexMap<GlyphName, Glyph>,
    /// Font-wide kerning group membership.
    #[serde(default)]
    pub kerning_groups: KerningGroups,
    /// Editable OpenType feature source.
    #[serde(default)]
    pub features: Features,
    /// Saved editor draft, independent from effective layout source.
    #[serde(default)]
    pub feature_draft: Option<crate::FeatureDraft>,
    /// Active generation and writer policy.
    #[serde(default)]
    pub feature_settings: crate::FeatureGenerationSettings,
    /// Persistent monotonic editor revision, including discard and undo.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub feature_revision: u64,
    /// Original-layout metadata retained for native-document revert.
    #[serde(default)]
    pub layout_conversion: Option<crate::LayoutConversion>,
    /// Embedded bitmap strikes.
    #[serde(default)]
    pub bitmap_strikes: Vec<BitmapStrike>,
    /// Decoded/edited strikes own embedded tables, including an empty collection.
    #[serde(default)]
    pub bitmap_tables_owned: bool,
    /// Editable font-wide mathematical typesetting data.
    #[serde(default)]
    pub math: Option<crate::MathTable>,
    /// Whether editable MATH, including deletion, supersedes imported bytes.
    #[serde(default)]
    pub math_table_owned: bool,
    /// Editable COLR v0 compositions and CPAL palettes.
    #[serde(default)]
    pub color: Option<crate::ColorTable>,
    /// Editable color data, including deletion, supersedes imported COLR/CPAL.
    #[serde(default)]
    pub color_tables_owned: bool,
    /// CID metadata when this is a CID-keyed font.
    #[serde(default)]
    pub cid: Option<CidInfo>,
    /// Font-wide guides.
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Opaque imported data preserved for round trips.
    #[serde(default)]
    pub extra: Extra,
    /// Format-specific document metadata.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub user_data: serde_json::Value,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            info: FontInfo::default(),
            axes: Vec::new(),
            masters: default_masters(),
            instances: Vec::new(),
            glyphs: IndexMap::new(),
            kerning_groups: KerningGroups::default(),
            features: Features::default(),
            feature_draft: None,
            feature_settings: crate::FeatureGenerationSettings::default(),
            feature_revision: 0,
            layout_conversion: None,
            bitmap_strikes: Vec::new(),
            bitmap_tables_owned: false,
            math: None,
            math_table_owned: false,
            color: None,
            color_tables_owned: false,
            cid: None,
            guides: Vec::new(),
            extra: Extra::default(),
            user_data: serde_json::Value::Null,
        }
    }
}

fn regular_master() -> Master {
    Master {
        name: "Regular".to_owned(),
        ..Master::default()
    }
}

fn default_masters() -> Vec<Master> {
    vec![regular_master()]
}

fn deserialize_masters<'de, D>(deserializer: D) -> Result<Vec<Master>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let masters = Vec::<Master>::deserialize(deserializer)?;
    if masters.is_empty() {
        Ok(default_masters())
    } else {
        Ok(masters)
    }
}

static FALLBACK_MASTER: LazyLock<Master> = LazyLock::new(regular_master);

impl Font {
    /// Whether the saved draft differs from the active source or policy.
    pub fn has_unapplied_feature_draft(&self) -> bool {
        self.feature_draft.as_ref().is_some_and(|draft| {
            draft.source != self.features || draft.settings != self.feature_settings
        })
    }

    /// Creates a static empty font with one `Regular` master.
    pub fn new_empty(family: &str, units_per_em: u16) -> Font {
        let mut info = FontInfo {
            family_name: family.to_owned(),
            style_name: "Regular".to_owned(),
            units_per_em,
            ..FontInfo::default()
        };
        if info.version_major == 0 && info.version_minor == 0 {
            info.version_major = 1;
        }
        Font {
            info,
            ..Font::default()
        }
    }

    /// Returns the first master, which is guaranteed by document invariants.
    pub fn default_master(&self) -> &Master {
        self.masters.first().unwrap_or(&FALLBACK_MASTER)
    }

    /// Finds a master by persistent identifier.
    pub fn master(&self, id: &MasterId) -> Option<&Master> {
        self.masters.iter().find(|master| &master.id == id)
    }

    /// Finds a glyph by name.
    pub fn glyph(&self, name: &str) -> Option<&Glyph> {
        self.glyphs.get(name)
    }

    /// Finds a mutable glyph by name.
    pub fn glyph_mut(&mut self, name: &str) -> Option<&mut Glyph> {
        self.glyphs.get_mut(name)
    }

    /// Inserts a glyph, filling in missing main layers for every master.
    pub fn insert_glyph(&mut self, mut glyph: Glyph) -> Result<(), Error> {
        let name = glyph.name.clone();
        if self.glyphs.contains_key(name.as_ref()) {
            return Err(Error::DuplicateGlyph { glyph: name });
        }
        for master in &self.masters {
            if glyph.master_layer(&master.id).is_none() {
                glyph.layers.push(Layer::new_master(master.id.clone()));
            }
        }
        let has_components = glyph
            .layers
            .iter()
            .any(|layer| component_bases(layer).next().is_some());
        let previous = self.glyphs.insert(name.clone(), glyph);
        debug_assert!(previous.is_none());
        if has_components {
            match component_cycle_from(self, &name) {
                Ok(Some(cycle)) => {
                    self.glyphs.shift_remove(name.as_ref());
                    return Err(Error::ComponentCycle { glyph: cycle });
                }
                Err(error) => {
                    self.glyphs.shift_remove(name.as_ref());
                    return Err(error);
                }
                Ok(None) => {}
            }
        }
        Ok(())
    }

    /// Removes a glyph while retaining all other glyphs' relative order.
    pub fn remove_glyph(&mut self, name: &str) -> Option<Glyph> {
        self.glyphs.shift_remove(name)
    }

    /// Renames a glyph and updates every model reference to it.
    pub fn rename_glyph(&mut self, from: &str, to: &str) -> Result<(), Error> {
        if from == to {
            return self
                .glyphs
                .contains_key(from)
                .then_some(())
                .ok_or_else(|| Error::MissingGlyph { glyph: from.into() });
        }
        if self.glyphs.contains_key(to) {
            return Err(Error::DuplicateGlyph { glyph: to.into() });
        }
        let Some(index) = self.glyphs.get_index_of(from) else {
            return Err(Error::MissingGlyph { glyph: from.into() });
        };
        if self
            .math
            .as_ref()
            .is_some_and(|math| math.glyphs.contains_key(to))
        {
            return Err(Error::InvalidMath(format!(
                "rename would overwrite unresolved MATH data for `{to}`"
            )));
        }
        let Some((_, mut glyph)) = self.glyphs.shift_remove_entry(from) else {
            return Err(Error::MissingGlyph { glyph: from.into() });
        };
        let new_name = GlyphName::from(to);
        glyph.name = new_name.clone();
        let replaced = self.glyphs.shift_insert(index, new_name.clone(), glyph);
        debug_assert!(replaced.is_none());

        for glyph in self.glyphs.values_mut() {
            for layer in &mut glyph.layers {
                for component in &mut layer.components {
                    rename_name(&mut component.base, from, &new_name);
                }
                if let Some(color_layers) = &mut layer.color_layers {
                    for color_layer in color_layers {
                        for component in &mut color_layer.components {
                            rename_name(&mut component.base, from, &new_name);
                        }
                    }
                }
            }
        }
        for master in &mut self.masters {
            rename_kerning(&mut master.kerning_h, from, &new_name);
            rename_kerning(&mut master.kerning_v, from, &new_name);
        }
        rename_group_members(&mut self.kerning_groups.side1, from, &new_name);
        if let Some(math) = &mut self.math {
            math.rename_glyph(from, &new_name);
        }
        if let Some(color) = &mut self.color {
            color.rename_glyph(from, &new_name);
        }
        rename_group_members(&mut self.kerning_groups.side2, from, &new_name);
        for strike in &mut self.bitmap_strikes {
            if let Some(bitmap) = strike.glyphs.remove(from) {
                strike.glyphs.insert(new_name.clone(), bitmap);
            }
            for bitmap_glyph in strike.glyphs.values_mut() {
                for reference in &mut bitmap_glyph.refs {
                    rename_name(&mut reference.base, from, &new_name);
                }
            }
        }
        if let Some(cid) = &mut self.cid {
            for mapped_name in cid.cid_map.values_mut() {
                rename_name(mapped_name, from, &new_name);
            }
        }
        Ok(())
    }

    /// Adds a master and creates its empty main layer in every glyph.
    pub fn add_master(&mut self, master: Master) {
        if self.master(&master.id).is_some() {
            return;
        }
        for glyph in self.glyphs.values_mut() {
            if glyph.master_layer(&master.id).is_none() {
                glyph.layers.push(Layer::new_master(master.id.clone()));
            }
        }
        self.masters.push(master);
    }

    /// Iterates glyph names in persistent glyph order.
    pub fn glyph_order(&self) -> impl Iterator<Item = &GlyphName> {
        self.glyphs.keys()
    }
}

fn rename_name(name: &mut GlyphName, from: &str, to: &GlyphName) {
    if name.as_ref() == from {
        *name = to.clone();
    }
}

fn rename_kerning(kerning: &mut Kerning, from: &str, to: &GlyphName) {
    kerning.0 = std::mem::take(&mut kerning.0)
        .into_iter()
        .map(|((left, right), value)| {
            (
                (
                    rename_kern_side(left, from, to),
                    rename_kern_side(right, from, to),
                ),
                value,
            )
        })
        .collect();
}

fn rename_kern_side(side: KernSide, from: &str, to: &GlyphName) -> KernSide {
    match side {
        KernSide::Glyph(name) if name.as_ref() == from => KernSide::Glyph(to.clone()),
        other => other,
    }
}

fn rename_group_members(
    groups: &mut BTreeMap<String, std::collections::BTreeSet<GlyphName>>,
    from: &str,
    to: &GlyphName,
) {
    for members in groups.values_mut() {
        if members.remove(from) {
            members.insert(to.clone());
        }
    }
}

const MAX_COMPONENT_DEPTH: usize = 64;

fn component_cycle_from(font: &Font, name: &GlyphName) -> Result<Option<GlyphName>, Error> {
    let mut visiting = HashSet::new();
    let mut finished = HashSet::new();
    cycle_from(font, name, 0, &mut visiting, &mut finished)
}

fn cycle_from(
    font: &Font,
    name: &GlyphName,
    depth: usize,
    visiting: &mut HashSet<GlyphName>,
    finished: &mut HashSet<GlyphName>,
) -> Result<Option<GlyphName>, Error> {
    if depth > MAX_COMPONENT_DEPTH {
        return Err(Error::ComponentDepthLimit {
            glyph: name.clone(),
        });
    }
    if finished.contains(name) {
        return Ok(None);
    }
    if !visiting.insert(name.clone()) {
        return Ok(Some(name.clone()));
    }
    if let Some(glyph) = font.glyph(name) {
        for base in glyph.layers.iter().flat_map(component_bases) {
            if font.glyph(base).is_some() {
                if let Some(cycle) = cycle_from(font, base, depth + 1, visiting, finished)? {
                    return Ok(Some(cycle));
                }
            }
        }
    }
    visiting.remove(name);
    finished.insert(name.clone());
    Ok(None)
}

fn component_bases(layer: &Layer) -> impl Iterator<Item = &GlyphName> {
    layer
        .components
        .iter()
        .map(|component| &component.base)
        .chain(
            layer
                .color_layers
                .iter()
                .flatten()
                .flat_map(|color_layer| color_layer.components.iter())
                .map(|component| &component.base),
        )
}

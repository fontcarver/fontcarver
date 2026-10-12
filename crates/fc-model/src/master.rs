//! Design-space masters and their private data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{DesignLocation, Guide, Kerning, MasterId};

/// Metrics that may vary between masters.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct MasterMetrics {
    /// Typographic ascender.
    #[serde(default)]
    pub ascender: f64,
    /// Typographic descender.
    #[serde(default)]
    pub descender: f64,
    /// x-height.
    #[serde(default)]
    pub x_height: f64,
    /// Cap height.
    #[serde(default)]
    pub cap_height: f64,
    /// Italic angle in degrees.
    #[serde(default)]
    pub italic_angle: f64,
    /// `hhea` ascender override.
    #[serde(default)]
    pub hhea_ascender: Option<f64>,
    /// `hhea` descender override.
    #[serde(default)]
    pub hhea_descender: Option<f64>,
    /// `hhea` line gap override.
    #[serde(default)]
    pub hhea_line_gap: Option<f64>,
    /// OS/2 typo ascender override.
    #[serde(default)]
    pub typo_ascender: Option<f64>,
    /// OS/2 typo descender override.
    #[serde(default)]
    pub typo_descender: Option<f64>,
    /// OS/2 typo line gap override.
    #[serde(default)]
    pub typo_line_gap: Option<f64>,
    /// OS/2 Windows ascent override.
    #[serde(default)]
    pub win_ascent: Option<f64>,
    /// OS/2 Windows descent override.
    #[serde(default)]
    pub win_descent: Option<f64>,
    /// `vhea` ascender override.
    #[serde(default)]
    pub vhea_ascender: Option<f64>,
    /// `vhea` descender override.
    #[serde(default)]
    pub vhea_descender: Option<f64>,
    /// `vhea` line gap override.
    #[serde(default)]
    pub vhea_line_gap: Option<f64>,
    /// Default vertical origin.
    #[serde(default)]
    pub default_vertical_origin: Option<f64>,
    /// Default vertical advance.
    #[serde(default)]
    pub default_vertical_advance: Option<f64>,
}

impl MasterMetrics {
    /// Validate finite source metrics and OpenType-sized overrides before editing.
    pub fn validate(&self) -> Result<(), crate::Error> {
        let signed = [
            Some(self.ascender),
            Some(self.descender),
            Some(self.x_height),
            Some(self.cap_height),
            self.hhea_ascender,
            self.hhea_descender,
            self.hhea_line_gap,
            self.typo_ascender,
            self.typo_descender,
            self.typo_line_gap,
            self.vhea_ascender,
            self.vhea_descender,
            self.vhea_line_gap,
            self.default_vertical_origin,
        ];
        let unsigned = [
            self.win_ascent,
            self.win_descent,
            self.default_vertical_advance,
        ];
        if signed
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite() || !(-32768.0..=32767.0).contains(&v))
            || unsigned
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=65535.0).contains(&v))
            || !self.italic_angle.is_finite()
            || self.italic_angle.abs() >= 90.0
        {
            return Err(crate::Error::InvalidDesignSpace(
                "master metrics exceed finite OpenType ranges".into(),
            ));
        }
        Ok(())
    }
}

/// A PostScript alignment zone.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Zone {
    /// Zone position.
    #[serde(default)]
    pub position: f64,
    /// Zone size; negative values denote bottom zones.
    #[serde(default)]
    pub size: f64,
}

/// A named global stem width.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Stem {
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Whether this is a horizontal stem.
    #[serde(default)]
    pub horizontal: bool,
    /// Stem width.
    #[serde(default)]
    pub width: f64,
}

/// PostScript private-dictionary values.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PsPrivate {
    /// BlueValues array.
    #[serde(default)]
    pub blue_values: Vec<f64>,
    /// OtherBlues array.
    #[serde(default)]
    pub other_blues: Vec<f64>,
    /// FamilyBlues array.
    #[serde(default)]
    pub family_blues: Vec<f64>,
    /// FamilyOtherBlues array.
    #[serde(default)]
    pub family_other_blues: Vec<f64>,
    /// BlueScale value.
    #[serde(default)]
    pub blue_scale: Option<f64>,
    /// BlueShift value.
    #[serde(default)]
    pub blue_shift: Option<f64>,
    /// BlueFuzz value.
    #[serde(default)]
    pub blue_fuzz: Option<f64>,
    /// StdHW value.
    #[serde(default)]
    pub std_hw: Option<f64>,
    /// StdVW value.
    #[serde(default)]
    pub std_vw: Option<f64>,
    /// StemSnapH values.
    #[serde(default)]
    pub stem_snap_h: Vec<f64>,
    /// StemSnapV values.
    #[serde(default)]
    pub stem_snap_v: Vec<f64>,
    /// ForceBold value.
    #[serde(default)]
    pub force_bold: Option<bool>,
    /// LanguageGroup value.
    #[serde(default)]
    pub language_group: Option<u8>,
    /// Other private-dictionary values.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, unknown>"))]
    pub other: BTreeMap<String, serde_json::Value>,
}

/// Master-level TrueType programs and maxima.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct TtMasterData {
    /// UPEM at explicit authoring time; absence identifies imported or legacy data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub authored_units_per_em: Option<u16>,
    /// Control value table entries.
    #[serde(default)]
    pub cvt: Vec<i16>,
    /// Font program bytes.
    #[serde(default)]
    pub fpgm: Vec<u8>,
    /// Control value program bytes.
    #[serde(default)]
    pub prep: Vec<u8>,
    /// Maximum stack elements.
    #[serde(default)]
    pub max_stack_elements: Option<u16>,
    /// Maximum storage locations.
    #[serde(default)]
    pub max_storage: Option<u16>,
    /// Maximum function definitions.
    #[serde(default)]
    pub max_function_defs: Option<u16>,
    /// Maximum instruction definitions.
    #[serde(default)]
    pub max_instruction_defs: Option<u16>,
    /// Maximum twilight points.
    #[serde(default)]
    pub max_twilight_points: Option<u16>,
    /// Maximum zones.
    #[serde(default)]
    pub max_zones: Option<u16>,
    /// Maximum per-glyph instruction byte length.
    #[serde(default)]
    pub max_size_of_instructions: Option<u16>,
}

/// One interpolation master.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct Master {
    /// Persistent master identifier; deserialization mints a fresh ID when omitted.
    #[serde(default)]
    pub id: MasterId,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Design-space location.
    #[serde(default, with = "crate::font::design_location_serde")]
    #[cfg_attr(feature = "ts", ts(type = "Record<string, number>"))]
    pub location: DesignLocation,
    /// Master metrics.
    #[serde(default)]
    pub metrics: MasterMetrics,
    /// PostScript alignment zones.
    #[serde(default)]
    pub alignment_zones: Vec<Zone>,
    /// Named global stems.
    #[serde(default)]
    pub stems: Vec<Stem>,
    /// Master-local guides.
    #[serde(default)]
    pub guides: Vec<Guide>,
    /// Horizontal kerning.
    #[serde(default)]
    pub kerning_h: Kerning,
    /// Vertical kerning.
    #[serde(default)]
    pub kerning_v: Kerning,
    /// PostScript private dictionary.
    #[serde(default)]
    pub private_dict: PsPrivate,
    /// Additional CFF1 CID private dictionaries, indexed from FD 1; FD 0 uses `private_dict`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<PsPrivate>>", optional))]
    pub cff_private_dicts: Vec<PsPrivate>,
    /// TrueType programs and maxima.
    #[serde(default)]
    pub tt: TtMasterData,
    /// Format-specific master metadata.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub user_data: serde_json::Value,
}

impl Default for Master {
    fn default() -> Self {
        Self {
            id: MasterId::new(),
            name: String::new(),
            location: DesignLocation::new(),
            metrics: MasterMetrics::default(),
            alignment_zones: Vec::new(),
            stems: Vec::new(),
            guides: Vec::new(),
            kerning_h: Kerning::default(),
            kerning_v: Kerning::default(),
            private_dict: PsPrivate::default(),
            cff_private_dicts: Vec::new(),
            tt: TtMasterData::default(),
            user_data: serde_json::Value::Null,
        }
    }
}

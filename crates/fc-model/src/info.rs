//! Font-wide naming and metrics metadata.

use serde::{Deserialize, Serialize};

/// Platform family used by an imported name record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum NamePlatform {
    /// Microsoft Windows name encoding.
    Windows,
    /// Classic Macintosh name encoding.
    Mac,
    /// Platform-independent Unicode encoding.
    #[default]
    Unicode,
    /// Another OpenType platform identifier retained for round-trip fidelity.
    Other(u16),
}

/// A platform-normalized OpenType name table record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct NameRecord {
    /// OpenType name identifier.
    #[serde(default)]
    pub name_id: u16,
    /// BCP 47 language tag.
    #[serde(default)]
    pub language: String,
    /// Decoded record text.
    #[serde(default)]
    pub text: String,
    /// Source platform family.
    #[serde(default)]
    pub platform: NamePlatform,
}

/// A `gasp` behavior range.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct GaspRecord {
    /// Maximum pixels per em covered by the range.
    #[serde(default)]
    pub range_max_ppem: u16,
    /// OpenType `gasp` behavior bits represented by their bit indexes.
    #[serde(default)]
    pub behavior: Vec<u8>,
}

/// Font-wide information following UFO 3 `fontinfo.plist` field names.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FontInfo {
    /// Default family name.
    #[serde(default)]
    pub family_name: String,
    /// Default style name.
    #[serde(default)]
    pub style_name: String,
    /// Units per em.
    #[serde(default)]
    pub units_per_em: u16,
    /// Major version number.
    #[serde(default)]
    pub version_major: u32,
    /// Minor version number.
    #[serde(default)]
    pub version_minor: u32,
    /// Copyright notice.
    #[serde(default)]
    pub copyright: Option<String>,
    /// Trademark notice.
    #[serde(default)]
    pub trademark: Option<String>,
    /// Free-form font note.
    #[serde(default)]
    pub note: Option<String>,
    /// Designer name.
    #[serde(default)]
    pub open_type_name_designer: Option<String>,
    /// Designer URL.
    #[serde(default)]
    pub open_type_name_designer_url: Option<String>,
    /// Manufacturer name.
    #[serde(default)]
    pub open_type_name_manufacturer: Option<String>,
    /// Manufacturer URL.
    #[serde(default)]
    pub open_type_name_manufacturer_url: Option<String>,
    /// License text.
    #[serde(default)]
    pub open_type_name_license: Option<String>,
    /// License URL.
    #[serde(default)]
    pub open_type_name_license_url: Option<String>,
    /// Version name string.
    #[serde(default)]
    pub open_type_name_version: Option<String>,
    /// Unique identifier string.
    #[serde(default)]
    pub open_type_name_unique_id: Option<String>,
    /// Font description.
    #[serde(default)]
    pub open_type_name_description: Option<String>,
    /// Preferred family name.
    #[serde(default)]
    pub open_type_name_preferred_family_name: Option<String>,
    /// Preferred subfamily name.
    #[serde(default)]
    pub open_type_name_preferred_subfamily_name: Option<String>,
    /// WWS-compatible full name.
    #[serde(default)]
    pub open_type_name_compatible_full_name: Option<String>,
    /// Suggested sample text.
    #[serde(default)]
    pub open_type_name_sample_text: Option<String>,
    /// WWS family name.
    #[serde(default)]
    pub open_type_name_wws_family_name: Option<String>,
    /// WWS subfamily name.
    #[serde(default)]
    pub open_type_name_wws_subfamily_name: Option<String>,
    /// PostScript font name.
    #[serde(default)]
    pub postscript_font_name: Option<String>,
    /// PostScript full name.
    #[serde(default)]
    pub postscript_full_name: Option<String>,
    /// All imported name records, including non-default languages.
    #[serde(default)]
    pub names: Vec<NameRecord>,
    /// Four-character OS/2 vendor identifier.
    #[serde(default)]
    pub open_type_os2_vendor_id: Option<String>,
    /// OS/2 weight class.
    #[serde(default)]
    pub open_type_os2_weight_class: Option<u16>,
    /// OS/2 width class.
    #[serde(default)]
    pub open_type_os2_width_class: Option<u16>,
    /// Selected OS/2 `fsSelection` bit indexes.
    #[serde(default)]
    pub open_type_os2_selection: Vec<u8>,
    /// Selected OS/2 embedding-type bit indexes.
    #[serde(default)]
    pub open_type_os2_type: Vec<u8>,
    /// OS/2 PANOSE classification bytes.
    #[serde(default)]
    pub open_type_os2_panose: Option<[u8; 10]>,
    /// OS/2 family class and subclass.
    #[serde(default)]
    pub open_type_os2_family_class: Option<(u8, u8)>,
    /// Selected OS/2 Unicode range bit indexes.
    #[serde(default)]
    pub open_type_os2_unicode_ranges: Vec<u8>,
    /// Selected OS/2 code page range bit indexes.
    #[serde(default)]
    pub open_type_os2_code_page_ranges: Vec<u8>,
    /// Subscript horizontal size.
    #[serde(default)]
    pub open_type_os2_subscript_x_size: Option<f64>,
    /// Subscript vertical size.
    #[serde(default)]
    pub open_type_os2_subscript_y_size: Option<f64>,
    /// Subscript horizontal offset.
    #[serde(default)]
    pub open_type_os2_subscript_x_offset: Option<f64>,
    /// Subscript vertical offset.
    #[serde(default)]
    pub open_type_os2_subscript_y_offset: Option<f64>,
    /// Superscript horizontal size.
    #[serde(default)]
    pub open_type_os2_superscript_x_size: Option<f64>,
    /// Superscript vertical size.
    #[serde(default)]
    pub open_type_os2_superscript_y_size: Option<f64>,
    /// Superscript horizontal offset.
    #[serde(default)]
    pub open_type_os2_superscript_x_offset: Option<f64>,
    /// Superscript vertical offset.
    #[serde(default)]
    pub open_type_os2_superscript_y_offset: Option<f64>,
    /// Strikeout thickness.
    #[serde(default)]
    pub open_type_os2_strikeout_size: Option<f64>,
    /// Strikeout vertical position.
    #[serde(default)]
    pub open_type_os2_strikeout_position: Option<f64>,
    /// OpenType `head.created` timestamp.
    #[serde(default)]
    pub open_type_head_created: Option<String>,
    /// Selected `head.flags` bit indexes.
    #[serde(default)]
    pub open_type_head_flags: Vec<u8>,
    /// Lowest recommended pixels per em.
    #[serde(default)]
    pub open_type_head_lowest_rec_ppem: Option<u16>,
    /// Grid-fitting and scan-conversion ranges.
    #[serde(default)]
    pub open_type_gasp_range_records: Vec<GaspRecord>,
    /// Horizontal caret slope rise.
    #[serde(default)]
    pub open_type_hhea_caret_slope_rise: Option<i32>,
    /// Horizontal caret slope run.
    #[serde(default)]
    pub open_type_hhea_caret_slope_run: Option<i32>,
    /// Horizontal caret offset.
    #[serde(default)]
    pub open_type_hhea_caret_offset: Option<i32>,
    /// Vertical caret slope rise.
    #[serde(default)]
    pub open_type_vhea_caret_slope_rise: Option<i32>,
    /// Vertical caret slope run.
    #[serde(default)]
    pub open_type_vhea_caret_slope_run: Option<i32>,
    /// Vertical caret offset.
    #[serde(default)]
    pub open_type_vhea_caret_offset: Option<i32>,
    /// Whether all glyphs are fixed-pitch.
    #[serde(default)]
    pub postscript_is_fixed_pitch: Option<bool>,
    /// CFF default width.
    #[serde(default)]
    pub postscript_default_width_x: Option<f64>,
    /// CFF nominal width.
    #[serde(default)]
    pub postscript_nominal_width_x: Option<f64>,
    /// PostScript underline position.
    #[serde(default)]
    pub postscript_underline_position: Option<f64>,
    /// PostScript underline thickness.
    #[serde(default)]
    pub postscript_underline_thickness: Option<f64>,
    /// PostScript unique integer identifier.
    #[serde(default)]
    pub postscript_unique_id: Option<i32>,
    /// Macintosh FOND resource name.
    #[serde(default)]
    pub macintosh_fond_name: Option<String>,
    /// Macintosh FOND family identifier.
    #[serde(default)]
    pub macintosh_fond_family_id: Option<i32>,
    /// Format-specific metadata not covered by named fields.
    #[serde(default)]
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub user_data: serde_json::Value,
}

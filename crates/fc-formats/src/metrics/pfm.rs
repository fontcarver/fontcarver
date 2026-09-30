//! Bounded Windows v1 PostScript printer font metrics (PFM).
//!
//! Implements the Microsoft Windows DDK Printer and Font Kit, chapters 2 and 4.
//! Only 8-bit PostScript extents at 1000 units per em and pair kerning are
//! supported. PCL widths, origin/track tables, bitmap data and nonzero unknown
//! payloads are rejected. Strings remain bytes: a charset byte is not Unicode.
//! All header and extended-metric scalar values in this subset survive a
//! semantic roundtrip. File organization and zero padding are canonicalized.

use std::{collections::BTreeSet, ops::Range};

use serde::{Deserialize, Serialize};

use crate::Error;

mod font_adapter;
pub use font_adapter::{read_pfm_path, write_pfm_font};

pub(super) const MAX_FILE_BYTES: usize = 1024 * 1024;
const HEADER_BYTES: usize = 147;
const EXTENDED_BYTES: usize = 52;
const MAX_NAME_BYTES: usize = 255;

/// Scalar fields from the Windows font header, retained without reinterpretation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PfmHeader {
    /// Nominal point size (`dfPoints`).
    pub points: u16,
    /// Device vertical resolution in dots per inch.
    pub vertical_resolution: u16,
    /// Device horizontal resolution in dots per inch.
    pub horizontal_resolution: u16,
    /// Baseline ascent from the top of the character cell.
    pub ascent: u16,
    /// Internal leading.
    pub internal_leading: u16,
    /// External leading.
    pub external_leading: u16,
    /// Italic style flag.
    pub italic: bool,
    /// Underline style flag.
    pub underline: bool,
    /// Strikeout style flag.
    pub strikeout: bool,
    /// Windows weight value.
    pub weight: u16,
    /// Original Windows character-set identifier; no Unicode conversion implied.
    pub charset: u8,
    /// Fixed pixel width, or zero for variable width.
    pub pixel_width: u16,
    /// Nominal character-cell height.
    pub pixel_height: u16,
    /// Windows pitch and family bit field.
    pub pitch_and_family: u8,
    /// Declared average width in 1000-unit font space.
    pub average_width: u16,
    /// Declared maximum width in 1000-unit font space.
    pub maximum_width: u16,
    /// Default character offset relative to [`PfmMetrics::first_char`].
    pub default_char: u8,
    /// Word-break character offset relative to [`PfmMetrics::first_char`].
    pub break_char: u8,
}

impl Default for PfmHeader {
    fn default() -> Self {
        Self {
            points: 10,
            vertical_resolution: 300,
            horizontal_resolution: 300,
            ascent: 800,
            internal_leading: 0,
            external_leading: 0,
            italic: false,
            underline: false,
            strikeout: false,
            weight: 400,
            charset: 0,
            pixel_width: 0,
            pixel_height: 1000,
            pitch_and_family: 1,
            average_width: 0,
            maximum_width: 0,
            default_char: 0,
            break_char: 0,
        }
    }
}

/// Fixed-size Windows extended text metrics; measurements retain their PFM units.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PfmExtendedMetrics {
    /// Nominal point size in twentieths of a point.
    pub point_size: i16,
    /// Orientation code (zero is either orientation).
    pub orientation: i16,
    /// Master height in device units.
    pub master_height: i16,
    /// Minimum scale in device units.
    pub min_scale: i16,
    /// Maximum scale in device units.
    pub max_scale: i16,
    /// Font units per em; the PostScript subset requires 1000.
    pub master_units: i16,
    /// Capital height.
    pub cap_height: i16,
    /// Lowercase x-height.
    pub x_height: i16,
    /// Lowercase ascender height.
    pub lowercase_ascent: i16,
    /// Lowercase descender distance below the baseline.
    pub lowercase_descent: i16,
    /// Clockwise slant in tenths of a degree.
    pub slant: i16,
    /// Superscript baseline offset.
    pub superscript: i16,
    /// Subscript baseline offset.
    pub subscript: i16,
    /// Superscript character size.
    pub superscript_size: i16,
    /// Subscript character size.
    pub subscript_size: i16,
    /// Single-underline offset below the baseline.
    pub underline_offset: i16,
    /// Single-underline thickness.
    pub underline_width: i16,
    /// Upper double-underline offset.
    pub double_upper_underline_offset: i16,
    /// Lower double-underline offset.
    pub double_lower_underline_offset: i16,
    /// Upper double-underline thickness.
    pub double_upper_underline_width: i16,
    /// Lower double-underline thickness.
    pub double_lower_underline_width: i16,
    /// Strikeout offset above the baseline.
    pub strikeout_offset: i16,
    /// Strikeout thickness.
    pub strikeout_width: i16,
}

impl Default for PfmExtendedMetrics {
    fn default() -> Self {
        Self::from_values([
            200, 0, 300, 1, 1000, 1000, 700, 450, 800, 200, 0, 0, 0, 0, 0, 100, 50, 0, 0, 0, 0,
            300, 50,
        ])
    }
}

impl PfmExtendedMetrics {
    fn from_values(v: [i16; 23]) -> Self {
        Self {
            point_size: v[0],
            orientation: v[1],
            master_height: v[2],
            min_scale: v[3],
            max_scale: v[4],
            master_units: v[5],
            cap_height: v[6],
            x_height: v[7],
            lowercase_ascent: v[8],
            lowercase_descent: v[9],
            slant: v[10],
            superscript: v[11],
            subscript: v[12],
            superscript_size: v[13],
            subscript_size: v[14],
            underline_offset: v[15],
            underline_width: v[16],
            double_upper_underline_offset: v[17],
            double_lower_underline_offset: v[18],
            double_upper_underline_width: v[19],
            double_lower_underline_width: v[20],
            strikeout_offset: v[21],
            strikeout_width: v[22],
        }
    }

    fn values(&self) -> [i16; 23] {
        [
            self.point_size,
            self.orientation,
            self.master_height,
            self.min_scale,
            self.max_scale,
            self.master_units,
            self.cap_height,
            self.x_height,
            self.lowercase_ascent,
            self.lowercase_descent,
            self.slant,
            self.superscript,
            self.subscript,
            self.superscript_size,
            self.subscript_size,
            self.underline_offset,
            self.underline_width,
            self.double_upper_underline_offset,
            self.double_lower_underline_offset,
            self.double_upper_underline_width,
            self.double_lower_underline_width,
            self.strikeout_offset,
            self.strikeout_width,
        ]
    }
}

/// Signed horizontal kerning between two encoded byte values.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PfmKernPair {
    /// Left character byte.
    pub left: u8,
    /// Right character byte.
    pub right: u8,
    /// Signed adjustment in 1000-unit font space.
    pub value: i16,
}

/// Metrics-only Windows PostScript font with a contiguous 8-bit character range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PfmMetrics {
    /// Exactly 60 copyright bytes, including any original zero padding.
    pub copyright: Vec<u8>,
    /// Driver-name bytes, case-insensitively equal to `PostScript`.
    pub device_name: Vec<u8>,
    /// Nonempty Windows face-name bytes without their terminating NUL.
    pub face_name: Vec<u8>,
    /// Nonempty PostScript name bytes without their terminating NUL.
    pub postscript_name: Vec<u8>,
    /// Font-header scalar data.
    pub header: PfmHeader,
    /// Extended metrics, excluding automatically derived table sizes/counts.
    pub extended: PfmExtendedMetrics,
    /// Byte value of the first encoded character.
    pub first_char: u8,
    /// One width for every byte from `first_char`; 1 to 256 entries.
    pub widths: Vec<u16>,
    /// Unique pairs; the writer canonicalizes ordering by `(right, left)`.
    pub kerning: Vec<PfmKernPair>,
}

impl Default for PfmMetrics {
    fn default() -> Self {
        Self {
            copyright: vec![0; 60],
            device_name: b"PostScript".to_vec(),
            face_name: b"Untitled".to_vec(),
            postscript_name: b"Untitled".to_vec(),
            header: PfmHeader::default(),
            extended: PfmExtendedMetrics::default(),
            first_char: 0,
            widths: vec![0],
            kerning: Vec::new(),
        }
    }
}

fn invalid(reason: impl Into<String>) -> Error {
    Error::Source {
        format: "PFM",
        reason: reason.into(),
    }
}

fn word(data: &[u8], offset: usize) -> Result<u16, Error> {
    let value = data
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("truncated word"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn long(data: &[u8], offset: usize) -> Result<usize, Error> {
    let value = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("truncated offset"))?;
    usize::try_from(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
        .map_err(|_| invalid("offset exceeds addressable range"))
}

struct Reader<'a> {
    data: &'a [u8],
    ranges: Vec<Range<usize>>,
}

impl Reader<'_> {
    fn claim(&mut self, start: usize, length: usize) -> Result<(), Error> {
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid("offset overflow"))?;
        if start < HEADER_BYTES || end > self.data.len() {
            return Err(invalid("table offset or length is outside file payload"));
        }
        if self
            .ranges
            .iter()
            .any(|range| start < range.end && range.start < end)
        {
            return Err(invalid("overlapping PFM data structures"));
        }
        self.ranges.push(start..end);
        Ok(())
    }

    fn name(&mut self, pointer: usize) -> Result<Vec<u8>, Error> {
        let start = long(self.data, pointer)?;
        let tail = self
            .data
            .get(start..)
            .ok_or_else(|| invalid("name offset outside file"))?;
        let length = tail
            .iter()
            .take(MAX_NAME_BYTES + 1)
            .position(|b| *b == 0)
            .ok_or_else(|| invalid("unterminated or oversized PFM name"))?;
        if length == 0 {
            return Err(invalid("empty PFM name"));
        }
        let result = tail[..length].to_vec();
        self.claim(start, length + 1)?;
        Ok(result)
    }

    fn finish(mut self) -> Result<(), Error> {
        self.ranges.sort_by_key(|range| range.start);
        let mut end = HEADER_BYTES;
        for range in self.ranges {
            if self.data[end..range.start].iter().any(|byte| *byte != 0) {
                return Err(invalid("unsupported nonzero unclaimed payload"));
            }
            end = range.end;
        }
        if self.data[end..].iter().any(|byte| *byte != 0) {
            return Err(invalid("unsupported trailing payload"));
        }
        Ok(())
    }
}

/// Parses a bounded Windows v1 PostScript PFM without guessing Unicode mappings.
///
/// Rejects unsupported fields, aliasing tables, bad counts and unknown nonzero
/// data rather than silently discarding them. Input size is limited to 1 MiB.
pub fn read_pfm(data: &[u8]) -> Result<PfmMetrics, Error> {
    if !(HEADER_BYTES..=MAX_FILE_BYTES).contains(&data.len()) {
        return Err(invalid("file size is outside the supported PFM range"));
    }
    if word(data, 0)? != 0x100 || long(data, 2)? != data.len() {
        return Err(invalid(
            "unsupported version or mismatched declared file size",
        ));
    }
    if word(data, 66)? != 0x81 || word(data, 117)? != 30 {
        return Err(invalid(
            "only v1 PostScript printer metrics with 30-byte extension are supported",
        ));
    }
    if word(data, 99)? != 0
        || [109, 113, 127, 135, 143]
            .into_iter()
            .any(|offset| long(data, offset).is_ok_and(|value| value != 0))
    {
        return Err(invalid(
            "bitmap, origin, track or reserved fields are unsupported",
        ));
    }
    if data[80..83].iter().any(|value| *value > 1) {
        return Err(invalid("style flags must be zero or one"));
    }
    if data[96] < data[95] {
        return Err(invalid("reversed character range"));
    }
    let count = usize::from(data[96]) - usize::from(data[95]) + 1;
    let mut reader = Reader {
        data,
        ranges: Vec::new(),
    };
    let ext = long(data, 119)?;
    reader.claim(ext, EXTENDED_BYTES)?;
    if word(data, ext)? != EXTENDED_BYTES as u16 || word(data, ext + 50)? != 0 {
        return Err(invalid("unsupported extended metrics size or track count"));
    }
    let mut values = [0; 23];
    for (index, value) in values.iter_mut().enumerate() {
        *value = word(data, ext + 2 + index * 2)? as i16;
    }
    let extent = long(data, 123)?;
    reader.claim(extent, count * 2)?;
    let widths = (0..count)
        .map(|index| word(data, extent + index * 2))
        .collect::<Result<_, _>>()?;
    let pairs = usize::from(word(data, ext + 48)?);
    let pair_offset = long(data, 131)?;
    let mut kerning = Vec::new();
    if pair_offset != 0 {
        reader.claim(pair_offset, 2 + pairs * 4)?;
        if usize::from(word(data, pair_offset)?) != pairs {
            return Err(invalid("pair table count does not match extended metrics"));
        }
        for index in 0..pairs {
            let at = pair_offset + 2 + index * 4;
            kerning.push(PfmKernPair {
                left: data[at],
                right: data[at + 1],
                value: word(data, at + 2)? as i16,
            });
        }
    } else if pairs != 0 {
        return Err(invalid("kerning count has no pair table"));
    }
    let result = PfmMetrics {
        copyright: data[6..66].to_vec(),
        device_name: reader.name(101)?,
        face_name: reader.name(105)?,
        postscript_name: reader.name(139)?,
        header: PfmHeader {
            points: word(data, 68)?,
            vertical_resolution: word(data, 70)?,
            horizontal_resolution: word(data, 72)?,
            ascent: word(data, 74)?,
            internal_leading: word(data, 76)?,
            external_leading: word(data, 78)?,
            italic: data[80] != 0,
            underline: data[81] != 0,
            strikeout: data[82] != 0,
            weight: word(data, 83)?,
            charset: data[85],
            pixel_width: word(data, 86)?,
            pixel_height: word(data, 88)?,
            pitch_and_family: data[90],
            average_width: word(data, 91)?,
            maximum_width: word(data, 93)?,
            default_char: data[97],
            break_char: data[98],
        },
        extended: PfmExtendedMetrics::from_values(values),
        first_char: data[95],
        widths,
        kerning,
    };
    validate(&result)?;
    reader.finish()?;
    Ok(result)
}

fn validate(metrics: &PfmMetrics) -> Result<(), Error> {
    if metrics.widths.is_empty() || metrics.widths.len() > 256 - usize::from(metrics.first_char) {
        return Err(invalid(
            "width count does not fit the 8-bit character range",
        ));
    }
    if usize::from(metrics.header.default_char) >= metrics.widths.len()
        || usize::from(metrics.header.break_char) >= metrics.widths.len()
    {
        return Err(invalid(
            "default or break character lies outside the extent table",
        ));
    }
    if metrics.copyright.len() != 60 {
        return Err(invalid("copyright must preserve exactly 60 bytes"));
    }
    for name in [
        &metrics.device_name,
        &metrics.face_name,
        &metrics.postscript_name,
    ] {
        if name.is_empty() || name.len() > MAX_NAME_BYTES || name.contains(&0) {
            return Err(invalid("PFM names must contain 1 to 255 non-NUL bytes"));
        }
    }
    if !metrics.device_name.eq_ignore_ascii_case(b"PostScript") {
        return Err(invalid("only the PostScript driver is supported"));
    }
    if metrics.extended.master_units != 1000 {
        return Err(invalid(
            "PostScript extent metrics require 1000 units per em",
        ));
    }
    if metrics.kerning.len() > usize::from(u16::MAX) {
        return Err(invalid("too many kerning pairs"));
    }
    let mut seen = BTreeSet::new();
    let range =
        usize::from(metrics.first_char)..usize::from(metrics.first_char) + metrics.widths.len();
    for pair in &metrics.kerning {
        if !range.contains(&usize::from(pair.left)) || !range.contains(&usize::from(pair.right)) {
            return Err(invalid("kerning character lies outside the extent table"));
        }
        if !seen.insert((pair.left, pair.right)) {
            return Err(invalid("duplicate kerning pair"));
        }
    }
    Ok(())
}

fn put_word(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_long(bytes: &mut [u8], at: usize, value: usize) {
    // All callers write the bounded output buffer's length (less than 1 MiB).
    bytes[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
}

/// Serializes validated metrics as canonical Windows v1 PostScript PFM bytes.
///
/// Validates all data before serialization; pairs are sorted by their packed
/// Windows `(right, left)` key. There is no outline or Unicode data in PFM.
pub fn write_pfm(metrics: &PfmMetrics) -> Result<Vec<u8>, Error> {
    validate(metrics)?;
    let mut bytes = vec![0; HEADER_BYTES];
    put_word(&mut bytes, 0, 0x100);
    bytes[6..66].copy_from_slice(&metrics.copyright);
    put_word(&mut bytes, 66, 0x81);
    let h = &metrics.header;
    for (offset, value) in [
        (68, h.points),
        (70, h.vertical_resolution),
        (72, h.horizontal_resolution),
        (74, h.ascent),
        (76, h.internal_leading),
        (78, h.external_leading),
        (83, h.weight),
        (86, h.pixel_width),
        (88, h.pixel_height),
        (91, h.average_width),
        (93, h.maximum_width),
    ] {
        put_word(&mut bytes, offset, value);
    }
    for (offset, value) in [
        (80, u8::from(h.italic)),
        (81, u8::from(h.underline)),
        (82, u8::from(h.strikeout)),
        (85, h.charset),
        (90, h.pitch_and_family),
        (95, metrics.first_char),
        (
            96,
            (usize::from(metrics.first_char) + metrics.widths.len() - 1) as u8,
        ),
        (97, h.default_char),
        (98, h.break_char),
    ] {
        bytes[offset] = value;
    }
    put_word(&mut bytes, 117, 30);
    put_long(&mut bytes, 119, HEADER_BYTES);
    bytes.extend_from_slice(&(EXTENDED_BYTES as u16).to_le_bytes());
    for value in metrics.extended.values() {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&(metrics.kerning.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    let offset = bytes.len();
    put_long(&mut bytes, 123, offset);
    for value in &metrics.widths {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    if !metrics.kerning.is_empty() {
        let offset = bytes.len();
        put_long(&mut bytes, 131, offset);
        bytes.extend_from_slice(&(metrics.kerning.len() as u16).to_le_bytes());
        let mut pairs: Vec<_> = metrics.kerning.iter().collect();
        pairs.sort_by_key(|pair| (pair.right, pair.left));
        for pair in pairs {
            bytes.extend_from_slice(&[pair.left, pair.right]);
            bytes.extend_from_slice(&pair.value.to_le_bytes());
        }
    }
    for (pointer, name) in [
        (101, &metrics.device_name),
        (105, &metrics.face_name),
        (139, &metrics.postscript_name),
    ] {
        let offset = bytes.len();
        put_long(&mut bytes, pointer, offset);
        bytes.extend_from_slice(name);
        bytes.push(0);
    }
    let size = bytes.len();
    put_long(&mut bytes, 2, size);
    Ok(bytes)
}

use std::{collections::BTreeMap, ops::Range};

use fw_model::{CidInfo, Contour, FontInfo, GlyphName, Layer, Point, PointKind, PsPrivate};
use read_fonts::{
    ps::cff::{dict, index::Index, CffFontRef},
    types::GlyphId,
    FontRef, TableProvider,
};
use skrifa::{
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlineGlyphCollection, OutlineGlyphFormat, OutlinePen},
};

use crate::Error;

pub(crate) struct CffSource<'a> {
    outlines: OutlineGlyphCollection<'a>,
}

impl<'a> CffSource<'a> {
    pub(crate) fn new(face: &FontRef<'a>) -> Option<Self> {
        OutlineGlyphCollection::with_format(face, OutlineGlyphFormat::Cff2)
            .or_else(|| OutlineGlyphCollection::with_format(face, OutlineGlyphFormat::Cff))
            .map(|outlines| Self { outlines })
    }

    pub(crate) fn apply(&self, gid: u16, layer: &mut Layer) -> Result<(), Error> {
        let Some(glyph) = self.outlines.get(GlyphId::new(u32::from(gid))) else {
            return Ok(());
        };
        let mut pen = CffPen::default();
        glyph
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .map_err(|error| Error::parse("CFF/CFF2", error))?;
        pen.finish_open();
        if pen.saw_quadratic {
            return Err(Error::Parse {
                table: "CFF/CFF2",
                reason: format!("glyph {gid} emitted an unexpected quadratic segment"),
            });
        }
        layer.contours = pen.contours;
        Ok(())
    }
}

pub(crate) fn cid_values(face: &FontRef<'_>) -> Result<Option<Vec<u16>>, Error> {
    let Some(cff) = cff_font(face)? else {
        return Ok(None);
    };
    if !cff.is_cid() {
        return Ok(None);
    }
    let charset = cff.charset().ok_or_else(|| Error::Parse {
        table: "CFF ",
        reason: "CID font has no charset".to_owned(),
    })?;
    let mut cids = vec![0; cff.num_glyphs() as usize];
    for (gid, cid) in charset.iter() {
        let slot = cids
            .get_mut(gid.to_u32() as usize)
            .ok_or_else(|| Error::Parse {
                table: "CFF ",
                reason: "CID charset glyph is out of range".to_owned(),
            })?;
        *slot = cid.to_u16();
    }
    Ok(Some(cids))
}

pub(crate) fn cid_info(
    face: &FontRef<'_>,
    glyph_count: u16,
    glyph_names: &[GlyphName],
) -> Result<Option<CidInfo>, Error> {
    let Some(cids) = cid_values(face)? else {
        return Ok(None);
    };
    if cids.len() != usize::from(glyph_count) {
        return Err(Error::Parse {
            table: "CFF ",
            reason: "CID charset length differs from maxp glyph count".to_owned(),
        });
    }
    let (registry, ordering, supplement) = ros(face)?.ok_or_else(|| Error::Parse {
        table: "CFF ",
        reason: "CID font has no ROS operator".to_owned(),
    })?;
    let cid_map = cids
        .into_iter()
        .zip(glyph_names.iter().cloned())
        .map(|(cid, name)| (u32::from(cid), name))
        .collect::<BTreeMap<_, _>>();
    Ok(Some(CidInfo {
        registry,
        ordering,
        supplement,
        cid_map,
    }))
}

pub(crate) fn apply_private(
    face: &FontRef<'_>,
    info: &mut FontInfo,
    target: &mut PsPrivate,
) -> Result<(), Error> {
    let Some(cff_data) = cff_data_and_top_dict(face)? else {
        return Ok(());
    };
    let top_entries = dict_entries(cff_data.top_dict, "CFF/CFF2 Top DICT")?;
    for entry in &top_entries {
        if let dict::Entry::UniqueId(value) = entry {
            info.postscript_unique_id = Some(*value);
        }
    }
    let Some(range) = private_range(cff_data.data, &top_entries, cff_data.is_cff2)? else {
        return Ok(());
    };
    let private_data = cff_data.data.get(range).ok_or_else(|| Error::Parse {
        table: "CFF/CFF2 Private DICT",
        reason: "private dictionary range is out of bounds".to_owned(),
    })?;
    for entry in dict_entries(private_data, "CFF/CFF2 Private DICT")? {
        match entry {
            dict::Entry::BlueValues(values) => target.blue_values = flatten_blues(&values),
            dict::Entry::OtherBlues(values) => target.other_blues = flatten_blues(&values),
            dict::Entry::FamilyBlues(values) => target.family_blues = flatten_blues(&values),
            dict::Entry::FamilyOtherBlues(values) => {
                target.family_other_blues = flatten_blues(&values)
            }
            dict::Entry::BlueScale(value) => target.blue_scale = Some(value.to_f64()),
            dict::Entry::BlueShift(value) => target.blue_shift = Some(value.to_f64()),
            dict::Entry::BlueFuzz(value) => target.blue_fuzz = Some(value.to_f64()),
            dict::Entry::StdHw(value) => target.std_hw = Some(value.to_f64()),
            dict::Entry::StdVw(value) => target.std_vw = Some(value.to_f64()),
            dict::Entry::StemSnapH(values) => {
                target.stem_snap_h = values.values().iter().map(|value| value.to_f64()).collect()
            }
            dict::Entry::StemSnapV(values) => {
                target.stem_snap_v = values.values().iter().map(|value| value.to_f64()).collect()
            }
            dict::Entry::ForceBold(value) => target.force_bold = Some(value),
            dict::Entry::LanguageGroup(value) => target.language_group = u8::try_from(value).ok(),
            dict::Entry::DefaultWidthX(value) => {
                info.postscript_default_width_x = Some(value.to_f64())
            }
            dict::Entry::NominalWidthX(value) => {
                info.postscript_nominal_width_x = Some(value.to_f64())
            }
            _ => {}
        }
    }
    Ok(())
}

fn cff_font<'a>(face: &FontRef<'a>) -> Result<Option<CffFontRef<'a>>, Error> {
    let Some(data) = face.table_data(read_fonts::types::Tag::new(b"CFF ")) else {
        return Ok(None);
    };
    let upem = face.head().ok().map(|head| i32::from(head.units_per_em()));
    CffFontRef::new_cff(data.as_bytes(), 0, upem)
        .map(Some)
        .map_err(|error| Error::parse("CFF ", error))
}

fn ros(face: &FontRef<'_>) -> Result<Option<(String, String, u32)>, Error> {
    let Some(cff_data) = cff_data_and_top_dict(face)? else {
        return Ok(None);
    };
    let cff = face.cff().map_err(|error| Error::parse("CFF ", error))?;
    for entry in dict_entries(cff_data.top_dict, "CFF Top DICT")? {
        if let dict::Entry::Ros {
            registry,
            ordering,
            supplement,
        } = entry
        {
            let registry = cff_string(&cff, registry)?;
            let ordering = cff_string(&cff, ordering)?;
            return Ok(Some((
                registry,
                ordering,
                supplement.to_f64().max(0.0) as u32,
            )));
        }
    }
    Ok(None)
}

fn cff_string(
    cff: &read_fonts::tables::cff::Cff<'_>,
    sid: read_fonts::ps::string::Sid,
) -> Result<String, Error> {
    let bytes = cff.string(sid).ok_or_else(|| Error::Parse {
        table: "CFF ",
        reason: format!("unresolved string identifier {}", sid.to_u16()),
    })?;
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| Error::parse("CFF ", error))
}

struct CffData<'a> {
    data: &'a [u8],
    top_dict: &'a [u8],
    is_cff2: bool,
}

fn cff_data_and_top_dict<'a>(face: &FontRef<'a>) -> Result<Option<CffData<'a>>, Error> {
    if let Some(data) = face.table_data(read_fonts::types::Tag::new(b"CFF ")) {
        let cff = face.cff().map_err(|error| Error::parse("CFF ", error))?;
        let top = cff
            .top_dicts()
            .get(0)
            .map_err(|error| Error::parse("CFF Top DICT", error))?;
        return Ok(Some(CffData {
            data: data.as_bytes(),
            top_dict: top,
            is_cff2: false,
        }));
    }
    if let Some(data) = face.table_data(read_fonts::types::Tag::new(b"CFF2")) {
        let cff2 = face.cff2().map_err(|error| Error::parse("CFF2", error))?;
        return Ok(Some(CffData {
            data: data.as_bytes(),
            top_dict: cff2.top_dict_data(),
            is_cff2: true,
        }));
    }
    Ok(None)
}

fn private_range(
    data: &[u8],
    top_entries: &[dict::Entry],
    is_cff2: bool,
) -> Result<Option<Range<usize>>, Error> {
    if let Some(range) = top_entries.iter().find_map(|entry| match entry {
        dict::Entry::PrivateDictRange(range) => Some(range.clone()),
        _ => None,
    }) {
        return Ok(Some(range));
    }
    let Some(offset) = top_entries.iter().find_map(|entry| match entry {
        dict::Entry::FdArrayOffset(offset) => Some(*offset),
        _ => None,
    }) else {
        return Ok(None);
    };
    let index_data = data.get(offset..).ok_or_else(|| Error::Parse {
        table: "CFF/CFF2 FDArray",
        reason: "FDArray offset is out of bounds".to_owned(),
    })?;
    let fd_array =
        Index::new(index_data, is_cff2).map_err(|error| Error::parse("CFF/CFF2 FDArray", error))?;
    let default_fd = fd_array
        .get(0)
        .map_err(|error| Error::parse("CFF/CFF2 FDArray", error))?;
    Ok(dict_entries(default_fd, "CFF/CFF2 Font DICT")?
        .into_iter()
        .find_map(|entry| match entry {
            dict::Entry::PrivateDictRange(range) => Some(range),
            _ => None,
        }))
}

fn dict_entries(data: &[u8], table: &'static str) -> Result<Vec<dict::Entry>, Error> {
    dict::entries(data, None)
        .map(|entry| entry.map_err(|error| Error::parse(table, error)))
        .collect()
}

fn flatten_blues(values: &read_fonts::ps::hinting::Blues) -> Vec<f64> {
    values
        .values()
        .iter()
        .flat_map(|(start, end)| [start.to_f64(), end.to_f64()])
        .collect()
}

#[derive(Default)]
struct CffPen {
    contours: Vec<Contour>,
    points: Vec<Point>,
    last_segment_line: bool,
    saw_quadratic: bool,
}

impl CffPen {
    fn push(&mut self, x: f32, y: f32, kind: PointKind) {
        self.points.push(Point {
            x: f64::from(x),
            y: f64::from(y),
            kind,
            ..Point::default()
        });
    }

    fn finish_open(&mut self) {
        if self.points.is_empty() {
            return;
        }
        self.contours.push(Contour {
            closed: false,
            points: std::mem::take(&mut self.points),
            name: None,
            id: None,
        });
    }
}

impl OutlinePen for CffPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish_open();
        self.push(x, y, PointKind::Move);
        self.last_segment_line = false;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(x, y, PointKind::Line);
        self.last_segment_line = true;
    }

    fn quad_to(&mut self, _cx0: f32, _cy0: f32, _x: f32, _y: f32) {
        self.saw_quadratic = true;
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.push(cx0, cy0, PointKind::OffCurve);
        self.push(cx1, cy1, PointKind::OffCurve);
        self.push(x, y, PointKind::Curve);
        self.last_segment_line = false;
    }

    fn close(&mut self) {
        if self.points.is_empty() {
            return;
        }
        if let Some(first) = self.points.first_mut() {
            first.kind = if self.last_segment_line {
                PointKind::Line
            } else {
                PointKind::Curve
            };
        }
        self.contours.push(Contour {
            closed: true,
            points: std::mem::take(&mut self.points),
            name: None,
            id: None,
        });
    }
}

use std::{cell::Cell, collections::BTreeMap, ops::Range};

use fw_model::{CidInfo, Contour, FontInfo, GlyphName, Layer, Point, PointKind, PsPrivate};
use read_fonts::{
    ps::{
        cff::{dict, index::Index, CffFontRef},
        cs,
    },
    types::{Fixed, GlyphId},
    FontRef, TableProvider,
};
use skrifa::{
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlineGlyphCollection, OutlineGlyphFormat, OutlinePen},
};

use crate::Error;

pub(crate) struct CffSource<'a> {
    outlines: OutlineGlyphCollection<'a>,
    font: CffFontRef<'a>,
    has_hints: Cell<bool>,
}

impl<'a> CffSource<'a> {
    pub(crate) fn new(
        face: &FontRef<'a>,
        private_dict_has_hints: bool,
    ) -> Result<Option<Self>, Error> {
        let Some(outlines) = OutlineGlyphCollection::with_format(face, OutlineGlyphFormat::Cff2)
            .or_else(|| OutlineGlyphCollection::with_format(face, OutlineGlyphFormat::Cff))
        else {
            return Ok(None);
        };
        let font = cff_font(face)?.ok_or_else(|| Error::Parse {
            table: "CFF/CFF2",
            reason: "outline collection has no matching CFF table".to_owned(),
        })?;
        Ok(Some(Self {
            outlines,
            font,
            has_hints: Cell::new(private_dict_has_hints),
        }))
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
        self.detect_charstring_hints(gid)?;
        layer.contours = pen.contours;
        Ok(())
    }

    pub(crate) fn has_hints(&self) -> bool {
        self.has_hints.get()
    }

    fn detect_charstring_hints(&self, gid: u16) -> Result<(), Error> {
        if self.has_hints.get() {
            return Ok(());
        }
        let gid = GlyphId::new(u32::from(gid));
        let subfont_index = self.font.subfont_index(gid).unwrap_or(0);
        let subfont = self
            .font
            .subfont(subfont_index, &[])
            .map_err(|error| Error::parse("CFF/CFF2 Private DICT", error))?;
        let mut detector = HintDetector::default();
        self.font
            .evaluate_charstring(&subfont, gid, &[], &mut detector)
            .map_err(|error| Error::parse("CFF/CFF2 CharString", error))?;
        self.has_hints.set(detector.saw_hint);
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
) -> Result<bool, Error> {
    let Some(cff_data) = cff_data_and_top_dict(face)? else {
        return Ok(false);
    };
    let top_entries = dict_entries(cff_data.top_dict, "CFF/CFF2 Top DICT")?;
    for entry in &top_entries {
        if let dict::Entry::UniqueId(value) = entry {
            info.postscript_unique_id = Some(*value);
        }
    }
    let Some(range) = private_range(cff_data.data, &top_entries, cff_data.is_cff2)? else {
        return Ok(false);
    };
    let private_data = cff_data.data.get(range).ok_or_else(|| Error::Parse {
        table: "CFF/CFF2 Private DICT",
        reason: "private dictionary range is out of bounds".to_owned(),
    })?;
    let mut has_hints = false;
    for entry in dict_entries(private_data, "CFF/CFF2 Private DICT")? {
        match entry {
            dict::Entry::BlueValues(values) => {
                has_hints = true;
                target.blue_values = flatten_blues(&values);
            }
            dict::Entry::OtherBlues(values) => target.other_blues = flatten_blues(&values),
            dict::Entry::FamilyBlues(values) => target.family_blues = flatten_blues(&values),
            dict::Entry::FamilyOtherBlues(values) => {
                target.family_other_blues = flatten_blues(&values)
            }
            dict::Entry::BlueScale(value) => target.blue_scale = Some(value.to_f64()),
            dict::Entry::BlueShift(value) => target.blue_shift = Some(value.to_f64()),
            dict::Entry::BlueFuzz(value) => target.blue_fuzz = Some(value.to_f64()),
            dict::Entry::StdHw(value) => {
                has_hints = true;
                target.std_hw = Some(value.to_f64());
            }
            dict::Entry::StdVw(value) => {
                has_hints = true;
                target.std_vw = Some(value.to_f64());
            }
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
    Ok(has_hints)
}

fn cff_font<'a>(face: &FontRef<'a>) -> Result<Option<CffFontRef<'a>>, Error> {
    let upem = face.head().ok().map(|head| i32::from(head.units_per_em()));
    if let Some(data) = face.table_data(read_fonts::types::Tag::new(b"CFF ")) {
        return CffFontRef::new_cff(data.as_bytes(), 0, upem)
            .map(Some)
            .map_err(|error| Error::parse("CFF ", error));
    }
    if let Some(data) = face.table_data(read_fonts::types::Tag::new(b"CFF2")) {
        return CffFontRef::new_cff2(data.as_bytes(), upem)
            .map(Some)
            .map_err(|error| Error::parse("CFF2", error));
    }
    Ok(None)
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
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(x, y, PointKind::Line);
    }

    fn quad_to(&mut self, _cx0: f32, _cy0: f32, _x: f32, _y: f32) {
        self.saw_quadratic = true;
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.push(cx0, cy0, PointKind::OffCurve);
        self.push(cx1, cy1, PointKind::OffCurve);
        self.push(x, y, PointKind::Curve);
    }

    fn close(&mut self) {
        if self.points.is_empty() {
            return;
        }
        let first_position = self.points.first().map(|point| (point.x, point.y));
        let closing_kind = self.points.last().and_then(|last| {
            (last.kind != PointKind::OffCurve && Some((last.x, last.y)) == first_position)
                .then_some(last.kind)
        });
        if let Some(kind) = closing_kind {
            self.points.pop();
            if let Some(first) = self.points.first_mut() {
                first.kind = kind;
            }
        } else if let Some(first) = self.points.first_mut() {
            first.kind = PointKind::Line;
        }
        self.contours.push(Contour {
            closed: true,
            points: std::mem::take(&mut self.points),
            name: None,
            id: None,
        });
    }
}

#[derive(Default)]
struct HintDetector {
    saw_hint: bool,
}

impl cs::CommandSink for HintDetector {
    fn move_to(&mut self, _x: Fixed, _y: Fixed) {}

    fn line_to(&mut self, _x: Fixed, _y: Fixed) {}

    fn curve_to(
        &mut self,
        _cx0: Fixed,
        _cy0: Fixed,
        _cx1: Fixed,
        _cy1: Fixed,
        _x: Fixed,
        _y: Fixed,
    ) {
    }

    fn close(&mut self) {}

    fn hstem(&mut self, _y: Fixed, _dy: Fixed) {
        self.saw_hint = true;
    }

    fn vstem(&mut self, _x: Fixed, _dx: Fixed) {
        self.saw_hint = true;
    }

    fn hint_mask(&mut self, _mask: &[u8]) {
        self.saw_hint = true;
    }

    fn counter_mask(&mut self, _mask: &[u8]) {
        self.saw_hint = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_curve_uses_final_endpoint_kind_without_repeating_start() {
        let mut pen = CffPen::default();
        pen.move_to(0.0, 0.0);
        pen.curve_to(1.0, 0.0, 1.0, 1.0, 0.0, 0.0);
        pen.close();

        let contour = &pen.contours[0];
        assert_eq!(contour.points[0].kind, PointKind::Curve);
        assert_eq!(contour.points.len(), 3);
        assert_ne!(
            (contour.points[0].x, contour.points[0].y),
            (contour.points[2].x, contour.points[2].y)
        );
        assert!(contour.try_to_bezpath().is_ok());
    }

    #[test]
    fn implicit_close_is_a_line_when_final_endpoint_differs() {
        let mut pen = CffPen::default();
        pen.move_to(0.0, 0.0);
        pen.curve_to(1.0, 0.0, 1.0, 1.0, 0.0, 1.0);
        pen.close();

        let contour = &pen.contours[0];
        assert_eq!(contour.points[0].kind, PointKind::Line);
        assert_eq!(
            contour.points.last().expect("last point").kind,
            PointKind::Curve
        );
        assert!(contour.try_to_bezpath().is_ok());
    }

    #[test]
    fn open_contour_retains_move_and_explicit_final_endpoint() {
        let mut pen = CffPen::default();
        pen.move_to(0.0, 0.0);
        pen.curve_to(1.0, 0.0, 1.0, 1.0, 0.0, 0.0);
        pen.finish_open();

        let contour = &pen.contours[0];
        assert!(!contour.closed);
        assert_eq!(contour.points[0].kind, PointKind::Move);
        assert_eq!(
            contour.points.last().expect("last point").kind,
            PointKind::Curve
        );
        assert_eq!(contour.points.len(), 4);
        assert!(contour.try_to_bezpath().is_ok());
    }
}

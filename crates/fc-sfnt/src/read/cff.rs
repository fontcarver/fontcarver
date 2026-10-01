use std::{cell::Cell, collections::BTreeMap};

use fc_model::{CidInfo, Contour, GlyphName, Layer, Point, PointKind};
use kurbo::Affine;
use read_fonts::{
    ps::{
        cff::{dict, CffFontRef},
        cs,
        transform::Transform,
    },
    types::{Fixed, GlyphId},
    FontRef, TableProvider,
};

use crate::Error;
mod flex;
mod hints;
mod private;
pub(crate) use private::apply_private;

pub(crate) struct CffSource<'a> {
    font: CffFontRef<'a>,
    is_cff2: bool,
    private_ranges: Vec<std::ops::Range<usize>>,
    has_hints: Cell<bool>,
    lost_hints: Cell<bool>,
    flex_budget: Cell<usize>,
    lost_flex: Cell<bool>,
    dropped_dotsection: Cell<bool>,
}

impl<'a> CffSource<'a> {
    pub(crate) fn new(
        face: &FontRef<'a>,
        private_dict_has_hints: bool,
    ) -> Result<Option<Self>, Error> {
        let Some(font) = cff_font(face)? else {
            return Ok(None);
        };
        Ok(Some(Self {
            font,
            is_cff2: cff_data_and_top_dict(face)?.is_some_and(|data| data.is_cff2),
            private_ranges: private::ranges(face)?,
            has_hints: Cell::new(private_dict_has_hints),
            lost_hints: Cell::new(false),
            flex_budget: Cell::new(16_000_000),
            lost_flex: Cell::new(false),
            dropped_dotsection: Cell::new(false),
        }))
    }

    pub(crate) fn apply(&self, gid: u16, layer: &mut Layer) -> Result<(), Error> {
        let gid = GlyphId::new(u32::from(gid));
        if gid.to_u32() >= self.font.num_glyphs() {
            return Ok(());
        }
        let subfont_index = self.font.subfont_index(gid).ok_or_else(|| Error::Parse {
            table: "CFF/CFF2 FDSelect",
            reason: format!("missing Font DICT selection for glyph {}", gid.to_u32()),
        })?;
        if self.font.is_cid() {
            layer.cff_private_dict_index =
                Some(u8::try_from(subfont_index).map_err(|_| Error::Parse {
                    table: "CFF FDSelect",
                    reason: "CFF1 Font DICT index exceeds 255".into(),
                })?);
        }
        let subfont = self
            .font
            .subfont(subfont_index, &[])
            .map_err(|error| Error::parse("CFF/CFF2 Private DICT", error))?;
        let transform = effective_transform(self.font.transform(&subfont, None));
        let mut pen = CffPen::new(self.font.transform(&subfont, None));
        self.font
            .evaluate_charstring(&subfont, gid, &[], &mut pen)
            .map_err(|error| Error::parse("CFF/CFF2 CharString", error))?;
        pen.finish_open();
        pen.hints.finish();
        self.has_hints.set(self.has_hints.get() || pen.saw_hint);
        self.lost_hints.set(self.lost_hints.get() || pen.hints.lost);
        layer.contours = pen.contours;
        layer.hints = pen.hints.values;
        self.recover_flex(gid, subfont_index, transform, layer);
        Ok(())
    }

    /// Recovers flex hints the outline dependency expanded into ordinary curves.
    ///
    /// The scan is independent of the evaluator, so a charstring it cannot follow
    /// only costs a disclosure, never a wrong hint. CFF2 charstrings are skipped:
    /// their blended operands belong to a variation model the importer already
    /// flattens to the default instance, which it discloses separately.
    fn recover_flex(&self, gid: GlyphId, subfont_index: u16, transform: Affine, layer: &mut Layer) {
        if self.is_cff2 || self.flex_budget.get() == 0 {
            return;
        }
        let Ok(charstring) = self.font.charstrings().get(gid.to_u32() as usize) else {
            return;
        };
        let subroutines = flex::Subroutines {
            local: self
                .private_ranges
                .get(usize::from(subfont_index))
                .and_then(|range| private::local_subrs(self.font.data(), range)),
            global: self.font.global_subrs(),
        };
        let mut budget = self.flex_budget.get();
        let found = flex::scan(charstring, &subroutines, &mut budget);
        self.flex_budget.set(budget);
        let Ok(found) = found else {
            self.lost_flex.set(true);
            return;
        };
        self.dropped_dotsection
            .set(self.dropped_dotsection.get() || found.dotsection);
        if found.flexes.is_empty() {
            return;
        }
        if found.seac || !bind_flex(layer, &found.flexes, transform) {
            layer.hints.flex_hints.clear();
            layer.hints.flex = true;
            self.lost_flex.set(true);
        }
    }

    pub(crate) fn has_hints(&self) -> bool {
        self.has_hints.get()
    }

    pub(crate) fn lost_hints(&self) -> bool {
        self.has_hints() && self.lost_hints.get()
    }

    pub(crate) fn unpreserved_private_dicts(&self) -> bool {
        !self.font.is_cid() && self.font.num_subfonts() > 1
    }

    /// Whether some flex hint could not be bound to the evaluated outline.
    pub(crate) fn lost_flex(&self) -> bool {
        self.lost_flex.get()
    }

    /// Whether a charstring used the obsolete Type 2 `dotsection` operator.
    pub(crate) fn dropped_dotsection(&self) -> bool {
        self.dropped_dotsection.get()
    }
}

/// Binds scanned flex geometry to the stored points the evaluator produced.
///
/// A flex is accepted only when exactly one run of seven stored points matches
/// its coordinates and point kinds, so an ambiguous match is disclosed instead of
/// attaching the hint to the wrong curve pair.
fn bind_flex(layer: &mut Layer, flexes: &[flex::Flex], transform: Affine) -> bool {
    let mut bound = Vec::new();
    for found in flexes {
        let points = found.points.map(|point| transform * point);
        let mut matches = Vec::new();
        for (index, contour) in layer.contours.iter().enumerate() {
            let count = contour.points.len();
            if count < 7 {
                continue;
            }
            for start in 0..count {
                if !matches_flex(contour, start, &points) {
                    continue;
                }
                matches.push((index, start));
            }
        }
        let [single] = matches.as_slice() else {
            return false;
        };
        bound.push(fc_model::FlexHint {
            contour: single.0,
            point: single.1,
            depth: found.depth,
        });
    }
    layer.hints.flex_hints = bound;
    // Reject anything the editable model cannot hold, such as a transformed flex
    // deeper than the mechanism allows.
    layer.hints.validate(&layer.contours).is_ok()
}

fn matches_flex(contour: &Contour, start: usize, points: &[kurbo::Point; 7]) -> bool {
    let count = contour.points.len();
    let kinds = [
        PointKind::OffCurve,
        PointKind::OffCurve,
        PointKind::Curve,
        PointKind::OffCurve,
        PointKind::OffCurve,
        PointKind::Curve,
    ];
    if contour.points[start].kind == PointKind::OffCurve {
        return false;
    }
    if !contour.closed && start + 6 >= count {
        return false;
    }
    for (offset, expected) in points.iter().enumerate() {
        let point = &contour.points[(start + offset) % count];
        if (point.x - expected.x).abs() > 1e-6 || (point.y - expected.y).abs() > 1e-6 {
            return false;
        }
        if offset > 0 && point.kind != kinds[offset - 1] {
            return false;
        }
    }
    true
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

fn dict_entries(data: &[u8], table: &'static str) -> Result<Vec<dict::Entry>, Error> {
    dict::entries(data, None)
        .map(|entry| entry.map_err(|error| Error::parse(table, error)))
        .collect()
}

fn effective_transform(transform: Transform) -> Affine {
    let matrix = transform.matrix;
    // read-fonts normalizes matrices for rendering: linear coefficients are
    // 16.16, translation Fixed bits contain integer font units, and optional
    // scale targets 26.6. Retain f64 precision for editable geometry and hints.
    let scale = transform.scale.map_or(1.0, |scale| scale.to_f64() / 64.0);
    Affine::scale(scale)
        * Affine::new([
            matrix.xx.to_f64(),
            matrix.yx.to_f64(),
            matrix.xy.to_f64(),
            matrix.yy.to_f64(),
            f64::from(matrix.dx.to_bits()),
            f64::from(matrix.dy.to_bits()),
        ])
}

#[derive(Default)]
struct CffPen {
    contours: Vec<Contour>,
    points: Vec<Point>,
    transform: Affine,
    saw_hint: bool,
    hints: hints::Hints,
}

impl CffPen {
    fn new(transform: Transform) -> Self {
        Self {
            transform: effective_transform(transform),
            ..Self::default()
        }
    }

    fn push(&mut self, x: Fixed, y: Fixed, kind: PointKind) {
        if kind != PointKind::OffCurve {
            self.hints.point(self.contours.len(), self.points.len());
        }
        let position = self.transform * kurbo::Point::new(x.to_f64(), y.to_f64());
        self.points.push(Point {
            x: position.x,
            y: position.y,
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

impl cs::CommandSink for CffPen {
    fn move_to(&mut self, x: Fixed, y: Fixed) {
        self.finish_open();
        self.push(x, y, PointKind::Move);
    }

    fn line_to(&mut self, x: Fixed, y: Fixed) {
        let position = self.transform * kurbo::Point::new(x.to_f64(), y.to_f64());
        if self
            .points
            .last()
            .is_some_and(|p| p.x == position.x && p.y == position.y)
        {
            return;
        }
        self.push(x, y, PointKind::Line);
    }

    fn curve_to(&mut self, cx0: Fixed, cy0: Fixed, cx1: Fixed, cy1: Fixed, x: Fixed, y: Fixed) {
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
            self.hints.close(self.contours.len(), self.points.len() - 1);
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

    fn hstem(&mut self, y: Fixed, dy: Fixed) {
        self.saw_hint = true;
        // fontations emits the two absolute edges, despite the trait's dy label.
        self.hints
            .stem(true, y.to_f64(), dy.to_f64() - y.to_f64(), self.transform);
    }

    fn vstem(&mut self, x: Fixed, dx: Fixed) {
        self.saw_hint = true;
        self.hints
            .stem(false, x.to_f64(), dx.to_f64() - x.to_f64(), self.transform);
    }

    fn hint_mask(&mut self, mask: &[u8]) {
        self.saw_hint = true;
        self.hints.mask(false, mask);
    }

    fn counter_mask(&mut self, mask: &[u8]) {
        self.saw_hint = true;
        self.hints.mask(true, mask);
    }

    fn clear_hints(&mut self) {
        self.hints.clear();
    }

    fn finish(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod fractional_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use cs::CommandSink;

    #[test]
    fn closed_curve_uses_final_endpoint_kind_without_repeating_start() {
        let mut pen = CffPen::default();
        pen.move_to(Fixed::ZERO, Fixed::ZERO);
        pen.curve_to(
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ZERO,
        );
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
        pen.move_to(Fixed::ZERO, Fixed::ZERO);
        pen.curve_to(
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ONE,
        );
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
        pen.move_to(Fixed::ZERO, Fixed::ZERO);
        pen.curve_to(
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ZERO,
        );
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

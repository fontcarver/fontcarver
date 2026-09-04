//! The curve family a font declares for its outlines.

use crate::{Font, Master, MasterId, Tag};

/// TrueType outline table tag.
const GLYF: Tag = Tag(*b"glyf");
/// CFF outline table tag.
const CFF: Tag = Tag(*b"CFF ");
/// CFF2 outline table tag.
const CFF2: Tag = Tag(*b"CFF2");

/// The curve family a font stores its outlines in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveFormat {
    /// Quadratic outlines, as in a TrueType `glyf` table.
    Quadratic,
    /// Cubic outlines, as in a PostScript `CFF` table.
    Cubic,
}

/// Reports the curve family the font itself declares, when it declares one.
///
/// The model has no explicit outline-format flag, so the answer is derived from
/// data an import preserves, in this order:
///
/// 1. the imported source's table list: `glyf` means quadratic, `CFF ` or
///    `CFF2` means cubic (a font carrying both is read as quadratic, because
///    `glyf` is what an editing session rasterizes);
/// 2. TrueType master data — a control value table, a font or control value
///    program, or any `maxp` maximum — on `master`, or on any master when
///    `master` is [`None`] or unknown: quadratic.
///
/// [`None`] means the font does not say. Callers then fall back to inspecting
/// the outlines themselves, which is all a font drawn from scratch offers.
pub fn declared_curve_format(font: &Font, master: Option<&MasterId>) -> Option<CurveFormat> {
    if let Some(source) = &font.extra.source {
        if source.table_tags.contains(&GLYF) {
            return Some(CurveFormat::Quadratic);
        }
        if source.table_tags.contains(&CFF) || source.table_tags.contains(&CFF2) {
            return Some(CurveFormat::Cubic);
        }
    }
    let named = master.and_then(|id| font.master(id));
    let truetype = match named {
        Some(master) => has_truetype_data(master),
        None => font.masters.iter().any(has_truetype_data),
    };
    truetype.then_some(CurveFormat::Quadratic)
}

/// Reports whether a master carries TrueType programs or maxima.
fn has_truetype_data(master: &Master) -> bool {
    let tt = &master.tt;
    !tt.cvt.is_empty()
        || !tt.fpgm.is_empty()
        || !tt.prep.is_empty()
        || tt.max_stack_elements.is_some()
        || tt.max_storage.is_some()
        || tt.max_function_defs.is_some()
        || tt.max_instruction_defs.is_some()
        || tt.max_twilight_points.is_some()
        || tt.max_zones.is_some()
        || tt.max_size_of_instructions.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceProvenance;

    fn font_with_tags(tags: &[Tag]) -> Font {
        let mut font = Font::new_empty("Curves", 1_000);
        font.extra.source = Some(SourceProvenance {
            table_tags: tags.to_vec(),
            ..SourceProvenance::default()
        });
        font
    }

    #[test]
    fn a_font_drawn_from_scratch_declares_nothing() {
        let font = Font::new_empty("Curves", 1_000);
        assert_eq!(declared_curve_format(&font, None), None);
    }

    #[test]
    fn imported_table_tags_decide_the_format() {
        let quadratic = font_with_tags(&[Tag(*b"head"), GLYF]);
        assert_eq!(
            declared_curve_format(&quadratic, None),
            Some(CurveFormat::Quadratic)
        );
        let cubic = font_with_tags(&[Tag(*b"head"), CFF]);
        assert_eq!(
            declared_curve_format(&cubic, None),
            Some(CurveFormat::Cubic)
        );
        let cff2 = font_with_tags(&[CFF2]);
        assert_eq!(declared_curve_format(&cff2, None), Some(CurveFormat::Cubic));
        assert_eq!(declared_curve_format(&font_with_tags(&[]), None), None);
    }

    #[test]
    fn truetype_master_data_declares_quadratic_outlines() {
        let mut font = Font::new_empty("Curves", 1_000);
        let id = font.default_master().id.clone();
        assert_eq!(declared_curve_format(&font, Some(&id)), None);
        if let Some(master) = font.masters.first_mut() {
            master.tt.prep = vec![0x00];
        }
        assert_eq!(
            declared_curve_format(&font, Some(&id)),
            Some(CurveFormat::Quadratic)
        );
        assert_eq!(
            declared_curve_format(&font, None),
            Some(CurveFormat::Quadratic)
        );
    }

    #[test]
    fn a_master_without_truetype_data_declares_nothing() {
        let mut font = Font::new_empty("Curves", 1_000);
        let mut second = Master::default();
        second.tt.max_zones = Some(2);
        let plain = font.default_master().id.clone();
        let hinted = second.id.clone();
        font.add_master(second);
        assert_eq!(declared_curve_format(&font, Some(&plain)), None);
        assert_eq!(
            declared_curve_format(&font, Some(&hinted)),
            Some(CurveFormat::Quadratic)
        );
    }
}

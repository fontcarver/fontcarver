//! PostScript hint invariants shared by authoring and output writers.

use crate::{Contour, Error, PointKind, PsHints, PsPrivate};

fn invalid(message: &str) -> Error {
    Error::InvalidHints(message.into())
}

impl PsPrivate {
    /// Checks Type 1/CFF private dictionary limits without changing values.
    pub fn validate(&self) -> Result<(), Error> {
        for (values, limit) in [
            (&self.blue_values, 14),
            (&self.other_blues, 10),
            (&self.family_blues, 14),
            (&self.family_other_blues, 10),
        ] {
            // The first BlueValues pair is the baseline; later pairs may occur
            // in any order (Adobe Type 1 specification, section 5.3).
            if values.len() > limit
                || !values.len().is_multiple_of(2)
                || values.iter().any(|v| !v.is_finite() || v.abs() > 32767.0)
                || values.chunks_exact(2).any(|pair| pair[0] > pair[1])
                || values.chunks_exact(2).enumerate().any(|(index, pair)| {
                    values
                        .chunks_exact(2)
                        .skip(index + 1)
                        .any(|other| pair[0] <= other[1] && other[0] <= pair[1])
                })
            {
                return Err(invalid(
                    "blue zones must contain nonoverlapping low/high pairs within format limits",
                ));
            }
        }
        for values in [&self.stem_snap_h, &self.stem_snap_v] {
            if values.len() > 12
                || values
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0.0 || *v > 32767.0)
                || values.windows(2).any(|p| p[0] >= p[1])
            {
                return Err(invalid(
                    "stem snaps must contain at most 12 increasing positive widths",
                ));
            }
        }
        for value in [self.std_hw, self.std_vw].into_iter().flatten() {
            if !value.is_finite() || !(0.0..=32767.0).contains(&value) || value == 0.0 {
                return Err(invalid("standard stem widths must be finite and positive"));
            }
        }
        if self
            .blue_scale
            .is_some_and(|v| !v.is_finite() || v <= 0.0 || v > 1.0)
            || [self.blue_shift, self.blue_fuzz]
                .into_iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=32767.0).contains(&v))
            || self.language_group.is_some_and(|v| v > 1)
        {
            return Err(invalid(
                "invalid BlueScale, BlueShift, BlueFuzz or LanguageGroup",
            ));
        }
        Ok(())
    }
}

impl PsHints {
    /// Checks hint geometry, ghost conventions and masks against stored points.
    pub fn validate(&self, contours: &[Contour]) -> Result<(), Error> {
        let count = self.h_stems.len() + self.v_stems.len();
        if count > 96 || self.d_stems.len() > 96 || self.hint_masks.len() > 65_535 {
            return Err(invalid("PostScript hint count exceeds format limits"));
        }
        for stems in [&self.h_stems, &self.v_stems] {
            for stem in stems {
                let ghost = stem.width == -20.0 || stem.width == -21.0;
                if !stem.start.is_finite()
                    || stem.start.abs() > 32767.0
                    || !stem.width.is_finite()
                    || (stem.width <= 0.0 && !ghost)
                    || stem.width > 32767.0
                    || (stem.start + stem.width).abs() > 32767.0
                    || stem.ranges.len() > 65_535
                    || stem
                        .ranges
                        .iter()
                        .any(|(a, b)| !a.is_finite() || !b.is_finite() || a > b)
                {
                    return Err(invalid(
                        "invalid stem coordinate, width, ghost or active range",
                    ));
                }
            }
        }
        for stem in &self.d_stems {
            if [
                stem.left.0,
                stem.left.1,
                stem.right.0,
                stem.right.1,
                stem.unit.0,
                stem.unit.1,
            ]
            .iter()
            .any(|v| !v.is_finite())
                || (stem.unit.0.hypot(stem.unit.1) - 1.0).abs() > 0.0001
            {
                return Err(invalid(
                    "diagonal hints require finite points and a unit direction",
                ));
            }
        }
        let mut references = std::collections::HashSet::new();
        for mask in &self.hint_masks {
            let point = contours
                .get(mask.contour)
                .and_then(|c| c.points.get(mask.point));
            if count == 0
                || mask.mask.len() != count.div_ceil(8)
                || point.is_none_or(|p| p.kind == PointKind::OffCurve)
                || (!mask.counter && !references.insert((mask.contour, mask.point, mask.closing)))
                || (mask.closing
                    && contours.get(mask.contour).is_none_or(|c| {
                        !c.closed
                            || c.points.iter().position(|p| p.kind != PointKind::OffCurve)
                                != Some(mask.point)
                    }))
                || (!count.is_multiple_of(8)
                    && mask
                        .mask
                        .last()
                        .is_some_and(|b| b & ((1 << (8 - count % 8)) - 1) != 0))
            {
                return Err(invalid(
                    "hint masks must reference distinct on-curve points and match the stem count",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::PsPrivate;

    #[test]
    fn blue_zone_pair_order_is_preserved_but_each_pair_must_be_valid() {
        let mut private = PsPrivate {
            blue_values: vec![-15.0, 0.0, 700.0, 715.0, 547.0, 559.0],
            ..Default::default()
        };
        assert!(private.validate().is_ok());
        private.blue_values[4..].copy_from_slice(&[-12.0, -2.0]);
        assert!(private.validate().is_err());
        private.blue_values[4..].copy_from_slice(&[559.0, 547.0]);
        assert!(private.validate().is_err());
    }
}

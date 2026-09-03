//! Interpolation compatibility checks.

use serde::{Deserialize, Serialize};

use crate::{Font, GlyphName, MasterId, PointKind};

/// One reason a glyph's master layers cannot interpolate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum Incompatibility {
    /// The requested glyph does not exist.
    MissingGlyph {
        /// Requested glyph name.
        glyph: GlyphName,
    },
    /// A master layer is absent.
    MissingMasterLayer {
        /// Master without a layer.
        master: MasterId,
    },
    /// Contour counts differ.
    ContourCount {
        /// Master being compared.
        master: MasterId,
        /// Count in the reference master.
        expected: usize,
        /// Count in this master.
        actual: usize,
    },
    /// Point counts differ for a contour.
    PointCount {
        /// Master being compared.
        master: MasterId,
        /// Contour index.
        contour: usize,
        /// Count in the reference master.
        expected: usize,
        /// Count in this master.
        actual: usize,
    },
    /// Point kinds differ.
    PointKind {
        /// Master being compared.
        master: MasterId,
        /// Contour index.
        contour: usize,
        /// Point index.
        point: usize,
        /// Kind in the reference master.
        expected: PointKind,
        /// Kind in this master.
        actual: PointKind,
    },
    /// Component counts differ.
    ComponentCount {
        /// Master being compared.
        master: MasterId,
        /// Count in the reference master.
        expected: usize,
        /// Count in this master.
        actual: usize,
    },
    /// Component bases differ at the same index.
    ComponentBase {
        /// Master being compared.
        master: MasterId,
        /// Component index.
        component: usize,
        /// Base in the reference master.
        expected: GlyphName,
        /// Base in this master.
        actual: GlyphName,
    },
}

/// Checks contour topology and component lists across all masters of a glyph.
pub fn masters_compatible(font: &Font, glyph: &str) -> Result<(), Vec<Incompatibility>> {
    let Some(glyph_data) = font.glyph(glyph) else {
        return Err(vec![Incompatibility::MissingGlyph {
            glyph: glyph.into(),
        }]);
    };
    let mut issues = Vec::new();
    let reference = font
        .masters
        .iter()
        .find_map(|master| glyph_data.master_layer(&master.id));

    for master in &font.masters {
        let Some(layer) = glyph_data.master_layer(&master.id) else {
            issues.push(Incompatibility::MissingMasterLayer {
                master: master.id.clone(),
            });
            continue;
        };
        let Some(reference) = reference else {
            continue;
        };
        compare_layers(reference, layer, &master.id, &mut issues);
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn compare_layers(
    reference: &crate::Layer,
    layer: &crate::Layer,
    master: &MasterId,
    issues: &mut Vec<Incompatibility>,
) {
    if reference.contours.len() != layer.contours.len() {
        issues.push(Incompatibility::ContourCount {
            master: master.clone(),
            expected: reference.contours.len(),
            actual: layer.contours.len(),
        });
    }
    for (contour_index, (expected, actual)) in
        reference.contours.iter().zip(&layer.contours).enumerate()
    {
        if expected.points.len() != actual.points.len() {
            issues.push(Incompatibility::PointCount {
                master: master.clone(),
                contour: contour_index,
                expected: expected.points.len(),
                actual: actual.points.len(),
            });
        }
        for (point_index, (expected, actual)) in
            expected.points.iter().zip(&actual.points).enumerate()
        {
            if expected.kind != actual.kind {
                issues.push(Incompatibility::PointKind {
                    master: master.clone(),
                    contour: contour_index,
                    point: point_index,
                    expected: expected.kind,
                    actual: actual.kind,
                });
            }
        }
    }
    if reference.components.len() != layer.components.len() {
        issues.push(Incompatibility::ComponentCount {
            master: master.clone(),
            expected: reference.components.len(),
            actual: layer.components.len(),
        });
    }
    for (component, (expected, actual)) in reference
        .components
        .iter()
        .zip(&layer.components)
        .enumerate()
    {
        if expected.base != actual.base {
            issues.push(Incompatibility::ComponentBase {
                master: master.clone(),
                component,
                expected: expected.base.clone(),
                actual: actual.base.clone(),
            });
        }
    }
}

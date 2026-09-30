//! Persistent, ordered layer effects. Geometry evaluation lives in `fc-geom`.

use kurbo::{BezPath, PathEl};
use serde::{Deserialize, Serialize};

use crate::{Error, Font, Layer};

/// Maximum number of operations stored on one layer, including disabled entries.
pub const MAX_LAYER_EFFECTS: usize = 16;

/// A reversible operation applied after the layer's components are resolved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerEffect {
    /// Disabled entries retain their parameters without changing the outline.
    #[serde(default = "enabled")]
    pub enabled: bool,
    /// Operation and its parameters.
    pub effect: LayerEffectKind,
}

fn enabled() -> bool {
    true
}

/// Supported effects, evaluated in list order in layer coordinates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LayerEffectKind {
    /// Expand source centerlines into a filled outline.
    Stroke {
        /// Pen shape before rotation.
        nib: EffectNib,
        /// Full pen width in font units.
        width: f64,
        /// Full pen height in font units.
        height: f64,
        /// Counterclockwise pen rotation in degrees.
        angle: f64,
        /// Endpoint treatment for open contours.
        cap: EffectCap,
        /// Corner treatment.
        join: EffectJoin,
        /// Maximum miter length as a multiple of stroke width.
        miter_limit: f64,
    },
    /// Expand centerlines using pen dimensions sampled along each contour.
    VariableStroke {
        /// Pen shape before rotation.
        nib: EffectNib,
        /// Endpoint treatment for open contours.
        cap: EffectCap,
        /// Corner treatment.
        join: EffectJoin,
        /// Maximum miter length as a multiple of the larger pen radius.
        miter_limit: f64,
        /// Interpolation between adjacent normalized positions.
        progression: EffectProgression,
        /// Ordered pen samples spanning normalized contour length zero to one.
        points: Vec<PenPoint>,
    },
    /// Remove interior edges using the nonzero winding rule.
    RemoveOverlap,
    /// Combine the current fill with a stored copy of a closed mask outline.
    Boolean {
        /// Filled-region operation; subtraction removes the mask from the source.
        operation: EffectBoolean,
        /// SVG path data in layer coordinates, independent of later source edits.
        path: String,
    },
}

/// Interpolation used between neighboring variable pen points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EffectProgression {
    /// Constant rate of change between points.
    Linear,
    /// Cubic smoothstep, with zero parameter slope at each pen point.
    Smooth,
}

/// One pen sample at a normalized centerline arc-length position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct PenPoint {
    /// Normalized distance along each source contour, in `[0,1]`.
    pub position: f64,
    /// Full pen width in font units.
    pub width: f64,
    /// Full pen height in font units.
    pub height: f64,
    /// Counterclockwise rotation in degrees.
    pub angle: f64,
}

impl PenPoint {
    /// Interpolate dimensions and the shortest angular interval at a position.
    pub fn interpolate(a: &Self, b: &Self, position: f64, progression: EffectProgression) -> Self {
        let t = ((position - a.position) / (b.position - a.position)).clamp(0., 1.);
        let t = match progression {
            EffectProgression::Linear => t,
            EffectProgression::Smooth => t * t * (3. - 2. * t),
        };
        let angle = (b.angle - a.angle + 180.).rem_euclid(360.) - 180.;
        Self {
            position,
            width: a.width + (b.width - a.width) * t,
            height: a.height + (b.height - a.height) * t,
            angle: a.angle + angle * t,
        }
    }
}

/// Shape of a layer-effect pen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EffectNib {
    /// Ellipse, including a circle when both dimensions are equal.
    Ellipse,
    /// Rectangular calligraphic nib.
    Rectangle,
}

/// Stroke cap used by a layer effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EffectCap {
    /// Half-circle in normalized pen coordinates.
    Round,
    /// End at the source endpoint.
    Butt,
    /// Extend by half a pen width.
    Square,
}

/// Stroke corner used by a layer effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EffectJoin {
    /// Rounded corner.
    Round,
    /// Straight bevel.
    Bevel,
    /// Extended intersection, limited by the miter limit.
    Miter,
}

/// Boolean fill operation for a stored mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub enum EffectBoolean {
    /// Either filled region.
    Union,
    /// Shared filled region.
    Intersect,
    /// Source with the mask removed.
    Subtract,
    /// Regions filled by exactly one operand.
    Xor,
}

impl Layer {
    /// Whether source-node corner intent contributes to the evaluated outline.
    pub fn has_smart_corners(&self) -> bool {
        self.contours
            .iter()
            .any(|c| c.points.iter().any(|p| p.corner_radius != 0.))
    }

    /// Whether local source geometry needs live construction or effect evaluation.
    pub fn has_live_geometry(&self) -> bool {
        self.has_smart_corners()
            || !self.constructions.is_empty()
            || !self.node_links.is_empty()
            || !self.linked_guides.is_empty()
            || self.effects.iter().any(|effect| effect.enabled)
    }

    /// Validate every stored operation, including disabled entries.
    pub fn validate_effects(&self) -> Result<(), Error> {
        self.validate_constructions()?;
        let mut corners = 0;
        for contour in &self.contours {
            for (index, point) in contour.points.iter().enumerate() {
                let radius = point.corner_radius;
                if radius == 0. {
                    continue;
                }
                corners += 1;
                if corners > 1024 || !radius.is_finite() || radius.abs() > 8192. {
                    return Err(invalid(
                        "at most 1024 smart corners per layer, with radius magnitude at most 8192",
                    ));
                }
                if point.kind == crate::PointKind::OffCurve
                    || (!contour.closed && (index == 0 || index + 1 == contour.points.len()))
                {
                    return Err(invalid("smart corners require an interior on-curve node"));
                }
                if radius < 0. && !contour.closed {
                    return Err(invalid("ink traps require closed contours"));
                }
            }
        }
        if self.effects.len() > MAX_LAYER_EFFECTS {
            return Err(invalid("a layer may contain at most 16 effects"));
        }
        for entry in &self.effects {
            match &entry.effect {
                LayerEffectKind::Stroke {
                    width,
                    height,
                    angle,
                    miter_limit,
                    ..
                } => {
                    if !width.is_finite()
                        || !(0.1..=8192.).contains(width)
                        || !height.is_finite()
                        || !(0.1..=8192.).contains(height)
                        || !angle.is_finite()
                        || angle.abs() > 360.
                        || !miter_limit.is_finite()
                        || !(1.0..=100.).contains(miter_limit)
                        || width.max(*height) / width.min(*height) > 100.
                    {
                        return Err(invalid("invalid pen dimensions, angle or miter limit (maximum aspect ratio 100)"));
                    }
                }
                LayerEffectKind::VariableStroke {
                    points,
                    miter_limit,
                    ..
                } => {
                    if !(2..=64).contains(&points.len())
                        || points.first().is_none_or(|p| p.position != 0.)
                        || points.last().is_none_or(|p| p.position != 1.)
                        || points.windows(2).any(|p| p[0].position >= p[1].position)
                        || !miter_limit.is_finite()
                        || !(1.0..=100.).contains(miter_limit)
                        || points.iter().any(|p| {
                            !p.position.is_finite()
                                || !(0.0..=1.).contains(&p.position)
                                || !p.width.is_finite()
                                || !(0.1..=8192.).contains(&p.width)
                                || !p.height.is_finite()
                                || !(0.1..=8192.).contains(&p.height)
                                || p.width.max(p.height) / p.width.min(p.height) > 100.
                                || !p.angle.is_finite()
                                || p.angle.abs() > 360.
                        })
                    {
                        return Err(invalid("variable pen requires 2–64 ordered points from 0 to 1, valid dimensions/angles and miter limit"));
                    }
                }
                LayerEffectKind::Boolean { path, .. } => {
                    validate_effect_mask(path)?;
                }
                LayerEffectKind::RemoveOverlap => {}
            }
        }
        Ok(())
    }
}

impl Font {
    /// Validate persistent layer-effect stacks throughout the document.
    pub fn validate_effects(&self) -> Result<(), Error> {
        for glyph in self.glyphs.values() {
            for layer in &glyph.layers {
                layer.validate_effects()?;
            }
        }
        Ok(())
    }
}

/// Parse a bounded, explicitly closed mask path in the signed font coordinate grid.
pub fn validate_effect_mask(source: &str) -> Result<BezPath, Error> {
    if source.len() > 262_144 {
        return Err(invalid("effect mask exceeds 256 KiB"));
    }
    let path =
        BezPath::from_svg(source).map_err(|e| invalid(&format!("invalid mask path: {e}")))?;
    if path.elements().len() > 4096 || !path.is_finite() {
        return Err(invalid(
            "mask exceeds 4096 path elements or has nonfinite coordinates",
        ));
    }
    let mut open = false;
    for element in path.elements() {
        if !open
            && matches!(
                element,
                PathEl::LineTo(_) | PathEl::QuadTo(..) | PathEl::CurveTo(..)
            )
        {
            return Err(invalid(
                "mask drawing must begin with a move after each close",
            ));
        }
        let points = match element {
            PathEl::MoveTo(p) => {
                if open {
                    return Err(invalid("mask contours must be explicitly closed"));
                }
                open = true;
                vec![*p]
            }
            PathEl::LineTo(p) => vec![*p],
            PathEl::QuadTo(p, q) => vec![*p, *q],
            PathEl::CurveTo(p, q, r) => vec![*p, *q, *r],
            PathEl::ClosePath => {
                open = false;
                vec![]
            }
        };
        if points
            .iter()
            .any(|p| p.x.abs() > 32767. || p.y.abs() > 32767.)
        {
            return Err(invalid("mask coordinates exceed the signed font grid"));
        }
    }
    if open {
        return Err(invalid("mask contours must be explicitly closed"));
    }
    Ok(path)
}

fn invalid(message: &str) -> Error {
    Error::InvalidEffects(message.to_owned())
}

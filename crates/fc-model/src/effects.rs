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
    /// Validate every stored operation, including disabled entries.
    pub fn validate_effects(&self) -> Result<(), Error> {
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

use crate::Incompatibility;

/// Errors returned by geometry operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid or excessive style-effect input.
    #[error("style effect: {0}")]
    InvalidStyle(String),
    /// Invalid coordinates or sources prevent interpolation.
    #[error("invalid design space: {0}")]
    DesignSpace(String),
    /// A tolerance, grid step, stroke width, or distance was not finite and positive.
    #[error("{parameter} must be finite and positive")]
    InvalidParameter {
        /// Name of the invalid parameter.
        parameter: &'static str,
    },

    /// The curve-preserving boolean engine could not process the supplied outlines.
    #[error("boolean operation failed: {reason}")]
    BooleanFailed {
        /// Short description of the failure.
        reason: String,
    },

    /// The curve-preserving result disagreed with the flattened polygon oracle.
    #[error("boolean area mismatch: oracle={area_a}, result={area_b}")]
    BooleanMismatch {
        /// Filled area reported by the flattened input oracle.
        area_a: f64,
        /// Filled area of the curve-preserving result.
        area_b: f64,
    },

    /// Two layers cannot be interpolated.
    #[error("layers are incompatible ({count} issues)", count = .0.len())]
    Incompatible(Vec<Incompatibility>),
}

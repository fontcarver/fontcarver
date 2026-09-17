//! Errors exposed by the kernel API.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A transport-safe API error with a stable machine-readable code.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize, TS, thiserror::Error)]
#[error("{message}")]
#[ts(export)]
pub struct ApiError {
    /// Stable error category.
    pub code: String,
    /// Human-readable error detail.
    pub message: String,
}

impl ApiError {
    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::new("not_found", message)
    }

    pub(crate) fn invalid_params(message: impl Into<String>) -> Self {
        Self::new("invalid_params", message)
    }

    pub(crate) fn io(message: impl Into<String>) -> Self {
        Self::new("io", message)
    }

    pub(crate) fn parse(message: impl Into<String>) -> Self {
        Self::new("parse", message)
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }

    pub(crate) fn geometry(message: impl Into<String>) -> Self {
        Self::new("geometry", message)
    }

    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

impl From<fc_doc::Error> for ApiError {
    fn from(error: fc_doc::Error) -> Self {
        let message = error.to_string();
        match error {
            fc_doc::Error::DocumentNotFound(_)
            | fc_doc::Error::MissingGlyph(_)
            | fc_doc::Error::MissingLayer { .. }
            | fc_doc::Error::MissingMaster(_) => Self::not_found(message),
            fc_doc::Error::InvalidEditToken
            | fc_doc::Error::InvalidCommand(_)
            | fc_doc::Error::NonFiniteKerning(_)
            | fc_doc::Error::MetricsKeyCycle
            | fc_doc::Error::Model(_) => Self::invalid_params(message),
            fc_doc::Error::Geometry(_) => Self::geometry(message),
            fc_doc::Error::KerningUnavailable(_) => Self::new("kerning_unavailable", message),
            fc_doc::Error::Io(_) => Self::io(message),
            fc_doc::Error::UnsupportedFormat(_)
            | fc_doc::Error::InvalidFcfFormat(_)
            | fc_doc::Error::UnsupportedFcfVersion(_)
            | fc_doc::Error::SfntUnavailable
            | fc_doc::Error::Sfnt(_)
            | fc_doc::Error::Json(_) => Self::parse(message),
            fc_doc::Error::Internal(_) | fc_doc::Error::LockPoisoned | fc_doc::Error::Reentrant => {
                Self::internal(message)
            }
        }
    }
}

impl From<fc_sfnt::Error> for ApiError {
    fn from(error: fc_sfnt::Error) -> Self {
        match error {
            fc_sfnt::Error::Io { .. } => Self::io(error.to_string()),
            other => Self::parse(other.to_string()),
        }
    }
}

impl From<fc_compile::Error> for ApiError {
    fn from(error: fc_compile::Error) -> Self {
        let message = error.to_string();
        match error {
            fc_compile::Error::MissingMaster(_) => Self::not_found(message),
            fc_compile::Error::NoExportedGlyphs
            | fc_compile::Error::InvalidUnitsPerEm(_)
            | fc_compile::Error::UnsupportedFormat(_) => Self::invalid_params(message),
            fc_compile::Error::Io(_) => Self::io(message),
            _ => Self::new("export", message),
        }
    }
}

impl From<fc_formats::Error> for ApiError {
    fn from(error: fc_formats::Error) -> Self {
        let message = error.to_string();
        match error {
            fc_formats::Error::MissingMaster(_) | fc_formats::Error::MissingSourceMaster(_) => {
                Self::not_found(message)
            }
            fc_formats::Error::Io { .. } => Self::io(message),
            fc_formats::Error::NoAxes
            | fc_formats::Error::InvalidName(_)
            | fc_formats::Error::InvalidIdentifier(_)
            | fc_formats::Error::InvalidAxisTag(_) => Self::invalid_params(message),
            _ => Self::parse(message),
        }
    }
}

impl From<fc_layout::Error> for ApiError {
    fn from(error: fc_layout::Error) -> Self {
        let message = error.to_string();
        match error {
            fc_layout::Error::MissingMaster(_) => Self::not_found(message),
            fc_layout::Error::Compile(error) => Self::from(error),
            fc_layout::Error::Parse(_) => Self::new("shaping", message),
            fc_layout::Error::MissingFont | fc_layout::Error::AbandonedCompile => {
                Self::internal(message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ApiError;

    #[test]
    fn document_errors_map_to_stable_codes() {
        let cases = [
            (
                fc_doc::Error::InvalidCommand("bad index".to_owned()),
                "invalid_params",
            ),
            (fc_doc::Error::MetricsKeyCycle, "invalid_params"),
            (
                fc_doc::Error::Geometry("overlap removal failed".to_owned()),
                "geometry",
            ),
        ];
        for (error, code) in cases {
            let message = error.to_string();
            let mapped = ApiError::from(error);
            assert_eq!(mapped.code, code);
            assert_eq!(mapped.message, message);
        }
    }
}

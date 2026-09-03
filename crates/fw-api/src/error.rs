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

    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

impl From<fw_doc::Error> for ApiError {
    fn from(error: fw_doc::Error) -> Self {
        let message = error.to_string();
        match error {
            fw_doc::Error::DocumentNotFound(_)
            | fw_doc::Error::MissingGlyph(_)
            | fw_doc::Error::MissingLayer { .. }
            | fw_doc::Error::MissingMaster(_) => Self::not_found(message),
            fw_doc::Error::InvalidEditToken
            | fw_doc::Error::InvalidCommand(_)
            | fw_doc::Error::MetricsKeyCycle
            | fw_doc::Error::Model(_) => Self::invalid_params(message),
            fw_doc::Error::Geometry(_) => Self::new("geometry", message),
            fw_doc::Error::Io(_) => Self::io(message),
            fw_doc::Error::UnsupportedFormat(_)
            | fw_doc::Error::InvalidFwfFormat(_)
            | fw_doc::Error::UnsupportedFwfVersion(_)
            | fw_doc::Error::SfntUnavailable
            | fw_doc::Error::Sfnt(_)
            | fw_doc::Error::Json(_) => Self::parse(message),
            fw_doc::Error::Internal(_) | fw_doc::Error::LockPoisoned | fw_doc::Error::Reentrant => {
                Self::internal(message)
            }
        }
    }
}

impl From<fw_sfnt::Error> for ApiError {
    fn from(error: fw_sfnt::Error) -> Self {
        match error {
            fw_sfnt::Error::Io { .. } => Self::io(error.to_string()),
            other => Self::parse(other.to_string()),
        }
    }
}

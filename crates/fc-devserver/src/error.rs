//! Errors produced by the development server.

/// An error produced by the development server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The API rejected a startup request.
    #[error("API error: {0}")]
    Api(#[from] fc_api::ApiError),
    /// A JSON value could not be decoded.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// A network operation failed.
    #[error("network error: {0}")]
    Io(#[from] std::io::Error),
    /// The configured tracing filter was invalid or could not be installed.
    #[error("tracing configuration error: {0}")]
    Tracing(String),
}

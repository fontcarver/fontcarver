/// Hint analysis, program syntax or external hinter failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid or unrepresentable hint data.
    #[error("{0}")]
    Invalid(String),
    /// An instruction cannot be assembled or decoded.
    #[error("instruction {location}: {message}")]
    Program {
        /// One-based source line or zero-based byte offset, identified in the message.
        location: usize,
        /// Explanation of the malformed instruction.
        message: String,
    },
    /// The external TrueType hinter is absent from PATH.
    #[error("ttfautohint is not installed on PATH")]
    Unavailable,
    /// External process failure or timeout.
    #[error("ttfautohint: {0}")]
    External(String),
    /// A temporary file or subprocess could not be accessed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

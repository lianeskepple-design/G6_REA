use std::fmt;

/// Every failure mode is reported with a distinct non-zero exit code so callers
/// can tell "bad arguments" apart from "bad audio file" or "refused to touch it".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// Invalid or missing command line arguments.
    Usage(String),
    /// Container/codec the tool does not accept.
    Unsupported(String),
    /// Container accepted but the payload is invalid (NaN/Inf, bad sizes, ...).
    InvalidInput(String),
    /// A band sits at or above Nyquist and strict mode was requested.
    Nyquist(String),
    /// Output path aliases the input or would overwrite an existing file.
    PathProtected(String),
    /// Filesystem failure.
    Io(String),
}

impl AppError {
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Usage(_) => 2,
            AppError::Unsupported(_) => 3,
            AppError::InvalidInput(_) => 3,
            AppError::Nyquist(_) => 4,
            AppError::PathProtected(_) => 5,
            AppError::Io(_) => 6,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            AppError::Usage(_) => "usage",
            AppError::Unsupported(_) => "unsupported",
            AppError::InvalidInput(_) => "invalid-input",
            AppError::Nyquist(_) => "nyquist",
            AppError::PathProtected(_) => "path-protected",
            AppError::Io(_) => "io",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Usage(msg)
            | AppError::Unsupported(msg)
            | AppError::InvalidInput(msg)
            | AppError::Nyquist(msg)
            | AppError::PathProtected(msg)
            | AppError::Io(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value.to_string())
    }
}

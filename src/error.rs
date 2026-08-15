use std::fmt;

#[derive(Debug)]
pub enum ZzzError {
    Io(std::io::Error),
    InvalidTimeFormat(String),
    TerminalError(String),
}

impl fmt::Display for ZzzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZzzError::Io(err) => write!(f, "I/O error: {}", err),
            ZzzError::InvalidTimeFormat(msg) => write!(f, "Invalid time format: {}", msg),
            ZzzError::TerminalError(msg) => write!(f, "Terminal error: {}", msg),
        }
    }
}

impl std::error::Error for ZzzError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ZzzError::Io(err) => Some(err),
            ZzzError::InvalidTimeFormat(_) => None,
            ZzzError::TerminalError(_) => None,
        }
    }
}

impl From<std::io::Error> for ZzzError {
    fn from(err: std::io::Error) -> Self {
        ZzzError::Io(err)
    }
}

impl From<indicatif::style::TemplateError> for ZzzError {
    fn from(err: indicatif::style::TemplateError) -> Self {
        ZzzError::TerminalError(err.to_string())
    }
}

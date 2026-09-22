use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum GoError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Http {
        url: String,
        source: reqwest::Error,
    },
    Feed {
        line: String,
        source: serde_json::Error,
    },
    Sumdb {
        path: String,
        version: String,
        reason: String,
    },
    GoMod {
        path: String,
        version: String,
        reason: String,
    },
    Cursor {
        source: std::io::Error,
    },
}

impl fmt::Display for GoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GoError::Io { path, source } => {
                write!(f, "{}: io error: {}", path.display(), source)
            }
            GoError::Http { url, source } => {
                write!(f, "{}: http error: {}", url, source)
            }
            GoError::Feed { line, source } => {
                write!(f, "bad feed line: {} ({})", source, line)
            }
            GoError::Sumdb {
                path,
                version,
                reason,
            } => {
                write!(f, "sumdb {}@{}: {}", path, version, reason)
            }
            GoError::GoMod {
                path,
                version,
                reason,
            } => {
                write!(f, "go.mod {}@{}: {}", path, version, reason)
            }
            GoError::Cursor { source } => {
                write!(f, "cursor: {}", source)
            }
        }
    }
}

impl std::error::Error for GoError {}

pub type Result<T> = std::result::Result<T, GoError>;

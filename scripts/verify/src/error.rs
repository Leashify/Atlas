use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum VerifyError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Json {
        path: PathBuf,
        line: usize,
        source: serde_json::Error,
    },
    Shard {
        path: PathBuf,
        expected: PathBuf,
        actual: PathBuf,
    },
    Duplicate {
        path: PathBuf,
        line: usize,
        version: String,
    },
    Hash {
        path: PathBuf,
        line: usize,
        value: String,
    },
    Link {
        path: PathBuf,
        line: usize,
        value: String,
    },
    EmptyVersion {
        path: PathBuf,
        line: usize,
    },
    EmptyFile {
        path: PathBuf,
    },
    Path {
        path: PathBuf,
        reason: String,
    },
    Fatal(String),
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::Io { path, source } => {
                write!(f, "{}: io error: {}", path.display(), source)
            }
            VerifyError::Json { path, line, source } => {
                write!(f, "{}:{}: invalid JSON: {}", path.display(), line, source)
            }
            VerifyError::Shard {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "{}: sharding mismatch, expected {}, got {}",
                    path.display(),
                    expected.display(),
                    actual.display()
                )
            }
            VerifyError::Duplicate {
                path,
                line,
                version,
            } => {
                write!(
                    f,
                    "{}:{}: duplicate version {}",
                    path.display(),
                    line,
                    version
                )
            }
            VerifyError::Hash { path, line, value } => {
                write!(
                    f,
                    "{}:{}: invalid sha256 {:?} (need 64 lowercase hex chars)",
                    path.display(),
                    line,
                    value
                )
            }
            VerifyError::Link { path, line, value } => {
                write!(f, "{}:{}: invalid url {:?}", path.display(), line, value)
            }
            VerifyError::EmptyVersion { path, line } => {
                write!(f, "{}:{}: empty version string", path.display(), line)
            }
            VerifyError::EmptyFile { path } => {
                write!(f, "{}: package file is empty", path.display())
            }
            VerifyError::Path { path, reason } => {
                write!(f, "{}: {}", path.display(), reason)
            }
            VerifyError::Fatal(msg) => write!(f, "fatal: {}", msg),
        }
    }
}

impl std::error::Error for VerifyError {}

pub type Result<T> = std::result::Result<T, VerifyError>;

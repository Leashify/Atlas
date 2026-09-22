use crate::error::{Result, VerifyError};
use std::path::Path;

/// A valid artifact URL must be https, non-empty, and free of whitespace.
pub fn check(path: &Path, line: usize, value: &str) -> Result<()> {
    let ok = value.starts_with("https://")
        && value.len() > "https://".len()
        && !value.contains(' ')
        && !value.contains('\t')
        && !value.contains('\n');
    if !ok {
        return Err(VerifyError::Link {
            path: path.to_path_buf(),
            line,
            value: value.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https() {
        check(Path::new("t"), 1, "https://example.com/x.tar.gz").unwrap();
    }

    #[test]
    fn rejects_http() {
        assert!(check(Path::new("t"), 1, "http://example.com/x").is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(check(Path::new("t"), 1, "").is_err());
    }

    #[test]
    fn rejects_bare_scheme() {
        assert!(check(Path::new("t"), 1, "https://").is_err());
    }

    #[test]
    fn rejects_space() {
        assert!(check(Path::new("t"), 1, "https://example.com/a b").is_err());
    }

    #[test]
    fn rejects_tab() {
        assert!(check(Path::new("t"), 1, "https://example.com/a\tb").is_err());
    }
}

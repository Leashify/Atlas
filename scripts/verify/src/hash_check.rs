use crate::error::{Result, VerifyError};
use std::path::Path;

/// A valid SHA-256 is exactly 64 lowercase hex characters.
pub fn check(path: &Path, line: usize, value: &str) -> Result<()> {
    let ok = value.len() == 64
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    if !ok {
        return Err(VerifyError::Hash {
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
    fn accepts_valid() {
        let h = "a".repeat(64);
        check(Path::new("t"), 1, &h).unwrap();
    }

    #[test]
    fn accepts_digits() {
        let h = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcd".to_string();
        check(Path::new("t"), 1, &h).unwrap();
    }

    #[test]
    fn rejects_short() {
        let h = "a".repeat(63);
        assert!(check(Path::new("t"), 1, &h).is_err());
    }

    #[test]
    fn rejects_long() {
        let h = "a".repeat(65);
        assert!(check(Path::new("t"), 1, &h).is_err());
    }

    #[test]
    fn rejects_uppercase() {
        let h = "A".repeat(64);
        assert!(check(Path::new("t"), 1, &h).is_err());
    }

    #[test]
    fn rejects_non_hex() {
        let h = "g".repeat(64);
        assert!(check(Path::new("t"), 1, &h).is_err());
    }
}

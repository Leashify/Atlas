use crate::error::{Result, VerifyError};
use atlas_common::VersionEntry;
use std::path::Path;

/// Parse one line as a `VersionEntry`. Returns the parsed entry or an
/// error with the line number.
pub fn parse_line(path: &Path, line_no: usize, line: &str) -> Result<VersionEntry> {
    let entry: VersionEntry = serde_json::from_str(line).map_err(|e| VerifyError::Json {
        path: path.to_path_buf(),
        line: line_no,
        source: e,
    })?;

    if entry.v.is_empty() {
        return Err(VerifyError::EmptyVersion {
            path: path.to_path_buf(),
            line: line_no,
        });
    }

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid() {
        let line = r#"{"v":"1.0.0","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","url":"https://example.com/x.tgz","deps":{},"yanked":false,"published":"2024-01-01T00:00:00Z"}"#;
        let e = parse_line(Path::new("t"), 1, line).unwrap();
        assert_eq!(e.v, "1.0.0");
    }

    #[test]
    fn rejects_bad_json() {
        assert!(parse_line(Path::new("t"), 1, "{not json").is_err());
    }

    #[test]
    fn rejects_empty_version() {
        let line = r#"{"v":"","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","url":"https://example.com/x","deps":{},"yanked":false,"published":""}"#;
        assert!(parse_line(Path::new("t"), 1, line).is_err());
    }

    #[test]
    fn accepts_missing_published() {
        let line = r#"{"v":"1.0.0","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","url":"https://example.com/x","deps":{},"yanked":false}"#;
        let e = parse_line(Path::new("t"), 1, line).unwrap();
        assert_eq!(e.published, "");
    }
}

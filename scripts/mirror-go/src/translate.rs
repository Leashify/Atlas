use atlas_common::VersionEntry;
use std::collections::HashMap;

/// Parse a go.mod file and return its `require` entries as
/// `module path -> version`.
///
/// Handles both:
///   require github.com/a/b v1.0.0
///   require (
///       github.com/a/b v1.0.0
///       github.com/c/d v2.0.0 // indirect
///   )
///
/// Comments and the `// indirect` marker are stripped.
pub fn parse_requires(content: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut in_block = false;

    for raw in content.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }

        if in_block {
            if line == ")" {
                in_block = false;
                continue;
            }
            if let Some((path, version)) = parse_require_line(line) {
                out.insert(path, version);
            }
            continue;
        }

        if line == "require (" {
            in_block = true;
            continue;
        }

        if let Some(rest) = line.strip_prefix("require ") {
            if let Some((path, version)) = parse_require_line(rest) {
                out.insert(path, version);
            }
        }
    }

    out
}

fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn parse_require_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split_whitespace();
    let path = parts.next()?;
    let version = parts.next()?;
    if path.is_empty() || version.is_empty() {
        return None;
    }
    Some((path.to_string(), version.to_string()))
}

/// Build a `VersionEntry` from the pieces fetched for one version.
pub fn build_entry(
    version: &str,
    sha256: String,
    url: String,
    deps: HashMap<String, String>,
    published: String,
) -> VersionEntry {
    VersionEntry {
        v: version.to_string(),
        sha256,
        url,
        deps,
        yanked: false,
        published,
        size: None,
        sig: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_require() {
        let src = "module foo\n\nrequire github.com/a/b v1.0.0\n";
        let deps = parse_requires(src);
        assert_eq!(deps.get("github.com/a/b"), Some(&"v1.0.0".to_string()));
    }

    #[test]
    fn parses_block() {
        let src = "module foo\n\nrequire (\n    github.com/a/b v1.0.0\n    github.com/c/d v2.0.0\n)\n";
        let deps = parse_requires(src);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps.get("github.com/a/b"), Some(&"v1.0.0".to_string()));
        assert_eq!(deps.get("github.com/c/d"), Some(&"v2.0.0".to_string()));
    }

    #[test]
    fn strips_indirect_comment() {
        let src = "require (\n    github.com/a/b v1.0.0 // indirect\n)\n";
        let deps = parse_requires(src);
        assert_eq!(deps.get("github.com/a/b"), Some(&"v1.0.0".to_string()));
    }

    #[test]
    fn ignores_non_require_lines() {
        let src = "module foo\n\ngo 1.18\n\ntoolchain go1.21.0\n";
        let deps = parse_requires(src);
        assert!(deps.is_empty());
    }
}

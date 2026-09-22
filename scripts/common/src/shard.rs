use std::path::PathBuf;

/// Compute the sharded path for a package name in a given ecosystem.
///
/// Rules:
/// - npm: `@scope/name` → `npm/@scope/name`
///        otherwise by length (1, 2, 3, then first2/next2)
/// - cargo/pypi: by length (1, 2, 3, then first2/next2)
/// - go: full URL path preserved
/// - github: `org/repo` → `github/<first2>/<next2>/<org>/<repo>`
pub fn shard_path(ecosystem: &str, name: &str) -> PathBuf {
    match ecosystem {
        "npm" => npm_path(name),
        "cargo" => length_path("cargo", name),
        "pypi" => length_path("pypi", &normalize_pypi(name)),
        "go" => PathBuf::from(format!("go/{}", name)),
        "github" => github_path(name),
        other => panic!("unknown ecosystem: {}", other),
    }
}

fn npm_path(name: &str) -> PathBuf {
    if let Some(rest) = name.strip_prefix('@') {
        // scoped: @scope/name → npm/@scope/name
        PathBuf::from(format!("npm/@{}", rest))
    } else {
        length_path("npm", name)
    }
}

fn length_path(eco: &str, name: &str) -> PathBuf {
    match name.len() {
        0 => panic!("empty name"),
        1 => PathBuf::from(format!("{}/1/{}", eco, name)),
        2 => PathBuf::from(format!("{}/2/{}", eco, name)),
        3 => PathBuf::from(format!("{}/3/{}/{}", eco, &name[..1], name)),
        _ => {
            let chars: Vec<char> = name.chars().collect();
            let first2: String = chars.iter().take(2).collect();
            let next2: String = chars.iter().skip(2).take(2).collect();
            PathBuf::from(format!("{}/{}/{}/{}", eco, first2, next2, name))
        }
    }
}

fn github_path(name: &str) -> PathBuf {
    // org/repo
    let parts: Vec<&str> = name.splitn(2, '/').collect();
    if parts.len() != 2 {
        panic!("github name must be org/repo, got: {}", name);
    }
    let org = parts[0];
    let repo = parts[1];
    let chars: Vec<char> = org.chars().collect();
    let first2: String = chars.iter().take(2).collect();
    let next2: String = chars.iter().skip(2).take(2).collect();
    PathBuf::from(format!("github/{}/{}/{}/{}", first2, next2, org, repo))
}

/// PyPI name normalization (PEP 503).
/// Lowercase, replace runs of `-_.` with a single `-`.
pub fn normalize_pypi(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = false;
    for c in name.chars() {
        if c == '-' || c == '_' || c == '.' {
            if !last_dash {
                out.push('-');
                last_dash = true;
            }
        } else {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npm_shards() {
        assert_eq!(shard_path("npm", "q"), PathBuf::from("npm/1/q"));
        assert_eq!(shard_path("npm", "fs"), PathBuf::from("npm/2/fs"));
        assert_eq!(shard_path("npm", "abs"), PathBuf::from("npm/3/a/abs"));
        assert_eq!(shard_path("npm", "react"), PathBuf::from("npm/re/act/react"));
        assert_eq!(shard_path("npm", "lodash"), PathBuf::from("npm/lo/da/lodash"));
        assert_eq!(shard_path("npm", "@types/node"), PathBuf::from("npm/@types/node"));
    }

    #[test]
    fn cargo_shards() {
        assert_eq!(shard_path("cargo", "serde"), PathBuf::from("cargo/se/rd/serde"));
        assert_eq!(shard_path("cargo", "tokio"), PathBuf::from("cargo/to/ki/tokio"));
    }

    #[test]
    fn pypi_normalize() {
        assert_eq!(normalize_pypi("Pillow"), "pillow");
        assert_eq!(normalize_pypi("scikit-learn"), "scikit-learn");
        assert_eq!(normalize_pypi("zope.interface"), "zope-interface");
        assert_eq!(normalize_pypi("Foo__Bar"), "foo-bar");
    }

    #[test]
    fn github_shards() {
        assert_eq!(
            shard_path("github", "leashify/leash"),
            PathBuf::from("github/le/as/leashify/leash")
        );
    }
  }

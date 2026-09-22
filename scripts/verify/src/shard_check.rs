use crate::error::{Result, VerifyError};
use atlas_common::shard_path;
use std::path::{Path, PathBuf};

/// Verify that `rel` matches the sharded location for package `name`
/// in ecosystem `eco`.
pub fn check(eco: &str, rel: &Path, name: &str) -> Result<()> {
    let expected: PathBuf = shard_path(eco, name);
    if expected != rel {
        return Err(VerifyError::Shard {
            path: rel.to_path_buf(),
            expected,
            actual: rel.to_path_buf(),
        });
    }
    Ok(())
}

/// Reverse a sharded path back to the package name.
pub fn reverse(eco: &str, rel: &Path) -> Result<String> {
    let components: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if components.is_empty() {
        return Err(VerifyError::Path {
            path: rel.to_path_buf(),
            reason: "empty path".into(),
        });
    }

    if components[0] != eco {
        return Err(VerifyError::Path {
            path: rel.to_path_buf(),
            reason: format!("first component must be {:?}, got {:?}", eco, components[0]),
        });
    }

    let rest = &components[1..];

    match eco {
        "go" => {
            if rest.len() < 2 {
                return Err(VerifyError::Path {
                    path: rel.to_path_buf(),
                    reason: format!("go path too short: {:?}", rel),
                });
            }
            Ok(rest.join("/"))
        }
        "github" => {
            if rest.len() < 4 {
                return Err(VerifyError::Path {
                    path: rel.to_path_buf(),
                    reason: format!("github path too short: {:?}", rel),
                });
            }
            let n = rest.len();
            Ok(format!("{}/{}", rest[n - 2], rest[n - 1]))
        }
        "npm" => {
            if rest.is_empty() {
                return Err(VerifyError::Path {
                    path: rel.to_path_buf(),
                    reason: "empty npm path".into(),
                });
            }
            if rest[0].starts_with('@') {
                if rest.len() != 2 {
                    return Err(VerifyError::Path {
                        path: rel.to_path_buf(),
                        reason: format!(
                            "scoped npm path needs 2 components, got {}",
                            rest.len()
                        ),
                    });
                }
                Ok(format!("{}/{}", rest[0], rest[1]))
            } else {
                rest.last().cloned().ok_or_else(|| VerifyError::Path {
                    path: rel.to_path_buf(),
                    reason: "empty npm name".into(),
                })
            }
        }
        "cargo" | "pypi" => rest.last().cloned().ok_or_else(|| VerifyError::Path {
            path: rel.to_path_buf(),
            reason: format!("empty {} name", eco),
        }),
        other => Err(VerifyError::Path {
            path: rel.to_path_buf(),
            reason: format!("unknown ecosystem: {}", other),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverses_npm() {
        assert_eq!(
            reverse("npm", Path::new("npm/re/act/react")).unwrap(),
            "react"
        );
        assert_eq!(
            reverse("npm", Path::new("npm/@types/node")).unwrap(),
            "@types/node"
        );
    }

    #[test]
    fn reverses_cargo() {
        assert_eq!(
            reverse("cargo", Path::new("cargo/se/rd/serde")).unwrap(),
            "serde"
        );
    }

    #[test]
    fn reverses_go() {
        assert_eq!(
            reverse("go", Path::new("go/github.com/gin-gonic/gin")).unwrap(),
            "github.com/gin-gonic/gin"
        );
    }

    #[test]
    fn reverses_github() {
        assert_eq!(
            reverse("github", Path::new("github/le/as/leashify/leash")).unwrap(),
            "leashify/leash"
        );
    }
}

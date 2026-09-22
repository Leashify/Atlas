use crate::error::{Result, VerifyError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Tracks versions seen so far for a package file and errors on duplicates.
pub struct DuplicateChecker {
    path: PathBuf,
    seen: HashSet<String>,
}

impl DuplicateChecker {
    pub fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            seen: HashSet::new(),
        }
    }

    /// Record a version at the given line. Returns an error if it was
    /// already seen.
    pub fn insert(&mut self, version: &str, line: usize) -> Result<()> {
        if !self.seen.insert(version.to_string()) {
            return Err(VerifyError::Duplicate {
                path: self.path.clone(),
                line,
                version: version.to_string(),
            });
        }
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_unique() {
        let mut c = DuplicateChecker::new(Path::new("test"));
        c.insert("1.0.0", 1).unwrap();
        c.insert("1.0.1", 2).unwrap();
        assert_eq!(c.count(), 2);
    }

    #[test]
    fn rejects_duplicate() {
        let mut c = DuplicateChecker::new(Path::new("test"));
        c.insert("1.0.0", 1).unwrap();
        let err = c.insert("1.0.0", 2).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }
}

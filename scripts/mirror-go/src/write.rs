use atlas_common::VersionEntry;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Append new versions to a package file. Existing versions stay in
/// place. Returns the number of versions added.
pub fn append_versions(path: &Path, entries: &[VersionEntry]) -> Result<usize, std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = fs::read_to_string(path).unwrap_or_default();

    let existing_versions: HashSet<String> = existing
        .lines()
        .filter_map(|line| serde_json::from_str::<VersionEntry>(line).ok())
        .map(|e| e.v)
        .collect();

    let mut output = existing;
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    let mut added = 0;
    for entry in entries {
        if existing_versions.contains(&entry.v) {
            continue;
        }
        let line = serde_json::to_string(entry).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        output.push_str(&line);
        output.push('\n');
        added += 1;
    }

    if added > 0 {
        fs::write(path, output)?;
    }

    Ok(added)
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Version string
    pub v: String,

    /// SHA-256 of the artifact
    pub sha256: String,

    /// Download URL
    pub url: String,

    /// Dependency map: name -> native constraint
    #[serde(default)]
    pub deps: HashMap<String, String>,

    /// Withdrawn by publisher
    #[serde(default)]
    pub yanked: bool,

    /// ISO 8601 timestamp. Empty string means the ecosystem does not
    /// expose a publish date (crates.io). Consumers must handle empty.
    #[serde(default)]
    pub published: String,

    /// Artifact size, optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Publisher signature, optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<String>,
}

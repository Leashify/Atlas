use crate::config::{INDEX_URL, PAGE_LIMIT, PROXY_URL, SUMDB_URL};
use crate::error::{GoError, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(rename = "Path")]
    pub path: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Timestamp")]
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct Info {
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Time")]
    pub time: String,
}

/// Fetch a page of events from the Go module index. If `since` is set,
/// only events after that timestamp are returned.
pub async fn fetch_events(
    client: &reqwest::Client,
    since: Option<&str>,
) -> Result<Vec<Event>> {
    let mut url = format!("{}?limit={}", INDEX_URL, PAGE_LIMIT);
    if let Some(ts) = since {
        url.push_str("&since=");
        url.push_str(ts);
    }

    let body = client
        .get(&url)
        .send()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .error_for_status()
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .text()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?;

    let mut events = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let event: Event = serde_json::from_str(line).map_err(|e| GoError::Feed {
            line: line.to_string(),
            source: e,
        })?;
        events.push(event);
    }

    Ok(events)
}

/// Fetch version metadata for a module.
pub async fn fetch_info(
    client: &reqwest::Client,
    path: &str,
    version: &str,
) -> Result<Info> {
    let url = format!(
        "{}/{}/@v/{}.info",
        PROXY_URL,
        escape_path(path),
        version
    );
    let info: Info = client
        .get(&url)
        .send()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .error_for_status()
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .json()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?;
    Ok(info)
}

/// Fetch the go.mod file for a module version.
pub async fn fetch_mod(
    client: &reqwest::Client,
    path: &str,
    version: &str,
) -> Result<String> {
    let url = format!(
        "{}/{}/@v/{}.mod",
        PROXY_URL,
        escape_path(path),
        version
    );
    let body = client
        .get(&url)
        .send()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .error_for_status()
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .text()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?;
    Ok(body)
}

/// Look up the canonical SHA-256 of a module version from sum.golang.org.
/// Returns a 64-character lowercase hex string.
pub async fn lookup_hash(
    client: &reqwest::Client,
    path: &str,
    version: &str,
) -> Result<String> {
    let url = format!("{}/lookup/{}@{}", SUMDB_URL, path, version);
    let body = client
        .get(&url)
        .send()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .error_for_status()
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?
        .text()
        .await
        .map_err(|e| GoError::Http {
            url: url.clone(),
            source: e,
        })?;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 3 {
            continue;
        }
        if parts[0] != path || parts[1] != version {
            continue;
        }
        return decode_h1(parts[2]).map_err(|reason| GoError::Sumdb {
            path: path.to_string(),
            version: version.to_string(),
            reason,
        });
    }

    Err(GoError::Sumdb {
        path: path.to_string(),
        version: version.to_string(),
        reason: "no hash found in sumdb response".into(),
    })
}

fn decode_h1(h1: &str) -> std::result::Result<String, String> {
    let b64 = h1
        .strip_prefix("h1:")
        .ok_or_else(|| format!("missing h1: prefix in {:?}", h1))?;
    let bytes = STANDARD
        .decode(b64)
        .map_err(|e| format!("invalid base64 {:?}: {}", b64, e))?;
    if bytes.len() != 32 {
        return Err(format!(
            "expected 32 bytes from h1, got {} in {:?}",
            bytes.len(),
            h1
        ));
    }
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

/// Go's proxy protocol escapes uppercase letters as `!` + lowercase.
pub fn escape_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if c.is_ascii_uppercase() {
            out.push('!');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

pub fn zip_url(path: &str, version: &str) -> String {
    format!(
        "{}/{}/@v/{}.zip",
        PROXY_URL,
        escape_path(path),
        version
    )
}

/// Read the cursor file, if present.
pub fn read_cursor() -> Option<String> {
    std::fs::read_to_string(crate::config::CURSOR_FILE)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Write the cursor file.
pub fn write_cursor(ts: &str) -> Result<()> {
    let path = PathBuf::from(crate::config::CURSOR_FILE);
    std::fs::write(&path, ts).map_err(|source| GoError::Io { path, source })
}

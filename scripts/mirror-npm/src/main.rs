use anyhow::{Context, Result};
use atlas_common::{shard_path, VersionEntry};
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

const NPM_REGISTRY: &str = "https://registry.npmjs.org";
const CONCURRENCY: usize = 20;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let names = load_package_names().await?;
    info!("mirroring {} npm packages", names.len());

    let client = reqwest::Client::builder()
        .user_agent("atlas-mirror-npm/0.9.1")
        .build()?;

    let stats = Arc::new(Mutex::new(Stats::default()));

    stream::iter(names)
        .map(|name| {
            let client = client.clone();
            let stats = stats.clone();
            async move {
                match mirror_one(&client, &name).await {
                    Ok(count) => {
                        let mut s = stats.lock().await;
                        s.packages += 1;
                        s.versions += count;
                    }
                    Err(e) => {
                        warn!("failed {}: {}", name, e);
                        let mut s = stats.lock().await;
                        s.failed += 1;
                    }
                }
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<()>>()
        .await;

    let s = stats.lock().await;
    info!(
        "done: {} packages, {} versions, {} failed",
        s.packages, s.versions, s.failed
    );
    Ok(())
}

#[derive(Default)]
struct Stats {
    packages: usize,
    versions: usize,
    failed: usize,
}

async fn load_package_names() -> Result<Vec<String>> {
    let path = Path::new(".cursor");
    if path.exists() {
        let cursor = fs::read_to_string(path)?;
        let trimmed = cursor.trim();
        if !trimmed.is_empty() {
            info!("resuming from cursor: {}", trimmed);
            return fetch_from_cursor(trimmed).await;
        }
    }
    fetch_from_cursor("").await
}

async fn fetch_from_cursor(start: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut cursor = start.to_string();

    loop {
        let url = if cursor.is_empty() {
            format!("{}?limit=1000", "https://replicate.npmjs.com/_all_docs")
        } else {
            format!(
                "https://replicate.npmjs.com/_all_docs?startkey=\"{}\"&skip=1&limit=1000",
                cursor
            )
        };

        let resp: serde_json::Value = reqwest::get(&url)
            .await
            .context("fetch package list")?
            .json()
            .await?;

        let rows = match resp["rows"].as_array() {
            Some(r) if !r.is_empty() => r.clone(),
            _ => break,
        };

        for row in &rows {
            if let Some(id) = row["id"].as_str() {
                if !id.starts_with('_') {
                    names.push(id.to_string());
                }
            }
        }

        cursor = rows
            .last()
            .and_then(|r| r["id"].as_str())
            .unwrap_or("")
            .to_string();

        fs::write(".cursor", &cursor)?;

        if rows.len() < 1000 {
            break;
        }
    }

    Ok(names)
}

async fn mirror_one(client: &reqwest::Client, name: &str) -> Result<usize> {
    let url = format!("{}/{}", NPM_REGISTRY, name);
    let meta: serde_json::Value = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let versions = match meta["versions"].as_object() {
        Some(v) => v,
        None => return Ok(0),
    };

    let mut entries: Vec<VersionEntry> = Vec::new();

    for (version, info) in versions {
        let dist = &info["dist"];
        let tarball = dist["tarball"].as_str().unwrap_or("").to_string();
        if tarball.is_empty() {
            continue;
        }

        let sha256 = dist["integrity"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha512-"))
            .map(|_| dist["shasum"].as_str().unwrap_or("").to_string())
            .unwrap_or_else(|| dist["shasum"].as_str().unwrap_or("").to_string());

        // npm's shasum is SHA-1, not SHA-256. For now we record what npm gives.
        // A later pass will download the tarball and compute real SHA-256.
        if sha256.len() != 64 {
            continue; // skip versions without a real SHA-256
        }

        let mut deps = HashMap::new();
        if let Some(d) = info["dependencies"].as_object() {
            for (k, v) in d {
                if let Some(s) = v.as_str() {
                    deps.insert(k.clone(), s.to_string());
                }
            }
        }

        let published = meta["time"]
            .get(version)
            .and_then(|v| v.as_str())
            .unwrap_or("1970-01-01T00:00:00Z")
            .to_string();

        entries.push(VersionEntry {
            v: version.clone(),
            sha256,
            url: tarball,
            deps,
            yanked: false,
            published,
            size: dist["unpackedSize"].as_u64(),
            sig: None,
        });
    }

    if entries.is_empty() {
        return Ok(0);
    }

    let path = shard_path("npm", name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Merge with existing lines to stay append-only
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let existing_versions: std::collections::HashSet<String> = existing
        .lines()
        .filter_map(|line| {
            serde_json::from_str::<VersionEntry>(line)
                .ok()
                .map(|e| e.v)
        })
        .collect();

    let mut output = existing;
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    let mut added = 0;
    for entry in &entries {
        if existing_versions.contains(&entry.v) {
            continue;
        }
        output.push_str(&serde_json::to_string(entry)?);
        output.push('\n');
        added += 1;
    }

    if added > 0 {
        fs::write(&path, output)?;
    }

    Ok(added)
}

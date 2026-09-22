use anyhow::{Context, Result};
use atlas_common::{shard_path, VersionEntry};
use futures::stream::{self, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

const CRATES_API: &str = "https://crates.io/api/v1/crates";
const CRATES_INDEX: &str = "https://index.crates.io";
const PAGE_SIZE: usize = 100;
const CONCURRENCY: usize = 10;
const API_RATE_DELAY_MS: u64 = 1000;
const USER_AGENT: &str =
    "atlas-mirror-crates/0.1.0 (https://github.com/Leashify/Atlas)";
const CURSOR_FILE: &str = ".crates-cursor";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;

    let start_page = read_cursor().unwrap_or(1);
    info!("starting from page {}", start_page);

    let stats = Arc::new(Mutex::new(Stats::default()));

    let mut page = start_page;
    loop {
        let names = match fetch_page(&client, page).await {
            Ok(n) => n,
            Err(e) => {
                warn!("page {} failed: {}", page, e);
                break;
            }
        };

        if names.is_empty() {
            info!("no more pages, stopping at page {}", page);
            break;
        }

        info!("page {}: {} crates", page, names.len());

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

        write_cursor(page + 1)?;
        page += 1;

        tokio::time::sleep(Duration::from_millis(API_RATE_DELAY_MS)).await;
    }

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

fn read_cursor() -> Option<u64> {
    fs::read_to_string(CURSOR_FILE)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn write_cursor(page: u64) -> Result<()> {
    fs::write(CURSOR_FILE, page.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
struct CrateListPage {
    #[serde(default)]
    crates: Vec<CrateListItem>,
    #[serde(default)]
    meta: PageMeta,
}

#[derive(Deserialize)]
struct CrateListItem {
    name: String,
}

#[derive(Deserialize, Default)]
struct PageMeta {
    #[serde(default)]
    total: u64,
}

async fn fetch_page(client: &reqwest::Client, page: u64) -> Result<Vec<String>> {
    let url = format!(
        "{}?page={}&per_page={}&sort=alpha",
        CRATES_API, page, PAGE_SIZE
    );

    let resp: CrateListPage = client
        .get(&url)
        .send()
        .await
        .context("crates.io API request")?
        .error_for_status()?
        .json()
        .await
        .context("crates.io API JSON")?;

    if page == 1 && resp.meta.total > 0 {
        info!("crates.io reports {} total crates", resp.meta.total);
    }

    Ok(resp.crates.into_iter().map(|c| c.name).collect())
}

#[derive(Deserialize)]
struct CrateVersionLine {
    vers: String,
    #[serde(default)]
    deps: Vec<CrateDepLine>,
    cksum: String,
    #[serde(default)]
    yanked: bool,
}

#[derive(Deserialize)]
struct CrateDepLine {
    name: String,
    req: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    optional: bool,
}

async fn mirror_one(client: &reqwest::Client, name: &str) -> Result<usize> {
    let url = format!("{}/{}", CRATES_INDEX, index_path(name));
    let body = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let mut entries: Vec<VersionEntry> = Vec::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let cv: CrateVersionLine = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                warn!("{}: bad line: {}", name, e);
                continue;
            }
        };

        let mut deps: HashMap<String, String> = HashMap::new();
        for d in &cv.deps {
            // Only normal runtime deps. Skip dev and build.
            let kind_is_normal = d.kind.is_none()
                || d.kind.as_deref() == Some("normal");
            if !kind_is_normal {
                continue;
            }
            // Skip optional deps — they're activated by features.
            if d.optional {
                continue;
            }
            // Skip platform-conditional deps; the resolver handles them later.
            if d.target.is_some() {
                continue;
            }
            // If `package` is set, this is a rename. Use the real crate name.
            let real_name = d.package.clone().unwrap_or_else(|| d.name.clone());
            deps.insert(real_name, d.req.clone());
        }

        let download = format!(
            "https://crates.io/api/v1/crates/{}/{}/download",
            name, cv.vers
        );

        entries.push(VersionEntry {
            v: cv.vers,
            sha256: cv.cksum,
            url: download,
            deps,
            yanked: cv.yanked,
            published: String::new(),
            size: None,
            sig: None,
        });
    }

    if entries.is_empty() {
        return Ok(0);
    }

    let path = shard_path("cargo", name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

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

/// crates.io sparse index path. Same sharding rules as the git index.
fn index_path(name: &str) -> String {
    match name.len() {
        0 => panic!("empty crate name"),
        1 => format!("1/{}", name),
        2 => format!("2/{}", name),
        3 => format!("3/{}/{}", &name[..1], name),
        _ => {
            let chars: Vec<char> = name.chars().collect();
            let first2: String = chars.iter().take(2).collect();
            let next2: String = chars.iter().skip(2).take(2).collect();
            format!("{}/{}/{}", first2, next2, name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_index_paths() {
        assert_eq!(index_path("a"), "1/a");
        assert_eq!(index_path("ab"), "2/ab");
        assert_eq!(index_path("abc"), "3/a/abc");
        assert_eq!(index_path("serde"), "se/rd/serde");
        assert_eq!(index_path("tokio"), "to/ki/tokio");
    }
}

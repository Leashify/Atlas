mod config;
mod error;
mod fetch;
mod shard;
mod translate;
mod write;

use atlas_common::VersionEntry;
use config::{CONCURRENCY, USER_AGENT};
use error::Result;
use futures::stream::{self, StreamExt};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    if let Err(e) = run().await {
        eprintln!("fatal: {}", e);
        std::process::exit(2);
    }
}

async fn run() -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| error::GoError::Http {
            url: "<client>".into(),
            source: e,
        })?;

    let since = fetch::read_cursor();
    if let Some(ref ts) = since {
        info!("resuming from {}", ts);
    }

    let events = fetch::fetch_events(&client, since.as_deref()).await?;
    info!("fetched {} events from index.golang.org", events.len());

    if events.is_empty() {
        info!("no new events, done");
        return Ok(());
    }

    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut latest_ts = String::new();

    for ev in &events {
        by_path
            .entry(ev.path.clone())
            .or_default()
            .push(ev.version.clone());
        if ev.timestamp > latest_ts {
            latest_ts = ev.timestamp.clone();
        }
    }

    info!("{} unique modules to mirror", by_path.len());

    let stats = Arc::new(Mutex::new(Stats::default()));

    stream::iter(by_path)
        .map(|(path, versions)| {
            let client = client.clone();
            let stats = stats.clone();
            async move {
                match mirror_module(&client, &path, &versions).await {
                    Ok((added, failed)) => {
                        let mut s = stats.lock().await;
                        s.modules += 1;
                        s.versions += added;
                        s.failed_versions += failed;
                    }
                    Err(e) => {
                        warn!("module {} failed: {}", path, e);
                        let mut s = stats.lock().await;
                        s.failed_modules += 1;
                    }
                }
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<()>>()
        .await;

    fetch::write_cursor(&latest_ts)?;

    let s = stats.lock().await;
    info!(
        "done: {} modules mirrored, {} versions written, {} versions failed, {} modules failed",
        s.modules, s.versions, s.failed_versions, s.failed_modules
    );

    Ok(())
}

#[derive(Default)]
struct Stats {
    modules: usize,
    versions: usize,
    failed_versions: usize,
    failed_modules: usize,
}

async fn mirror_module(
    client: &reqwest::Client,
    path: &str,
    versions: &[String],
) -> Result<(usize, usize)> {
    let mut entries = Vec::with_capacity(versions.len());
    let mut failed = 0;

    for version in versions {
        match build_entry(client, path, version).await {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                warn!("{}@{}: {}", path, version, e);
                failed += 1;
            }
        }
    }

    if entries.is_empty() {
        return Ok((0, failed));
    }

    let file_path = shard::shard_path(path);
    let added = write::append_versions(&file_path, &entries).map_err(|source| {
        error::GoError::Io {
            path: file_path.clone(),
            source,
        }
    })?;
    Ok((added, failed))
}

async fn build_entry(
    client: &reqwest::Client,
    path: &str,
    version: &str,
) -> Result<VersionEntry> {
    let info = fetch::fetch_info(client, path, version).await?;
    let mod_content = fetch::fetch_mod(client, path, version).await?;
    let deps = translate::parse_requires(&mod_content);
    let sha256 = fetch::lookup_hash(client, path, version).await?;
    let url = fetch::zip_url(path, version);
    Ok(translate::build_entry(
        version,
        sha256,
        url,
        deps,
        info.time,
    ))
}

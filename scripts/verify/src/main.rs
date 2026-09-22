use anyhow::{anyhow, bail, Context, Result};
use atlas_common::{shard_path, VersionEntry};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const ECOSYSTEMS: &[&str] = &["npm", "cargo", "pypi", "go", "github"];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(args.get(1).map(|s| s.as_str()).unwrap_or("."));

    match run(&root) {
        Ok(report) => {
            println!(
                "checked {} package files, {} versions",
                report.files, report.versions
            );
            if report.errors.is_empty() {
                println!("ok");
            } else {
                for err in &report.errors {
                    eprintln!("error: {}", err);
                }
                eprintln!("{} error(s)", report.errors.len());
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("fatal: {:#}", e);
            std::process::exit(2);
        }
    }
}

#[derive(Default)]
struct Report {
    files: usize,
    versions: usize,
    errors: Vec<String>,
}

fn run(root: &Path) -> Result<Report> {
    if !root.exists() {
        bail!("path does not exist: {}", root.display());
    }
    let mut report = Report::default();
    for eco in ECOSYSTEMS {
        let dir = root.join(eco);
        if !dir.exists() {
            continue;
        }
        verify_ecosystem(eco, &dir, root, &mut report);
    }
    Ok(report)
}

fn verify_ecosystem(eco: &str, dir: &Path, root: &Path, report: &mut Report) {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(e) => e,
            Err(e) => {
                report
                    .errors
                    .push(format!("{}: read_dir: {}", current.display(), e));
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    report
                        .errors
                        .push(format!("{}: entry: {}", current.display(), e));
                    continue;
                }
            };
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !is_package_file(name) {
                continue;
            }
            report.files += 1;
            match verify_file(eco, &path, root) {
                Ok(count) => report.versions += count,
                Err(e) => report.errors.push(format!("{}: {:#}", path.display(), e)),
            }
        }
    }
}

fn is_package_file(name: &str) -> bool {
    if name.is_empty() || name.starts_with('.') {
        return false;
    }
    const SKIP_EXT: &[&str] = &[
        ".md", ".json", ".toml", ".yml", ".yaml", ".lock", ".rs", ".txt",
    ];
    !SKIP_EXT.iter().any(|e| name.ends_with(e))
}

fn verify_file(eco: &str, path: &Path, root: &Path) -> Result<usize> {
    let rel = path
        .strip_prefix(root)
        .with_context(|| format!("strip_prefix for {}", path.display()))?;

    let name = reverse_shard(eco, rel)?;
    let expected = shard_path(eco, &name);
    if expected != rel {
        bail!("sharding mismatch: expected {:?}, got {:?}", expected, rel);
    }

    let content = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;

    let mut seen: HashSet<String> = HashSet::new();
    let mut count = 0;

    for (i, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let entry: VersionEntry = serde_json::from_str(line)
            .with_context(|| format!("line {}: invalid JSON", i + 1))?;

        if entry.v.is_empty() {
            bail!("line {}: empty version", i + 1);
        }
        if !seen.insert(entry.v.clone()) {
            bail!("line {}: duplicate version {}", i + 1, entry.v);
        }
        if !valid_sha256(&entry.sha256) {
            bail!(
                "line {}: invalid sha256 {:?} (need 64 lowercase hex chars)",
                i + 1,
                entry.sha256
            );
        }
        if !valid_url(&entry.url) {
            bail!("line {}: invalid url {:?}", i + 1, entry.url);
        }
        count += 1;
    }

    Ok(count)
}

fn reverse_shard(eco: &str, rel: &Path) -> Result<String> {
    let components: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    if components.is_empty() {
        bail!("empty path");
    }
    if components[0] != eco {
        bail!("path does not start with {:?}", eco);
    }
    let rest = &components[1..];

    match eco {
        "go" => {
            if rest.len() < 2 {
                bail!("go path too short: {:?}", rel);
            }
            Ok(rest.join("/"))
        }
        "github" => {
            if rest.len() < 4 {
                bail!("github path too short: {:?}", rel);
            }
            let n = rest.len();
            Ok(format!("{}/{}", rest[n - 2], rest[n - 1]))
        }
        "npm" => {
            if rest.is_empty() {
                bail!("empty npm path");
            }
            if rest[0].starts_with('@') {
                if rest.len() != 2 {
                    bail!("scoped npm path needs 2 components, got {}", rest.len());
                }
                Ok(format!("{}/{}", rest[0], rest[1]))
            } else {
                rest.last().cloned().ok_or_else(|| anyhow!("empty npm name"))
            }
        }
        "cargo" | "pypi" => rest
            .last()
            .cloned()
            .ok_or_else(|| anyhow!("empty {} name", eco)),
        other => bail!("unknown ecosystem: {}", other),
    }
}

fn valid_sha256(s: &str) -> bool {
    s.len() == 64
        && s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

fn valid_url(s: &str) -> bool {
    s.starts_with("https://") && s.len() > 10 && !s.contains(' ') && !s.contains('\t')
}

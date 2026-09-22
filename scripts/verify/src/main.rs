mod dup_check;
mod error;
mod hash_check;
mod link_check;
mod schema_check;
mod shard_check;

use crate::dup_check::DuplicateChecker;
use crate::error::{Result, VerifyError};
use std::fs;
use std::path::{Path, PathBuf};

const ECOSYSTEMS: &[&str] = &["npm", "cargo", "pypi", "go", "github"];

const SKIP_EXT: &[&str] = &[
    ".md", ".json", ".toml", ".yml", ".yaml", ".lock", ".rs", ".txt", ".gitkeep",
];

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
            eprintln!("fatal: {}", e);
            std::process::exit(2);
        }
    }
}

#[derive(Default)]
struct Report {
    files: usize,
    versions: usize,
    errors: Vec<VerifyError>,
}

fn run(root: &Path) -> Result<Report> {
    if !root.exists() {
        return Err(VerifyError::Fatal(format!(
            "path does not exist: {}",
            root.display()
        )));
    }

    let mut report = Report::default();

    for eco in ECOSYSTEMS {
        let dir = root.join(eco);
        if !dir.exists() {
            continue;
        }
        walk(eco, &dir, root, &mut report);
    }

    Ok(report)
}

fn walk(eco: &str, dir: &Path, root: &Path, report: &mut Report) {
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(e) => e,
            Err(e) => {
                report.errors.push(VerifyError::Io {
                    path: current.clone(),
                    source: e,
                });
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    report.errors.push(VerifyError::Io {
                        path: current.clone(),
                        source: e,
                    });
                    continue;
                }
            };

            let path = entry.path();

            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            if !is_package_file(name) {
                continue;
            }

            report.files += 1;

            match verify_file(eco, &path, root) {
                Ok(count) => report.versions += count,
                Err(e) => report.errors.push(e),
            }
        }
    }
}

fn is_package_file(name: &str) -> bool {
    if name.is_empty() || name.starts_with('.') {
        return false;
    }
    !SKIP_EXT.iter().any(|e| name.ends_with(e))
}

fn verify_file(eco: &str, path: &Path, root: &Path) -> Result<usize> {
    let rel = path.strip_prefix(root).map_err(|e| VerifyError::Path {
        path: path.to_path_buf(),
        reason: format!("strip_prefix failed: {}", e),
    })?;

    let name = shard_check::reverse(eco, rel)?;
    shard_check::check(eco, rel, &name)?;

    let content = fs::read_to_string(path).map_err(|e| VerifyError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    if content.trim().is_empty() {
        return Err(VerifyError::EmptyFile {
            path: path.to_path_buf(),
        });
    }

    let mut dup = DuplicateChecker::new(path);

    for (i, raw) in content.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        let entry = schema_check::parse_line(path, line_no, line)?;
        dup.insert(&entry.v, line_no)?;
        hash_check::check(path, line_no, &entry.sha256)?;
        link_check::check(path, line_no, &entry.url)?;
    }

    Ok(dup.count())
}

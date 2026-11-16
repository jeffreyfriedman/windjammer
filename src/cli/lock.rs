// wj lock - write or verify wj.lock (ADR-0010)
use crate::package::lock;
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

fn project(dir: Option<&Path>) -> Result<PathBuf> {
    let dir = dir.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    if !dir.join("wj.toml").is_file() {
        bail!("no wj.toml in {}: run `wj lock` in a project directory", dir.display());
    }
    Ok(dir)
}

/// `wj lock update`: resolve the whole graph, dependencies first, and write wj.lock.
pub fn update(dir: Option<&Path>) -> Result<()> {
    let root = project(dir)?;
    let before = lock::read_lock(&root);
    lock::update(&root).map_err(anyhow::Error::msg)?;
    let after = lock::read_lock(&root);
    if before == after {
        println!("{} is up to date ({} package(s))", lock::LOCKFILE, after.packages.len());
    } else {
        println!("wrote {} ({} package(s), dependencies first)", lock::LOCKFILE, after.packages.len());
        for (name, e) in &after.packages {
            println!("  {name}  {}{}", e.source, if e.rev.is_empty() { String::new() } else { format!(" @ {}", &e.rev[..e.rev.len().min(12)]) });
        }
    }
    Ok(())
}

/// `wj lock check`: exit 1 when wj.lock is missing or differs from what the manifests and dependency files give now.
pub fn check(dir: Option<&Path>) -> Result<()> {
    let root = project(dir)?;
    match lock::enforce(&root, true) {
        Ok(_) => {
            println!("{} matches the dependencies", lock::LOCKFILE);
            Ok(())
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

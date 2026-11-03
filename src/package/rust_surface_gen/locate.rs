//! Where a Rust crate dependency's source is, and which version: the version `Cargo.lock` locks when there is
//! one, otherwise the newest version in the local Cargo registry (or `vendor/`) that `wj.toml`'s requirement
//! accepts. Only [`resolve_with_cargo`] downloads, and only the explicit command and `wj build` call it
//! (`wj check` never downloads, ADR-0026).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A crate's unpacked source.
#[derive(Debug, Clone)]
pub struct CrateSource {
    /// Path spelling (`-` written `_`): the `crate` of `crate::f`.
    pub name: String,
    /// The package name Cargo knows (`package = ` rename or the `wj.toml` key).
    pub package: String,
    pub version: String,
    pub dir: PathBuf,
    /// The library root relative to `dir` (`src/lib.rs` unless `[lib] path` says otherwise).
    pub lib: String,
    /// Features enabled for it: the crate's defaults (unless disabled) and `wj.toml`'s, expanded.
    pub features: BTreeSet<String>,
}

/// The `wj.toml` entry of a Rust crate dependency.
#[derive(Debug, Clone, Default)]
pub struct DepEntry {
    pub key: String,
    pub package: String,
    pub req: Option<String>,
    pub path: Option<PathBuf>,
    pub features: Vec<String>,
    pub default_features: bool,
}

/// The entry for `crate_name` (path spelling) in the manifest at `manifest_dir`.
pub fn dep_entry(manifest_dir: &Path, crate_name: &str) -> Option<DepEntry> {
    let doc: toml::Value = std::fs::read_to_string(manifest_dir.join("wj.toml")).ok()?.parse().ok()?;
    let (key, spec) = doc.get("dependencies")?.as_table()?.iter().find(|(k, _)| k.replace('-', "_") == crate_name)?;
    let mut e = DepEntry { key: key.clone(), package: key.clone(), default_features: true, ..Default::default() };
    match spec {
        toml::Value::String(v) => e.req = Some(v.clone()),
        toml::Value::Table(t) => {
            e.req = t.get("version").and_then(|v| v.as_str()).map(str::to_string);
            e.path = t.get("path").and_then(|v| v.as_str()).map(|p| manifest_dir.join(p));
            e.package = t.get("package").and_then(|v| v.as_str()).unwrap_or(key).to_string();
            e.features = t.get("features").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            e.default_features = t.get("default-features").and_then(|v| v.as_bool()).unwrap_or(true);
        }
        _ => return None,
    }
    Some(e)
}

/// The lock files that may record the project's crate versions, most specific first: the build output's
/// (`extra`), then `build/Cargo.lock` (the default output) and `Cargo.lock` next to `wj.toml`.
pub fn lock_files(manifest_dir: &Path, extra: Option<&Path>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = extra.map(|d| d.join("Cargo.lock")).into_iter().collect();
    v.push(manifest_dir.join("build").join("Cargo.lock"));
    v.push(manifest_dir.join("Cargo.lock"));
    v.into_iter().filter(|p| p.is_file()).collect()
}

/// The version of `package` the first lock file that has it records (one that `req` accepts, when several).
pub fn locked_version(locks: &[PathBuf], package: &str, req: Option<&str>) -> Option<String> {
    let req = req.and_then(|r| semver::VersionReq::parse(r).ok());
    for lock in locks {
        let Ok(doc) = std::fs::read_to_string(lock).unwrap_or_default().parse::<toml::Value>() else { continue };
        let Some(pkgs) = doc.get("package").and_then(|p| p.as_array()) else { continue };
        let mut versions: Vec<semver::Version> = pkgs
            .iter()
            .filter(|p| p.get("name").and_then(|n| n.as_str()) == Some(package))
            .filter_map(|p| semver::Version::parse(p.get("version")?.as_str()?).ok())
            .filter(|v| req.as_ref().is_none_or(|r| r.matches(v)))
            .collect();
        versions.sort();
        if let Some(v) = versions.pop() {
            return Some(v.to_string());
        }
    }
    None
}

/// Unpacked registry sources (`$CARGO_HOME/registry/src/*`), sorted.
fn registry_roots() -> Vec<PathBuf> {
    let home = std::env::var_os("CARGO_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")));
    let Some(src) = home.map(|h| h.join("registry").join("src")) else { return Vec::new() };
    let mut v: Vec<PathBuf> = std::fs::read_dir(src).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// Find the source of `entry` locally: a `path` dependency, `vendor/<package>`, or the registry. `exact` pins
/// the version (from a lock file); otherwise the newest version the requirement accepts.
pub fn find_local(manifest_dir: &Path, entry: &DepEntry, exact: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = &entry.path {
        return p.join("Cargo.toml").is_file().then(|| p.clone());
    }
    let req = entry.req.as_deref().and_then(|r| semver::VersionReq::parse(r).ok());
    let accepts = |v: &semver::Version| match exact {
        Some(x) => v.to_string() == x,
        None => req.as_ref().is_none_or(|r| r.matches(v)),
    };
    let vendor = manifest_dir.join("vendor").join(&entry.package);
    if let Some(v) = manifest_version(&vendor) {
        if semver::Version::parse(&v).is_ok_and(|v| accepts(&v)) {
            return Some(vendor);
        }
    }
    let prefix = format!("{}-", entry.package);
    let mut best: Option<(semver::Version, PathBuf)> = None;
    for root in registry_roots() {
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let Some(v) = name.strip_prefix(&prefix).and_then(|v| semver::Version::parse(v).ok()) else { continue };
            if accepts(&v) && e.path().join("Cargo.toml").is_file() && best.as_ref().is_none_or(|(b, _)| v > *b) {
                best = Some((v, e.path()));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Ask Cargo (which may download) where `entry`'s source is. Used only when nothing is found locally.
pub fn resolve_with_cargo(scratch: &Path, entry: &DepEntry) -> Result<PathBuf, String> {
    let spec = match (&entry.path, &entry.req) {
        (Some(p), _) => format!("{{ path = {:?} }}", p.display().to_string()),
        (None, Some(r)) => format!("{{ package = {:?}, version = {:?} }}", entry.package, r),
        (None, None) => format!("{{ package = {:?}, version = \"*\" }}", entry.package),
    };
    std::fs::create_dir_all(scratch.join("src")).map_err(|e| e.to_string())?;
    std::fs::write(scratch.join("src/lib.rs"), "").map_err(|e| e.to_string())?;
    let manifest = format!("[package]\nname = \"wj-rust-surface-resolve\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[dependencies]\n{} = {spec}\n", entry.key);
    std::fs::write(scratch.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
    let out = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(scratch.join("Cargo.toml"))
        .output()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !out.status.success() {
        return Err(format!("cargo could not resolve `{}`: {}", entry.package, String::from_utf8_lossy(&out.stderr).trim()));
    }
    let meta: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    meta["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| p["name"] == entry.package.as_str())
        .and_then(|p| Path::new(p["manifest_path"].as_str()?).parent().map(Path::to_path_buf))
        .ok_or_else(|| format!("cargo did not report a package named `{}`", entry.package))
}

fn manifest_version(dir: &Path) -> Option<String> {
    let doc: toml::Value = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?.parse().ok()?;
    Some(doc.get("package")?.get("version")?.as_str()?.to_string())
}

/// Describe the crate at `dir` for `entry`: version, library root and enabled features.
pub fn describe(dir: &Path, crate_name: &str, entry: &DepEntry) -> Result<CrateSource, String> {
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).map_err(|e| format!("{}: {e}", dir.display()))?;
    let doc: toml::Value = text.parse().map_err(|e| format!("{}/Cargo.toml: {e}", dir.display()))?;
    let version = doc.get("package").and_then(|p| p.get("version")).and_then(|v| v.as_str()).ok_or("Cargo.toml has no package version")?;
    let lib = doc.get("lib").and_then(|l| l.get("path")).and_then(|p| p.as_str()).unwrap_or("src/lib.rs").to_string();
    let table = doc.get("features").and_then(|f| f.as_table()).cloned().unwrap_or_default();
    let mut todo: Vec<String> = entry.features.clone();
    if entry.default_features {
        todo.push("default".into());
    }
    let mut features = BTreeSet::new();
    while let Some(f) = todo.pop() {
        if !features.insert(f.clone()) {
            continue;
        }
        for sub in table.get(&f).and_then(|v| v.as_array()).into_iter().flatten().filter_map(|x| x.as_str()) {
            // `dep:x` and `x/feat` enable dependencies, not features of this crate
            if !sub.contains(':') && !sub.contains('/') {
                todo.push(sub.to_string());
            }
        }
    }
    Ok(CrateSource { name: crate_name.to_string(), package: entry.package.clone(), version: version.to_string(), dir: dir.to_path_buf(), lib, features })
}

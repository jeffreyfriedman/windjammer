//! `wj.lock` (ADR-0010, option A): the exact packages a project builds against.
//!
//! One entry per package of the whole graph (the project's dependencies and theirs), listed dependencies first. A path
//! dependency records the SHA-256 of its files; a git dependency records the commit and the hash of its tree. The
//! registry is deferred (ADR-0010), so version-only (Cargo) dependencies are not recorded: `Cargo.lock` owns those.
//!
//! * `wj lock update` re-resolves everything (floating git dependencies move to their branch head) and writes the file;
//! * `wj lock check` and `wj build --locked` fail when the graph no longer matches the file;
//! * a plain `wj build` / `wj run` follows the file: git dependencies stay at the recorded commit, a manifest that
//!   disagrees with it is an error, and path dependencies whose files changed since the lock are reported as a warning
//!   (they are under development; `--locked` makes it an error).

use super::graph::{self, DepKind, DepNode};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const LOCKFILE: &str = "wj.lock";
pub const VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Recorded commits win; a manifest that disagrees is an error.
    Follow,
    /// Manifest wins; unpinned git dependencies move to their branch head.
    Refresh,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LockEntry {
    pub source: String,
    /// Git commit (empty for a path dependency).
    pub rev: String,
    /// `sha256:<hex>` of a path dependency's files, `git-tree:<sha>` of a git dependency's tree; empty in a version 1 lock.
    pub hash: String,
    /// Names of the entries this package depends on.
    pub dependencies: Vec<String>,
}

/// A whole lock in file order (dependencies first).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lock {
    pub packages: Vec<(String, LockEntry)>,
}

pub fn read(root: &Path) -> BTreeMap<String, LockEntry> {
    read_lock(root).packages.into_iter().collect()
}

pub fn read_lock(root: &Path) -> Lock {
    let mut out = Lock::default();
    let Ok(text) = std::fs::read_to_string(root.join(LOCKFILE)) else { return out };
    let Ok(v) = text.parse::<toml::Value>() else { return out };
    for p in v.get("package").and_then(|p| p.as_array()).into_iter().flatten() {
        let get = |k: &str| p.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        let dependencies = p.get("dependencies").and_then(|d| d.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
        out.packages.push((get("name"), LockEntry { source: get("source"), rev: get("rev"), hash: get("hash"), dependencies }));
    }
    out
}

/// The file text: deterministic, so two runs over the same graph write the same bytes.
pub fn render(lock: &Lock) -> String {
    let mut s = format!("# wj.lock: written by `wj lock update`; commit it. Packages are listed dependencies first.\nversion = {VERSION}\n");
    for (name, e) in &lock.packages {
        s.push_str(&format!("\n[[package]]\nname = \"{name}\"\nsource = \"{}\"\n", e.source));
        if !e.rev.is_empty() {
            s.push_str(&format!("rev = \"{}\"\n", e.rev));
        }
        if !e.hash.is_empty() {
            s.push_str(&format!("hash = \"{}\"\n", e.hash));
        }
        if !e.dependencies.is_empty() {
            let d: Vec<String> = e.dependencies.iter().map(|x| format!("\"{x}\"")).collect();
            s.push_str(&format!("dependencies = [{}]\n", d.join(", ")));
        }
    }
    s
}

/// SHA-256 over the files of a package directory: sorted relative paths and bytes. Build output (`build/`, `target/`),
/// VCS data and caches are not content.
pub fn dir_hash(dir: &Path) -> Result<String, String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let rd = std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let ft = e.file_type().map_err(|e| e.to_string())?;
            if ft.is_dir() {
                if !matches!(name.as_str(), ".git" | "target" | "build" | ".wj-cache" | "node_modules") {
                    walk(base, &p, out)?;
                }
            } else if ft.is_file() && name != ".wj-building" && name != ".DS_Store" {
                out.push(p);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    let mut rel: Vec<(String, PathBuf)> = files.into_iter().map(|p| (p.strip_prefix(dir).unwrap_or(&p).to_string_lossy().replace('\\', "/"), p)).collect();
    rel.sort();
    let mut h = Sha256::new();
    for (r, p) in rel {
        let bytes = std::fs::read(&p).map_err(|e| format!("cannot read {}: {e}", p.display()))?;
        h.update(r.as_bytes());
        h.update([0]);
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(&bytes);
    }
    Ok(format!("sha256:{}", h.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>()))
}

fn entry_of(n: &DepNode, deps: Vec<String>) -> Result<LockEntry, String> {
    let root = n.root.as_ref().ok_or_else(|| format!("dependency `{}` has no directory", n.name))?;
    match &n.rev {
        Some(rev) => {
            let tree = graph::git(&["rev-parse", "HEAD^{tree}"], Some(root)).map_err(|e| format!("dependency `{}`: {e}", n.name))?;
            Ok(LockEntry { source: n.source.clone(), rev: rev.clone(), hash: format!("git-tree:{tree}"), dependencies: deps })
        }
        None => Ok(LockEntry { source: n.source.clone(), rev: String::new(), hash: dir_hash(root)?, dependencies: deps }),
    }
}

/// The full graph of the project at `root`, dependencies first (ties by name), resolved under `policy` against `old`.
pub fn plan(root: &Path, old: &BTreeMap<String, LockEntry>, policy: Policy) -> Result<Lock, String> {
    let mut pins = old.clone();
    let mut dirty = false;
    let mut nodes: BTreeMap<String, (DepNode, Vec<String>)> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    fn visit(root: &Path, dir: &Path, pins: &mut BTreeMap<String, LockEntry>, policy: Policy, dirty: &mut bool, nodes: &mut BTreeMap<String, (DepNode, Vec<String>)>, order: &mut Vec<String>, stack: &mut Vec<String>) -> Result<Vec<String>, String> {
        let mut direct = graph::resolve_in(root, dir, pins, policy, dirty)?;
        direct.sort_by(|a, b| a.name.cmp(&b.name));
        let mut names = Vec::new();
        for n in direct {
            if n.kind == DepKind::Rust && n.root.is_none() {
                continue; // a registry (Cargo) dependency: Cargo.lock owns it
            }
            if stack.contains(&n.name) {
                return Err(format!("dependency cycle: {} -> {}", stack.join(" -> "), n.name));
            }
            names.push(n.name.clone());
            if let Some((seen, _)) = nodes.get(&n.name) {
                if seen.source != n.source || seen.rev != n.rev {
                    return Err(format!("dependency `{}` is required from two sources: {} and {}", n.name, seen.source, n.source));
                }
                continue;
            }
            let child_dir = n.root.clone();
            let name = n.name.clone();
            let kind = n.kind.clone();
            nodes.insert(name.clone(), (n, Vec::new()));
            let deps = match (kind, child_dir) {
                (DepKind::Windjammer, Some(d)) => {
                    stack.push(name.clone());
                    let r = visit(root, &d, pins, policy, dirty, nodes, order, stack);
                    stack.pop();
                    r?
                }
                _ => Vec::new(),
            };
            if let Some(slot) = nodes.get_mut(&name) {
                slot.1 = deps;
            }
            order.push(name);
        }
        Ok(names)
    }
    visit(root, root, &mut pins, policy, &mut dirty, &mut nodes, &mut order, &mut Vec::new())?;
    let mut lock = Lock::default();
    let mut seen = BTreeSet::new();
    for name in order {
        if !seen.insert(name.clone()) {
            continue;
        }
        let (n, deps) = &nodes[&name];
        lock.packages.push((name.clone(), entry_of(n, deps.clone())?));
    }
    Ok(lock)
}

/// `wj lock update`: re-resolve the whole graph and write `wj.lock`. Returns the file text.
pub fn update(root: &Path) -> Result<String, String> {
    let old = read(root);
    let lock = plan(root, &old, Policy::Refresh)?;
    write(root, &lock)
}

/// Write the lock for the graph as it stands, keeping every recorded commit (used when a git dependency is first pinned).
pub fn sync(root: &Path) -> Result<(), String> {
    let old = read(root);
    let lock = plan(root, &old, Policy::Follow)?;
    write(root, &lock).map(|_| ())
}

fn write(root: &Path, lock: &Lock) -> Result<String, String> {
    let text = render(lock);
    if lock.packages.is_empty() && !root.join(LOCKFILE).exists() {
        return Ok(text);
    }
    std::fs::write(root.join(LOCKFILE), &text).map_err(|e| e.to_string())?;
    Ok(text)
}

/// What differs between the recorded lock and the graph as the manifests resolve it now.
pub struct Drift {
    /// Always an error: a revision, source or tree moved, a dependency was added or removed.
    pub errors: Vec<String>,
    /// Path dependency files changed since the lock: an error under `--locked`.
    pub path_changes: Vec<String>,
}

pub fn diff(root: &Path) -> Result<Drift, String> {
    let recorded = read_lock(root);
    let old: BTreeMap<String, LockEntry> = recorded.packages.iter().cloned().collect();
    let now = plan(root, &old, Policy::Follow)?;
    let mut d = Drift { errors: Vec::new(), path_changes: Vec::new() };
    let fix = "run `wj lock update`";
    for (name, e) in &now.packages {
        match old.get(name) {
            None => d.errors.push(format!("dependency `{name}` ({}) is not in {LOCKFILE}; {fix}", e.source)),
            Some(l) => {
                if l.source != e.source {
                    d.errors.push(format!("dependency `{name}`: wj.toml says {} but {LOCKFILE} records {}; {fix}", e.source, l.source));
                } else if l.rev != e.rev {
                    d.errors.push(format!("dependency `{name}`: resolved to revision {} but {LOCKFILE} records {}; {fix}", e.rev, l.rev));
                } else if l.hash.is_empty() {
                    d.errors.push(format!("dependency `{name}` has no content hash in {LOCKFILE} (a version 1 lock); {fix}"));
                } else if l.hash != e.hash {
                    if e.rev.is_empty() {
                        d.path_changes.push(format!("dependency `{name}` ({}) changed since {LOCKFILE} was written ({} -> {}); {fix}", e.source, short(&l.hash), short(&e.hash)));
                    } else {
                        d.errors.push(format!("dependency `{name}`: the tree of revision {} is {} but {LOCKFILE} records {}; the upstream history was rewritten", e.rev, short(&e.hash), short(&l.hash)));
                    }
                }
                if l.dependencies != e.dependencies {
                    d.errors.push(format!("dependency `{name}`: its dependencies are now [{}] but {LOCKFILE} records [{}]; {fix}", e.dependencies.join(", "), l.dependencies.join(", ")));
                }
            }
        }
    }
    for (name, _) in &recorded.packages {
        if !now.packages.iter().any(|(n, _)| n == name) {
            d.errors.push(format!("{LOCKFILE} lists `{name}`, which no manifest depends on any more; {fix}"));
        }
    }
    let ordered: Vec<&String> = recorded.packages.iter().map(|(n, _)| n).collect();
    let want: Vec<&String> = now.packages.iter().map(|(n, _)| n).collect();
    if d.errors.is_empty() && ordered != want {
        d.errors.push(format!("{LOCKFILE} lists packages in a different order than their dependencies require; {fix}"));
    }
    Ok(d)
}

fn short(h: &str) -> String {
    let (k, v) = h.split_once(':').unwrap_or(("", h));
    format!("{k}:{}", &v[..v.len().min(12)])
}

/// What `wj build` does before generating code. No lock file: nothing (a project without dependencies has none; one with
/// git dependencies gets one when they are first resolved). With one: drift is an error; path dependency changes are a
/// warning, or an error with `strict` (`wj build --locked`). A strict check with no lock file is an error too.
pub fn enforce(root: &Path, strict: bool) -> Result<Vec<String>, String> {
    let has_manifest = root.join("wj.toml").is_file();
    if !has_manifest {
        return Ok(Vec::new());
    }
    if !root.join(LOCKFILE).is_file() {
        return if strict { Err(format!("--locked needs a {LOCKFILE}; run `wj lock update` and commit it")) } else { Ok(Vec::new()) };
    }
    let d = diff(root)?;
    let mut errors = d.errors;
    let mut warnings = Vec::new();
    if strict {
        errors.extend(d.path_changes);
    } else {
        warnings = d.path_changes;
    }
    if errors.is_empty() {
        Ok(warnings)
    } else {
        Err(errors.join("\n"))
    }
}


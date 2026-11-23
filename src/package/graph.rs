//! Dependency graph of a `wj.toml` project: path and git dependencies (ADR-0010 option A), pinned by `wj.lock`.
//!
//! One graph for both execution paths: the interpreter loads Windjammer dependencies as source from
//! [`DepNode::source_root`]; the compiled build points Cargo at the same directories. Git dependencies are
//! cloned once into a cache keyed by URL and checked out at the commit recorded in `wj.lock` (written on
//! first resolution, so later runs and machines get the same code).

use super::lock::{self, LockEntry, Policy};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use super::lock::LOCKFILE;

/// What a dependency is, decided from its files (not its name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepKind {
    /// Has `wj.toml` or `.wj` sources: loaded as Windjammer source.
    Windjammer,
    /// A Rust crate (Cargo.toml, or a version-only registry dependency).
    Rust,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepNode {
    /// Name as written in `wj.toml` (`-` kept); the importable crate name is [`DepNode::crate_name`].
    pub name: String,
    pub kind: DepKind,
    /// Package directory (None for registry-only Rust crates).
    pub root: Option<PathBuf>,
    /// Pinned git commit, when the dependency comes from git.
    pub rev: Option<String>,
    pub source: String,
}

impl DepNode {
    pub fn crate_name(&self) -> String {
        self.name.replace('-', "_")
    }

    /// Directory whose files form the library's module tree (`src/` when present).
    pub fn source_root(&self) -> Option<PathBuf> {
        let root = self.root.as_ref()?;
        let src = root.join("src");
        Some(if src.is_dir() { src } else { root.clone() })
    }

    /// Library entry (`lib.wj` / `mod.wj` at the source root), if any.
    pub fn lib_entry(&self) -> Option<PathBuf> {
        let s = self.source_root()?;
        ["lib.wj", "mod.wj"].iter().map(|f| s.join(f)).find(|p| p.is_file())
    }
}

/// Cache directory for git checkouts: `<project>/.wj-cache/git` (the project-local cache that is already
/// gitignored). Project-scoped, so resolution never reads the environment or the user's home directory.
pub fn git_cache_dir(project_root: &Path) -> PathBuf {
    project_root.join(".wj-cache").join("git")
}

pub(crate) fn git(args: &[&str], dir: Option<&Path>) -> Result<String, String> {
    let mut c = Command::new("git");
    if let Some(d) = dir {
        c.current_dir(d);
    }
    let o = c.args(args).output().map_err(|e| format!("cannot run git: {e}"))?;
    if !o.status.success() {
        return Err(format!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&o.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn cache_key(url: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in url.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    let stem = url.trim_end_matches(".git").rsplit('/').next().unwrap_or("repo");
    format!("{}-{:016x}", stem, h)
}

/// Check out `url` at `rev` (or `branch`/default HEAD when unlocked) in the cache; returns (dir, commit).
fn checkout(root: &Path, url: &str, branch: Option<&str>, rev: Option<&str>) -> Result<(PathBuf, String), String> {
    let base = git_cache_dir(root).join(cache_key(url));
    std::fs::create_dir_all(base.parent().unwrap()).map_err(|e| e.to_string())?;
    if !base.join(".git").is_dir() {
        git(&["clone", "--quiet", url, &base.to_string_lossy()], None)?;
    }
    let have = |r: &str| git(&["cat-file", "-e", &format!("{r}^{{commit}}")], Some(&base)).is_ok();
    let target = match rev {
        Some(r) => {
            if !have(r) {
                git(&["fetch", "--quiet", "origin"], Some(&base))?;
            }
            r.to_string()
        }
        None => {
            git(&["fetch", "--quiet", "origin"], Some(&base)).ok();
            let refname = match branch {
                Some(b) => format!("origin/{b}"),
                None => "origin/HEAD".to_string(),
            };
            git(&["rev-parse", &refname], Some(&base)).or_else(|_| git(&["rev-parse", "HEAD"], Some(&base)))?
        }
    };
    let cur = git(&["rev-parse", "HEAD"], Some(&base)).unwrap_or_default();
    if cur != target {
        git(&["checkout", "--quiet", "--detach", &target], Some(&base))?;
    }
    Ok((base, target))
}

fn classify(root: &Path) -> DepKind {
    if root.join("wj.toml").is_file()
        || root.join("src").join("lib.wj").is_file()
        || root.join("lib.wj").is_file()
        || root.join("src").join("mod.wj").is_file()
    {
        DepKind::Windjammer
    } else {
        DepKind::Rust
    }
}

/// Is the `path` dependency directory `dir` a Windjammer package (directly, or as `../pkg/build`, the generated
/// output of one)? The single rule for "Windjammer or Rust" that every consumer shares (ADR-0026).
pub fn is_windjammer_path_dep(dir: &Path) -> bool {
    classify(&package_of_build_output(dir.to_path_buf())) == DepKind::Windjammer
}

/// `path = "../pkg/build"` names the generated output of the Windjammer package `../pkg`: the package is the
/// dependency (its `src/` and `wj.toml`), not the Rust crate that was transpiled from it.
fn package_of_build_output(dir: PathBuf) -> PathBuf {
    if dir.file_name().is_some_and(|n| n == "build") && classify(&dir) == DepKind::Rust {
        if let Some(parent) = dir.parent() {
            if classify(parent) == DepKind::Windjammer {
                return parent.to_path_buf();
            }
        }
    }
    dir
}

/// Expand `${NAME}` and `${NAME:-default}` in a manifest `path`, so a location (a sibling checkout, the
/// ecosystem) is configurable per machine instead of baked into the manifest as `../name`.
pub fn expand_path_vars(raw: &str) -> Result<String, String> {
    expand_path_vars_with(raw, |k| crate::toolchain::env::get(k).filter(|v| !v.is_empty()))
}

pub fn expand_path_vars_with(raw: &str, get: impl Fn(&str) -> Option<String>) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(i) = rest.find("${") {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('}') else { return Err(format!("unterminated `${{` in path `{raw}`")) };
        let inner = &rest[i + 2..i + j];
        let (name, default) = match inner.split_once(":-") {
            Some((n, d)) => (n, Some(d)),
            None => (inner, None),
        };
        match get(name).or_else(|| default.map(str::to_string)) {
            Some(v) => out.push_str(&v),
            None => return Err(format!("path `{raw}` needs the environment variable {name}, which is not set")),
        }
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// A manifest `path` resolved against `base`: variables expanded (left as written when they cannot be).
pub fn dep_path(base: &Path, raw: &str) -> PathBuf {
    let p = expand_path_vars(raw).unwrap_or_else(|_| raw.to_string());
    if Path::new(&p).is_absolute() { PathBuf::from(p) } else { base.join(p) }
}

/// A `${VAR}`-parameterised path dependency whose directory is missing.
struct MissingDep {
    name: String,
    why: String,
    raw: String,
    vars: Vec<String>,
}

fn missing_configurable(root: &Path) -> Vec<MissingDep> {
    let Ok(text) = std::fs::read_to_string(root.join("wj.toml")) else { return Vec::new() };
    let Ok(v) = text.parse::<toml::Value>() else { return Vec::new() };
    let Some(deps) = v.get("dependencies").and_then(|d| d.as_table()) else { return Vec::new() };
    let mut out = Vec::new();
    for (name, spec) in deps {
        let Some(raw) = spec.get("path").and_then(|x| x.as_str()) else { continue };
        if !raw.contains("${") {
            continue;
        }
        let vars: Vec<String> = raw.split("${").skip(1).filter_map(|s| s.split(['}', ':']).next()).map(str::to_string).collect();
        let why = match expand_path_vars(raw) {
            Err(e) => e,
            Ok(p) => {
                let dir = if Path::new(&p).is_absolute() { PathBuf::from(&p) } else { root.join(&p) };
                if dir.exists() {
                    continue;
                }
                format!("`{}` does not exist", dir.display())
            }
        };
        out.push(MissingDep { name: name.clone(), why, raw: raw.to_string(), vars });
    }
    out
}

fn one_message(m: &MissingDep) -> String {
    format!(
        "dependency `{}`: {}; set {} to the checkout that provides it (manifest path `{}`)",
        m.name,
        m.why,
        m.vars.join(" / "),
        m.raw
    )
}

/// One message per `${VAR}`-parameterised path dependency whose directory is missing, naming the variable to set.
pub fn missing_configurable_deps(root: &Path) -> Vec<String> {
    missing_configurable(root).iter().map(one_message).collect()
}

/// The `${VAR:-default}` default written in a manifest path for `var`, if any.
fn manifest_default(raw: &str, var: &str) -> Option<String> {
    let at = raw.find(&format!("${{{var}:-"))? + var.len() + 4;
    raw[at..].split('}').next().map(str::to_string)
}

/// What `wj check` prints: one message per set of variables, listing every dependency that is missing because of
/// them (a manifest with six dependencies under one variable gets one message that names the variable once).
pub fn missing_configurable_report(root: &Path) -> Vec<String> {
    let all = missing_configurable(root);
    let mut groups: Vec<(String, Vec<&MissingDep>)> = Vec::new();
    for m in &all {
        let key = m.vars.join(" / ");
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, g)) => g.push(m),
            None => groups.push((key, vec![m])),
        }
    }
    groups
        .into_iter()
        .map(|(vars, g)| {
            if let [only] = g.as_slice() {
                return one_message(only);
            }
            let names: Vec<String> = g.iter().map(|m| format!("`{}`", m.name)).collect();
            let mut defaults: Vec<String> = g.iter().flat_map(|m| m.vars.iter().filter_map(|v| manifest_default(&m.raw, v))).collect();
            defaults.sort();
            defaults.dedup();
            let default = if defaults.is_empty() {
                String::new()
            } else {
                format!(" The manifest default is `{}`.", defaults.join("`, `"))
            };
            format!(
                "{} dependencies are missing: {}. Set {vars} to the checkout that provides them.{default}",
                g.len(),
                names.join(", ")
            )
        })
        .collect()
}

/// Direct dependencies of the project at `root` (`[dependencies]`), git ones fetched and locked.
/// With a `wj.lock` present the lock decides: a git dependency whose manifest `rev` or url no longer matches its
/// entry is an error naming `wj lock update`, never a silent rewrite. The lock is (re)generated in full (every
/// package, hashes, dependency order) only when a git dependency was resolved without a pin.
pub fn resolve(root: &Path) -> Result<Vec<DepNode>, String> {
    let mut lock = lock::read(root);
    let mut dirty = false;
    let out = resolve_in(root, root, &mut lock, Policy::Follow, &mut dirty)?;
    if dirty {
        lock::sync(root)?;
    }
    Ok(out)
}

/// The dependencies declared by the manifest in `dir`. Git checkouts live in the cache of `cache_root` (the project
/// being built), so a dependency's own dependencies resolve into the same cache.
pub(crate) fn resolve_in(cache_root: &Path, dir: &Path, lock: &mut BTreeMap<String, LockEntry>, policy: Policy, dirty: &mut bool) -> Result<Vec<DepNode>, String> {
    resolve_table_in(cache_root, dir, lock, policy, dirty, "dependencies")
}

/// The project's `[dependencies]` and its `[dev-dependencies]` (what its tests see; a dependency's own
/// dev-dependencies are never part of a build, so only the project being tested asks for them).
pub fn resolve_with_dev(root: &Path) -> Result<Vec<DepNode>, String> {
    let mut nodes = resolve(root)?;
    let mut lock = lock::read(root);
    let mut dirty = false;
    for dev in resolve_table_in(root, root, &mut lock, Policy::Follow, &mut dirty, "dev-dependencies")? {
        if !nodes.iter().any(|n| n.name == dev.name) {
            nodes.push(dev);
        }
    }
    Ok(nodes)
}

fn resolve_table_in(cache_root: &Path, dir: &Path, lock: &mut BTreeMap<String, LockEntry>, policy: Policy, dirty: &mut bool, table: &str) -> Result<Vec<DepNode>, String> {
    let root = dir;
    let manifest = root.join("wj.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else { return Ok(Vec::new()) };
    let v: toml::Value = text.parse().map_err(|e| format!("{}: {}", manifest.display(), e))?;
    let mut out = Vec::new();
    let Some(deps) = v.get(table).and_then(|d| d.as_table()) else { return Ok(out) };
    for (name, spec) in deps {
        let s = |k: &str| spec.get(k).and_then(|x| x.as_str()).map(str::to_string);
        if let Some(p) = s("path") {
            if let Some(msg) = missing_configurable_deps(root).into_iter().find(|m| m.starts_with(&format!("dependency `{name}`:"))) {
                return Err(msg);
            }
            let p = expand_path_vars(&p).map_err(|e| format!("dependency `{name}`: {e}"))?;
            let dir = if Path::new(&p).is_absolute() { PathBuf::from(&p) } else { root.join(&p) };
            let dir = package_of_build_output(dir);
            out.push(DepNode { name: name.clone(), kind: classify(&dir), root: Some(dir), rev: None, source: format!("path+{p}") });
        } else if let Some(url) = s("git") {
            let source = format!("git+{url}");
            let locked = lock.get(name);
            if policy == Policy::Follow {
                if let Some(l) = locked {
                    if l.source != source {
                        return Err(format!("dependency `{name}`: wj.toml now says {source} but {LOCKFILE} records {}; run `wj lock update` to re-resolve it", l.source));
                    }
                    if let Some(r) = s("rev") {
                        if l.rev != r {
                            return Err(format!("dependency `{name}`: wj.toml pins rev {r} but {LOCKFILE} records {}; run `wj lock update` to move the lock to the new revision", l.rev));
                        }
                    }
                }
            }
            let pinned = s("rev").or_else(|| if policy == Policy::Follow { locked.filter(|e| e.source == source).map(|e| e.rev.clone()) } else { None });
            let (dir, commit) = checkout(cache_root, &url, s("branch").as_deref(), pinned.as_deref())
                .map_err(|e| format!("dependency `{name}`: {e}"))?;
            let prev = lock.get(name).cloned();
            let entry = LockEntry { source: source.clone(), rev: commit.clone(), hash: prev.as_ref().filter(|p| p.rev == commit).map(|p| p.hash.clone()).unwrap_or_default(), dependencies: prev.map(|p| p.dependencies).unwrap_or_default() };
            if lock.get(name).map(|l| (&l.source, &l.rev)) != Some((&entry.source, &entry.rev)) {
                lock.insert(name.clone(), entry);
                *dirty = true;
            }
            out.push(DepNode { name: name.clone(), kind: classify(&dir), root: Some(dir), rev: Some(commit), source });
        } else {
            out.push(DepNode { name: name.clone(), kind: DepKind::Rust, root: None, rev: None, source: "registry".into() });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, s: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    #[test]
    fn build_output_path_resolves_to_its_windjammer_package() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        write(r, "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\nwj_x = { path = \"../pkg/build\" }\nraw = { path = \"../raw/build\" }\n");
        write(r, "pkg/wj.toml", "[package]\nname = \"pkg\"\n");
        write(r, "pkg/src/lib.wj", "pub fn f() -> int { 1 }\n");
        write(r, "pkg/build/Cargo.toml", "[package]\nname = \"pkg\"\n");
        write(r, "raw/build/Cargo.toml", "[package]\nname = \"raw\"\n");
        let g = resolve(&r.join("app")).unwrap();
        let x = g.iter().find(|n| n.name == "wj_x").unwrap();
        assert_eq!(x.kind, DepKind::Windjammer);
        assert!(x.root.as_ref().unwrap().ends_with("pkg"));
        assert!(x.lib_entry().is_some());
        assert_eq!(g.iter().find(|n| n.name == "raw").unwrap().kind, DepKind::Rust);
    }

    #[test]
    fn path_dependencies_are_classified_by_their_files() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        write(r, "app/wj.toml", "[package]\nname = \"app\"\n\n[dependencies]\ntext-kit = { path = \"../text_kit\" }\nffi_crate = { path = \"../ffi\" }\nserde = \"1\"\n");
        write(r, "text_kit/wj.toml", "[package]\nname = \"text-kit\"\n");
        write(r, "text_kit/src/lib.wj", "pub fn shout(s: string) -> string { s }\n");
        write(r, "ffi/Cargo.toml", "[package]\nname = \"ffi_crate\"\n");
        let g = resolve(&r.join("app")).unwrap();
        let kinds: Vec<(String, DepKind)> = g.iter().map(|n| (n.crate_name(), n.kind.clone())).collect();
        assert!(kinds.contains(&("text_kit".into(), DepKind::Windjammer)));
        assert!(kinds.contains(&("ffi_crate".into(), DepKind::Rust)));
        assert!(kinds.contains(&("serde".into(), DepKind::Rust)));
        let tk = g.iter().find(|n| n.name == "text-kit").unwrap();
        assert_eq!(tk.lib_entry().unwrap(), r.join("app/../text_kit/src/lib.wj"));
    }

    #[test]
    fn path_variables_expand_with_defaults_and_report_what_is_missing() {
        let env = |k: &str| if k == "WJ_ECOSYSTEM" { Some("/opt/eco".to_string()) } else { None };
        assert_eq!(expand_path_vars_with("${WJ_ECOSYSTEM:-../../eco}/packages/x", env).unwrap(), "/opt/eco/packages/x");
        assert_eq!(expand_path_vars_with("${NOPE:-../../eco}/packages/x", env).unwrap(), "../../eco/packages/x");
        assert_eq!(expand_path_vars_with("../plain", env).unwrap(), "../plain");
        let e = expand_path_vars_with("${NOPE}/x", env).unwrap_err();
        assert!(e.contains("NOPE") && e.contains("not set"), "{e}");
    }

    #[test]
    fn missing_configurable_dependency_names_the_variable_to_set() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("wj.toml"),
            "[package]\nname = \"p\"\n\n[dependencies]\neco = { path = \"${WJ_TEST_UNSET_ECO:-nowhere}/packages/eco\" }\nplain = { path = \"../elsewhere\" }\n",
        )
        .unwrap();
        let msgs = missing_configurable_deps(d.path());
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(msgs[0].contains("WJ_TEST_UNSET_ECO") && msgs[0].contains("`eco`"), "{msgs:?}");
        assert!(resolve(d.path()).unwrap_err().contains("WJ_TEST_UNSET_ECO"));
    }

    #[test]
    fn many_missing_dependencies_under_one_variable_make_one_message() {
        let d = tempfile::tempdir().unwrap();
        let mut toml = String::from("[package]\nname = \"p\"\n\n[dependencies]\n");
        for n in ["a", "b", "c"] {
            toml.push_str(&format!("{n} = {{ path = \"${{WJ_TEST_UNSET_ECO:-../eco}}/packages/{n}\" }}\n"));
        }
        toml.push_str("d = { path = \"${WJ_TEST_UNSET_OTHER}/d\" }\n");
        std::fs::write(d.path().join("wj.toml"), toml).unwrap();
        let msgs = missing_configurable_report(d.path());
        assert_eq!(msgs.len(), 2, "{msgs:?}");
        let eco = msgs.iter().find(|m| m.contains("WJ_TEST_UNSET_ECO")).unwrap();
        assert_eq!(eco.matches("WJ_TEST_UNSET_ECO").count(), 1, "{eco}");
        assert!(eco.contains("`a`") && eco.contains("`b`") && eco.contains("`c`") && eco.contains("`../eco`"), "{eco}");
        assert!(msgs.iter().any(|m| m.starts_with("dependency `d`")), "a single missing dependency keeps its own message");
        assert_eq!(missing_configurable_deps(d.path()).len(), 4, "resolve still gets one message per dependency");
    }

    #[test]
    fn git_dependency_is_cloned_once_and_pinned_in_the_lockfile() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        std::env::set_var("WJ_HOME", r.join("home"));
        write(r, "lib/src/lib.wj", "pub fn v() -> int { 1 }\n");
        let g = |args: &[&str]| git(args, Some(&r.join("lib"))).unwrap();
        g(&["init", "-q"]);
        g(&["-c", "user.email=t@t", "-c", "user.name=t", "add", "."]);
        g(&["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "one"]);
        let first = g(&["rev-parse", "HEAD"]);
        let url = format!("file://{}", r.join("lib").display());
        write(r, "app/wj.toml", &format!("[package]\nname = \"app\"\n\n[dependencies]\nlib = {{ git = \"{url}\" }}\n"));
        let n = resolve(&r.join("app")).unwrap();
        assert_eq!(n[0].rev.as_deref(), Some(first.as_str()));
        assert!(std::fs::read_to_string(r.join("app/wj.lock")).unwrap().contains(&first));
        // a new upstream commit is NOT picked up while the lock pins the old one
        write(r, "lib/src/lib.wj", "pub fn v() -> int { 2 }\n");
        g(&["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qam", "two"]);
        let n2 = resolve(&r.join("app")).unwrap();
        assert_eq!(n2[0].rev.as_deref(), Some(first.as_str()));
        let src = std::fs::read_to_string(n2[0].lib_entry().unwrap()).unwrap();
        assert!(src.contains("{ 1 }"), "locked checkout must stay at the pinned commit: {src}");
        std::env::remove_var("WJ_HOME");
    }
}

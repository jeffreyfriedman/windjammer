//! Builds (once) and caches the crate host that lets the interpreter call Rust crate dependencies (ADR-0031).
//!
//! Inputs: the signature surfaces of the crates the program names (hand-written `rust/<crate>.wji` or the
//! generated cache copy, refreshed here exactly as `wj build` refreshes it), their `wj.toml` dependency lines
//! (same renderer as the compiled build) and the versions the project's lock file records. The host is keyed by
//! a hash of all of them plus the generator version. A hit costs one `stat`; a miss writes a small cargo
//! package and runs one `cargo build --release` in a target directory shared by every host, so a crate is
//! compiled once per version and feature set, and each host keeps only its binary (moved next to its sources).

use crate::interpreter::crate_host::{gen, sig::CrateFnSig, CrateHost};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A built host: the binary, the key it announces, the functions it serves, and simulation stubs.
pub struct Prepared {
    pub exe: PathBuf,
    pub key: String,
    pub sigs: Vec<CrateFnSig>,
    pub stubs: std::collections::HashMap<String, serde_json::Value>,
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h = (*h ^ *b as u64).wrapping_mul(0x100000001b3);
    }
    *h = (*h ^ 0xff).wrapping_mul(0x100000001b3); // field separator
}

/// Where hosts live: `<cache>/host/crates/<key>/` and the shared `<cache>/host/crates-target/`.
fn crates_root() -> PathBuf {
    crate::cargo_cache::cache_root().join("host").join("crates")
}

/// Does the program text name `krate` (`krate::f(..)` or `use krate`)?
fn names_crate(texts: &[String], krate: &str) -> bool {
    let call = format!("{krate}::");
    let import = format!("use {krate}");
    texts.iter().any(|t| t.contains(&call) || t.contains(&import))
}

/// The host signatures of the surfaced crates `texts` names. Functions whose types cannot cross are left out
/// (a call to one stays a preflight report naming it); the second value lists why.
pub fn signatures(project_root: &Path, texts: &[String]) -> (Vec<CrateFnSig>, Vec<String>) {
    let crates = crate::package::rust_surface::load(project_root);
    let mut sigs = Vec::new();
    let mut skipped = Vec::new();
    for s in &crates.surfaces {
        if !s.problems.is_empty() || !names_crate(texts, &s.crate_name) {
            continue;
        }
        use crate::interpreter::crate_host::sig::{crate_fn_sig_in, Opaque};
        let free = Opaque { krate: &s.crate_name, types: &s.types, self_ty: None };
        let methods = s.methods.iter().map(|(t, f)| (Opaque { self_ty: Some(t.as_str()), ..free.clone() }, *f));
        let bytes = crate::interpreter::crate_host::sig::byte_monomorphs(&s.text);
        for (scope, f) in s.fns.iter().map(|f| (free.clone(), *f)).chain(methods) {
            match crate_fn_sig_in(&scope, f) {
                Ok(mut sig) => {
                    if let Some((_, names)) = bytes.iter().find(|(n, _)| *n == sig.name) {
                        sig.bytes_or_str = sig.params.iter().enumerate().filter(|(_, (p, _, _))| names.contains(p)).map(|(i, _)| i).collect();
                    }
                    sigs.push(sig)
                }
                Err(why) => skipped.push(why),
            }
        }
    }
    sigs.sort_by_key(|s| s.path());
    (sigs, skipped)
}

/// Dependency lines for the crates in `sigs` and the versions the lock files record for them.
fn dependency_lines(project_root: &Path, sigs: &[CrateFnSig]) -> (Vec<String>, Vec<String>) {
    let Ok(cfg) = crate::config::WjConfig::load_from_file(&project_root.join("wj.toml")) else { return (Vec::new(), Vec::new()) };
    let wanted: Vec<&str> = sigs.iter().map(|s| s.krate.as_str()).collect();
    let locks = crate::package::rust_surface_gen::locate::lock_files(project_root, None);
    let mut names: Vec<&String> = cfg.dependencies.keys().filter(|n| wanted.contains(&n.replace('-', "_").as_str())).collect();
    names.sort();
    let lines = names
        .iter()
        .map(|n| crate::cargo_toml::dependency_management::dep_spec_to_cargo_line_with_base(n, &cfg.dependencies[*n], Some(project_root)))
        .collect();
    let versions = names
        .iter()
        .map(|n| format!("{n}={}", crate::package::rust_surface_gen::locate::locked_version(&locks, n, None).unwrap_or_default()))
        .collect();
    (lines, versions)
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "wj-crate-host.exe"
    } else {
        "wj-crate-host"
    }
}

/// Simulation stubs: `fixtures/rust_stubs.json` next to `wj.toml`, `{ "crate::fn": <result> }`.
fn stubs(project_root: &Path) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let p = project_root.join("fixtures").join("rust_stubs.json");
    let Ok(text) = std::fs::read_to_string(&p) else { return Ok(Default::default()) };
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
}

/// Ensure the host for the crates the program at `entry_files` calls; `None` when it names no surfaced crate.
pub fn ensure(project_root: &Path, texts: &[String], log: &mut dyn FnMut(&str)) -> Result<Option<Prepared>, String> {
    if crate::package::rust_surface::load(project_root).is_empty() {
        return Ok(None);
    }
    // the same refresh `wj build` does: crates without a hand-written surface get a generated one (never downloads)
    for note in crate::package::rust_surface_gen::cache::ensure(project_root, None, crate::toolchain::install::cargo_home().as_deref()) {
        log(&format!("note: {note}"));
    }
    let (sigs, _skipped) = signatures(project_root, texts);
    if sigs.is_empty() {
        return Ok(None);
    }
    let (deps, versions) = dependency_lines(project_root, &sigs);
    let mut h: u64 = 0xcbf29ce484222325;
    fnv(&mut h, env!("CARGO_PKG_VERSION").as_bytes());
    fnv(&mut h, gen::GENERATOR_VERSION.to_string().as_bytes());
    fnv(&mut h, gen::host_source("", &sigs).as_bytes());
    for d in deps.iter().chain(&versions) {
        fnv(&mut h, d.as_bytes());
    }
    let key = format!("wj_crate_host_{h:016x}");
    let dir = crates_root().join(&key);
    let exe = dir.join(exe_name());
    let stubs = stubs(project_root)?;
    if exe.is_file() {
        return Ok(Some(Prepared { exe, key, sigs, stubs }));
    }
    let crates: Vec<&str> = {
        let mut c: Vec<&str> = sigs.iter().map(|s| s.krate.as_str()).collect();
        c.dedup();
        c
    };
    log(&format!("wj: building the crate host for {} ({} function(s); once, cached in {})", crates.join(", "), sigs.len(), dir.display()));
    std::fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
    let mut manifest = format!(
        "[package]\nname = \"{key}\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[[bin]]\nname = \"{key}\"\npath = \"src/main.rs\"\n\n[dependencies]\n"
    );
    for d in &deps {
        manifest.push_str(d);
        manifest.push('\n');
    }
    // small and quick to build: the host only forwards calls; the crate code is what runs
    manifest.push_str("\n[profile.release]\nopt-level = 2\ndebug = false\nincremental = false\ncodegen-units = 16\npanic = \"unwind\"\n");
    std::fs::write(dir.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("src/main.rs"), gen::host_source(&key, &sigs)).map_err(|e| e.to_string())?;
    if let Some(lock) = crate::package::rust_surface_gen::locate::lock_files(project_root, None).first() {
        // the versions the compiled build uses
        let _ = std::fs::copy(lock, dir.join("Cargo.lock"));
    }
    let target = crates_root().with_file_name("crates-target");
    let build = |offline: bool| {
        let mut c = Command::new("cargo");
        c.args(["build", "--release", "--quiet"]).current_dir(&dir).env("CARGO_TARGET_DIR", &target);
        if offline {
            c.arg("--offline");
        }
        c.output()
    };
    // the local registry first (fast, no network); then let cargo fetch what is missing
    let mut out = build(true).map_err(|e| format!("cannot run cargo: {e}"))?;
    if !out.status.success() {
        out = build(false).map_err(|e| format!("cannot run cargo: {e}"))?;
    }
    let built = target.join("release").join(if cfg!(windows) { format!("{key}.exe") } else { key.clone() });
    if !built.is_file() && exe.is_file() {
        return Ok(Some(Prepared { exe, key, sigs, stubs })); // a concurrent run built and moved it
    }
    if !out.status.success() || !built.is_file() {
        return Err(format!("crate host build failed (in {}):\n{}", dir.display(), String::from_utf8_lossy(&out.stderr)));
    }
    std::fs::rename(&built, &exe).or_else(|_| std::fs::copy(&built, &exe).map(|_| ())).map_err(|e| format!("{}: {e}", exe.display()))?;
    Ok(Some(Prepared { exe, key, sigs, stubs }))
}

/// Every `.wj` file of the project at `root` (not build output, caches or hidden directories), as text.
fn project_texts(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>, depth: usize) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if depth < 8 && !name.starts_with('.') && !matches!(name.as_str(), "build" | "target" | "node_modules" | "out") {
                    walk(&p, out, depth + 1);
                }
            } else if name.ends_with(".wj") {
                if let Ok(t) = std::fs::read_to_string(&p) {
                    out.push(t);
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out, 0);
    out
}

/// The crate host for whatever project contains `path` (a file or directory), for `wj test --interpret`: the
/// surfaced crates the project's `.wj` files name. `None` outside a project or when none is named. Attach it with
/// `Interpreter::attach_crate_host` before loading the program.
pub fn host_for_path(path: &Path, log: &mut dyn FnMut(&str)) -> Result<Option<CrateHost>, String> {
    host_near(path, false, log)
}

/// The same for `wj repl`, where the program is typed later: every surfaced crate of the project.
pub fn host_for_repl(path: &Path, log: &mut dyn FnMut(&str)) -> Result<Option<CrateHost>, String> {
    host_near(path, true, log)
}

fn host_near(path: &Path, every_crate: bool, log: &mut dyn FnMut(&str)) -> Result<Option<CrateHost>, String> {
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let start = if abs.is_file() { abs.parent().unwrap_or(&abs) } else { &abs };
    let Some(root) = crate::module_system::project::find_manifest_dir(start) else { return Ok(None) };
    let texts = if every_crate {
        crate::package::rust_surface::load(&root).names.iter().map(|n| format!("use {n}")).collect()
    } else {
        project_texts(&root)
    };
    let p = ensure(&root, &texts, log)?;
    Ok(p.map(Prepared::into_host))
}

impl Prepared {
    pub fn into_host(self) -> CrateHost {
        CrateHost::new(self.exe, self.key, self.sigs).with_stubs(self.stubs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crate_is_named_by_a_path_or_an_import() {
        let t = vec!["use strsim\nfn main() {}\n".to_string()];
        assert!(names_crate(&t, "strsim"));
        assert!(names_crate(&["let d = shlex::split(s)".to_string()], "shlex"));
        assert!(!names_crate(&t, "shlex"));
    }

    #[test]
    fn the_key_separates_fields() {
        let (mut a, mut b) = (0u64, 0u64);
        fnv(&mut a, b"ab");
        fnv(&mut a, b"c");
        fnv(&mut b, b"a");
        fnv(&mut b, b"bc");
        assert_ne!(a, b);
    }
}

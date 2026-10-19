//! Builds (once) and caches the host bridge library that lets the interpreter call `extern fn`s
//! implemented in the project's Rust (ADR-0020).
//!
//! Inputs are exactly what the compiled build links: the project's hand-written `.rs` files (project root
//! and source root, as `module_system::module_resolution` discovers them for `lib.rs`) and the
//! `wj.toml` Rust dependencies, rendered with the same `Cargo.toml` line generator. The crate is keyed by a
//! hash of all inputs; a cache hit costs one directory lookup, a miss one `cargo build --release`.

use crate::interpreter::host_bridge::{shim_source, ExternSig};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Hand-written Rust sources next to the program (`ffi.rs`, ...), excluding Cargo entry points.
pub fn rust_sources(project_root: &Path, source_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in [project_root, source_root] {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut v: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .filter(|p| !matches!(p.file_name().and_then(|n| n.to_str()), Some("build.rs" | "main.rs" | "lib.rs" | "mod.rs")))
            .collect();
        v.sort();
        for p in v {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h = (*h ^ *b as u64).wrapping_mul(0x100000001b3);
    }
}

fn cache_root() -> PathBuf {
    crate::package::graph::git_cache_dir().parent().map(|p| p.join("host")).unwrap_or_else(|| std::env::temp_dir().join("wj-host"))
}

fn dylib_name(stem: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else if cfg!(windows) {
        format!("{stem}.dll")
    } else {
        format!("lib{stem}.so")
    }
}

/// Rust dependency lines for the bridge crate (same renderer as the compiled build's Cargo.toml).
fn rust_dependency_lines(project_root: &Path) -> Vec<String> {
    let Ok(cfg) = crate::config::WjConfig::load_from_file(&project_root.join("wj.toml")) else {
        return Vec::new();
    };
    let wj_deps: Vec<String> = crate::package::graph::resolve(project_root)
        .unwrap_or_default()
        .into_iter()
        .filter(|n| n.kind == crate::package::graph::DepKind::Windjammer)
        .map(|n| n.name)
        .collect();
    let mut names: Vec<&String> = cfg.dependencies.keys().filter(|n| !wj_deps.contains(n)).collect();
    names.sort();
    names
        .into_iter()
        .map(|n| crate::cargo_toml::dependency_management::dep_spec_to_cargo_line_with_base(n, &cfg.dependencies[n], Some(project_root)))
        .collect()
}

/// Ensure the bridge library for `sigs` exists; returns its path. `log` receives progress lines.
pub fn ensure(project_root: &Path, rust_files: &[PathBuf], sigs: &[ExternSig], log: &mut dyn FnMut(&str)) -> Result<PathBuf, String> {
    let shims = shim_source(sigs);
    let deps = rust_dependency_lines(project_root);
    let uses_runtime = rust_files.iter().any(|f| std::fs::read_to_string(f).is_ok_and(|s| s.contains("windjammer_runtime")));
    let mut h: u64 = 0xcbf29ce484222325;
    fnv(&mut h, env!("CARGO_PKG_VERSION").as_bytes());
    fnv(&mut h, shims.as_bytes());
    for d in &deps {
        fnv(&mut h, d.as_bytes());
    }
    for f in rust_files {
        fnv(&mut h, f.to_string_lossy().as_bytes());
        fnv(&mut h, &std::fs::read(f).unwrap_or_default());
    }
    let stem = format!("wj_host_{:016x}", h);
    let dir = cache_root().join(&stem);
    let lib = dir.join("target").join("release").join(dylib_name(&stem));
    if lib.is_file() {
        return Ok(lib);
    }
    log(&format!("wj: building the host bridge for {} extern fn(s) (once; cached in {})", sigs.len(), dir.display()));
    std::fs::create_dir_all(dir.join("src")).map_err(|e| e.to_string())?;
    let mut manifest = format!(
        "[package]\nname = \"{stem}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[workspace]\n\n[lib]\ncrate-type = [\"cdylib\"]\npath = \"src/lib.rs\"\n\n[dependencies]\n"
    );
    for d in &deps {
        manifest.push_str(d);
        manifest.push('\n');
    }
    if uses_runtime && !deps.iter().any(|d| d.starts_with("windjammer-runtime")) {
        let rt = crate::cargo_toml::dependency_management::find_windjammer_runtime_path_with(None);
        manifest.push_str(&format!("windjammer-runtime = {{ path = \"{}\", default-features = false }}\n", rt.display()));
    }
    let mut lib_rs = String::from("#![allow(dead_code, unused_imports, non_snake_case, clippy::all)]\n");
    for f in rust_files {
        let m = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        lib_rs.push_str(&format!("#[path = {:?}]\npub mod {};\n", f.display().to_string(), m));
    }
    lib_rs.push_str("mod wj_host_shims;\n");
    std::fs::write(dir.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("src/lib.rs"), lib_rs).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("src/wj_host_shims.rs"), shims).map_err(|e| e.to_string())?;
    let out = Command::new("cargo")
        .args(["build", "--release", "--lib", "--quiet"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .output()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !out.status.success() || !lib.is_file() {
        return Err(format!("host bridge build failed:\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(lib)
}

//! Infer `wj test` options from project layout (`wj.toml`, `Cargo.toml`, `build/`).
//!
//! Explicit CLI flags always win; inference only fills defaults.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{DependencySpec, WjConfig};

use super::options::TestRunOptions;

/// Apply dogfood-friendly defaults when the user did not set explicit flags.
pub fn apply_inferred_test_options(project_root: &Path, opts: &mut TestRunOptions) {
    infer_runtime_path_dep(project_root, opts);
    infer_dogfood_build_layout(project_root, opts);
    infer_module_file(project_root, opts);
}

fn load_wj_config(project_root: &Path) -> Option<WjConfig> {
    for name in ["wj.toml", "windjammer.toml"] {
        let path = project_root.join(name);
        if path.exists() {
            return WjConfig::load_from_file(&path).ok();
        }
    }
    None
}

/// When `wj.toml` declares `windjammer-runtime` with a path, prefer a Cargo path
/// dep over copying the runtime into the temp tree (unless `--copy-runtime`).
fn infer_runtime_path_dep(project_root: &Path, opts: &mut TestRunOptions) {
    if opts.copy_runtime {
        opts.no_runtime_copy = false;
        return;
    }
    if opts.no_runtime_copy && opts.runtime_path.is_some() {
        return;
    }

    let Some(config) = load_wj_config(project_root) else {
        return;
    };

    let runtime_path = config
        .dependencies
        .get("windjammer-runtime")
        .and_then(dependency_path)
        .map(|p| resolve_path(project_root, &p));

    if let Some(path) = runtime_path {
        if !opts.no_runtime_copy {
            opts.no_runtime_copy = true;
        }
        if opts.runtime_path.is_none() {
            opts.runtime_path = Some(path);
        }
    }
}

fn dependency_path(spec: &DependencySpec) -> Option<String> {
    match spec {
        DependencySpec::Detailed { path: Some(p), .. } => Some(p.clone()),
        _ => None,
    }
}

fn resolve_path(project_root: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    project_root
        .join(p)
        .canonicalize()
        .unwrap_or_else(|_| project_root.join(p))
}

/// Detect `[lib] path = "build/lib.rs"` (or similar) + existing outbound tree.
fn infer_dogfood_build_layout(project_root: &Path, opts: &mut TestRunOptions) {
    if opts.use_build_dir.is_some() {
        return;
    }

    let cargo = project_root.join("Cargo.toml");
    if !cargo.exists() {
        return;
    }
    let Ok(content) = fs::read_to_string(&cargo) else {
        return;
    };
    let Some(lib_path) = read_toml_lib_path(&content) else {
        return;
    };

    let build_dir = Path::new(&lib_path)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("build"));

    let abs_build = if build_dir.is_absolute() {
        build_dir.clone()
    } else {
        project_root.join(&build_dir)
    };

    if !abs_build.join("lib.rs").exists() {
        return;
    }

    opts.use_build_dir = Some(build_dir);
    if !opts.use_project_cargo {
        opts.use_project_cargo = true;
    }
}

fn read_toml_lib_path(content: &str) -> Option<String> {
    let lib_start = content.find("[lib]")?;
    let section = &content[lib_start..];
    let path_key = section.find("path")?;
    let after = &section[path_key..];
    let eq = after.find('=')?;
    let rest = after[eq + 1..].trim_start();
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        return Some(stripped[..end].to_string());
    }
    None
}

/// Enable `--module-file` when the project uses `mod.wj` multi-module layout.
///
/// Flat `src/lib.wj` is a crate root, not a module-file project. Inferring
/// `--module-file` from `lib.wj` alone caused `wj test` to build a missing
/// `src/mod.wj`, produce no library `Cargo.toml`, and skip `use crate::` rewrite.
fn infer_module_file(project_root: &Path, opts: &mut TestRunOptions) {
    if opts.module_file {
        return;
    }
    // Fresh compile path (not using prebuilt build/) benefits from module-file layout.
    if opts.use_build_dir.is_some() {
        return;
    }
    let src = project_root.join("src");
    if src.join("mod.wj").exists() {
        opts.module_file = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn infers_no_runtime_copy_from_wj_toml_path_dep() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("wj.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
windjammer-runtime = { path = "../windjammer-runtime" }
"#,
        )
        .unwrap();
        let mut opts = TestRunOptions::default();
        apply_inferred_test_options(root, &mut opts);
        assert!(opts.no_runtime_copy);
        assert!(opts.runtime_path.is_some());
    }

    #[test]
    fn copy_runtime_overrides_inference() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(
            root.join("wj.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"

[dependencies]
windjammer-runtime = { path = "../windjammer-runtime" }
"#,
        )
        .unwrap();
        let mut opts = TestRunOptions {
            copy_runtime: true,
            ..TestRunOptions::default()
        };
        apply_inferred_test_options(root, &mut opts);
        assert!(!opts.no_runtime_copy);
    }

    #[test]
    fn infers_use_build_dir_from_cargo_lib_path() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("build")).unwrap();
        fs::write(root.join("build/lib.rs"), "// stub\n").unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "demo"
version = "0.1.0"
edition = "2021"

[lib]
name = "demo"
path = "build/lib.rs"
"#,
        )
        .unwrap();
        let mut opts = TestRunOptions::default();
        apply_inferred_test_options(root, &mut opts);
        assert_eq!(
            opts.use_build_dir.as_deref(),
            Some(Path::new("build"))
        );
        assert!(opts.use_project_cargo);
    }

    #[test]
    fn flat_lib_wj_does_not_infer_module_file() {
        // Idiomatic single-file packages use src/lib.wj (crate root), not mod.wj.
        // Inferring --module-file made wj test build a missing src/mod.wj and skip
        // the library Cargo.toml / use-crate rewrite (ecosystem wj-dotenv).
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.wj"), "pub fn parse() {}\n").unwrap();
        let mut opts = TestRunOptions::default();
        apply_inferred_test_options(root, &mut opts);
        assert!(
            !opts.module_file,
            "flat src/lib.wj must not enable --module-file"
        );
    }

    #[test]
    fn mod_wj_infers_module_file() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/mod.wj"), "pub mod domain\n").unwrap();
        let mut opts = TestRunOptions::default();
        apply_inferred_test_options(root, &mut opts);
        assert!(opts.module_file);
    }
}

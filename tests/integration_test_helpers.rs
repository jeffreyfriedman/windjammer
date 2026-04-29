//! Multi-file integration test harness for the Windjammer compiler.
//!
//! Shared via `#[path = "integration_test_helpers.rs"] mod integration_test_helpers;` from each
//! integration test binary (Cargo compiles `tests/*.rs` as separate crates).
//!
//! Uses [`windjammer::build_project_ext`] so tests exercise the same multipass pipeline as real
//! library builds (global ownership registry, cross-file trait inference, float/int passes).
//!
//! Prefer this over ad-hoc `Parser::new` + `Box::leak` patterns that hide lifetime bugs and skip
//! real project layout.
//!
//! See [`tests/README.md`](./README.md) for when to use these helpers vs single-string unit tests.
#![allow(dead_code)] // not every test uses every helper; keeps the API stable

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use tempfile::TempDir;
use windjammer::{build_project_ext, CompilationTarget};

/// Serializes `cargo check` invocations to reduce flakes when the suite runs with high parallelism.
static CARGO_CHECK_LOCK: Mutex<()> = Mutex::new(());

/// Temporary Windjammer project: `src/**/*.wj` → `build/**/*.rs` via multipass compilation.
pub struct MultiFileTest {
    /// Root directory containing `src/` (passed to `build_project_ext` as the source path).
    project_root: PathBuf,
    src_root: PathBuf,
    build_dir: PathBuf,
    _temp_dir: TempDir,
}

impl Default for MultiFileTest {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiFileTest {
    /// Creates an empty project with `src/` and `build/` under a fresh temp directory.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("tempdir for MultiFileTest");
        let project_root = temp_dir.path().to_path_buf();
        let src_root = project_root.join("src");
        let build_dir = project_root.join("build");
        fs::create_dir_all(&src_root).expect("create src");
        Self {
            project_root,
            src_root,
            build_dir,
            _temp_dir: temp_dir,
        }
    }

    /// Path to the generated output directory (`build/`).
    pub fn build_dir(&self) -> &Path {
        &self.build_dir
    }

    /// Add a Windjammer source file relative to `src/` (e.g. `"foo.wj"`, `"sub/bar.wj"`).
    pub fn add_file(&mut self, name: &str, content: &str) {
        let path = self.src_root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dirs for .wj");
        }
        fs::write(&path, content).unwrap_or_else(|e| panic!("write {}: {}", path.display(), e));
    }

    /// Run multipass compilation. On success, returns map **relative path in build/** → Rust source
    /// (e.g. `"holder.rs"`, `"types/entity.rs"`).
    ///
    /// On failure, returns a string with the compiler error (includes file context when available).
    pub fn compile(&self) -> Result<HashMap<String, String>, String> {
        build_project_ext(
            &self.src_root,
            &self.build_dir,
            CompilationTarget::Rust,
            false,
            true,
            &[],
        )
        .map_err(|e| format!("build_project_ext failed: {:#}", e))?;

        let mut out = HashMap::new();
        collect_rs_files(&self.build_dir, &self.build_dir, &mut out)
            .map_err(|e| format!("read generated .rs files: {}", e))?;
        Ok(out)
    }

    /// Compile, then assert `build/<file>` exists and contains `pattern`.
    ///
    /// `file` is relative to the build directory (e.g. `"impl_side.rs"`).
    pub fn assert_contains(&self, file: &str, pattern: &str) {
        let map = self
            .compile()
            .unwrap_or_else(|e| panic!("compile() failed before assert_contains: {}", e));
        let content = map.get(file).unwrap_or_else(|| {
            panic!(
                "no generated file {:?} (have keys: {:?})\nproject root: {}",
                file,
                map.keys().collect::<Vec<_>>(),
                self.project_root.display()
            )
        });
        assert!(
            content.contains(pattern),
            "expected {} to contain {:?}; project root {}\n\n----- {} -----\n{}",
            file,
            pattern,
            self.project_root.display(),
            file,
            content
        );
    }

    /// Compile and assert failure; `err_substr` must appear in the error string (case-sensitive).
    pub fn assert_compile_error(&self, err_substr: &str) {
        let err = self.compile().expect_err("expected compile() to fail");
        assert!(
            err.contains(err_substr),
            "expected error to contain {:?}, got:\n{}",
            err_substr,
            err
        );
    }

    /// Compile, synthesize a crate-root [`lib.rs`] that `pub mod ...`s every top-level `.rs` module
    /// in `build/`, write a minimal [`Cargo.toml`], then run `cargo check`.
    ///
    /// **Scope:** Supports **flat** `build/*.rs` layouts (typical for `src/*.wj`-only projects).
    /// Nested module trees are not wired automatically; use [`Self::compile`] and string asserts, or
    /// extend this helper.
    pub fn assert_compiles_without_error(&self) {
        self.compile()
            .unwrap_or_else(|e| panic!("compile failed before cargo check: {}", e));

        write_flat_lib_rs(&self.build_dir).expect("write lib.rs");
        write_verify_cargo_toml(&self.build_dir).expect("write Cargo.toml for cargo check");

        let _guard = CARGO_CHECK_LOCK.lock().unwrap_or_else(|p| p.into_inner());

        let status = Command::new("cargo")
            .current_dir(&self.build_dir)
            .args(["check", "--quiet"])
            .status()
            .unwrap_or_else(|e| panic!("failed to spawn cargo check: {}", e));

        assert!(
            status.success(),
            "cargo check failed in {} (see stdout/stderr above if --quiet hid details); \
             try running `cargo check` in that directory with RUST_BACKTRACE=1",
            self.build_dir.display()
        );
    }
}

fn collect_rs_files(root: &Path, dir: &Path, out: &mut HashMap<String, String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(root, &path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let rel = path
                .strip_prefix(root)
                .expect("path under root")
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path)?;
            out.insert(rel, text);
        }
    }
    Ok(())
}

/// One `pub mod stem;` per `stem.rs` in `build/` (excluding `lib.rs`).
fn write_flat_lib_rs(build_dir: &Path) -> io::Result<()> {
    let mut stems: Vec<String> = Vec::new();
    for entry in fs::read_dir(build_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("rs")
            && path.file_stem().and_then(|s| s.to_str()) != Some("lib")
            && path.file_stem().and_then(|s| s.to_str()) != Some("mod")
        {
            stems.push(
                path.file_stem()
                    .expect("stem")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    stems.sort();
    let body: String = stems
        .into_iter()
        .map(|s| format!("pub mod {};\n", s))
        .collect();
    fs::write(build_dir.join("lib.rs"), body)
}

fn write_verify_cargo_toml(build_dir: &Path) -> io::Result<()> {
    let runtime = windjammer_runtime_path_for_integration_tests();
    let runtime_display = runtime
        .canonicalize()
        .unwrap_or(runtime)
        .display()
        .to_string();
    let cargo = format!(
        r#"[package]
name = "wj_multi_file_integration_verify"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
windjammer-runtime = {{ path = "{}" }}
smallvec = "1.13"
serde = {{ version = "1.0", features = ["derive"] }}

[lib]
path = "lib.rs"
name = "wj_multi_file_integration_verify"
"#,
        runtime_display
    );
    fs::write(build_dir.join("Cargo.toml"), cargo)
}

/// Resolves `crates/windjammer-runtime` from the `windjammer` crate root (`CARGO_MANIFEST_DIR`).
fn windjammer_runtime_path_for_integration_tests() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("crates/windjammer-runtime")
}

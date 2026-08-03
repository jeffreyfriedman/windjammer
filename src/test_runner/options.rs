//! Options for `wj test` — library build parity with `wj build`.

use std::path::{Path, PathBuf};

/// Controls how `wj test` compiles and links the library under test.
#[derive(Debug, Clone)]
pub struct TestRunOptions {
    pub path: Option<PathBuf>,
    pub filter: Option<String>,
    pub nocapture: bool,
    pub parallel: bool,
    pub json: bool,
    /// Compile the project as a library (default when `src/` contains `.wj` files).
    pub library: bool,
    /// Generate `mod.rs` / scoped module layout after transpile (same as `wj build --module-file`).
    pub module_file: bool,
    /// Output directory for a fresh library compile (default: `{temp}/lib`).
    pub output: Option<PathBuf>,
    /// Use a pre-built outbound tree (e.g. `build/`) instead of recompiling the library.
    pub use_build_dir: Option<PathBuf>,
    /// Merge `[dependencies]` from the project root `Cargo.toml` into the test library crate.
    pub use_project_cargo: bool,
    /// Do not copy `windjammer-runtime` into the temp tree; use a Cargo path dependency.
    /// Also inferred when `wj.toml` declares a path dep on `windjammer-runtime`.
    pub no_runtime_copy: bool,
    /// Force recursive copy of `windjammer-runtime` into the temp tree (overrides inference).
    pub copy_runtime: bool,
    /// Explicit path to `windjammer-runtime` (overrides discovery).
    pub runtime_path: Option<PathBuf>,
    /// Skip auto-generated Cargo.toml for the library compile (same as `wj build --no-generate-cargo-toml`).
    pub no_generate_cargo_toml: bool,
}

impl Default for TestRunOptions {
    fn default() -> Self {
        Self {
            path: None,
            filter: None,
            nocapture: false,
            parallel: true,
            json: false,
            library: true,
            module_file: false,
            output: None,
            use_build_dir: None,
            use_project_cargo: false,
            no_runtime_copy: false,
            copy_runtime: false,
            runtime_path: None,
            no_generate_cargo_toml: false,
        }
    }
}

impl TestRunOptions {
    pub fn from_legacy(
        path: Option<&Path>,
        filter: Option<&str>,
        nocapture: bool,
        parallel: bool,
        json: bool,
    ) -> Self {
        Self {
            path: path.map(Path::to_path_buf),
            filter: filter.map(str::to_string),
            nocapture,
            parallel,
            json,
            ..Self::default()
        }
    }
}

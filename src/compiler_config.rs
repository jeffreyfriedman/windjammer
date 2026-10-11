//! Explicit compiler configuration.
//!
//! `CompilerConfig` is plain data. `Default` is exactly production behavior, so tests and
//! the CLI run the same pipeline. The only code that interprets environment variables is
//! [`CompilerConfig::from_env`] (called once at the driver edge); every other module
//! receives the config explicitly. See `src/compiler_config.MODULE.md`.

use crate::codegen::rust::IrCutoverConfig;
use std::path::PathBuf;

/// Behavior-affecting compiler settings. Build once at the driver edge and pass down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerConfig {
    /// Which codegen decisions read from the IR solver (all on in production).
    pub ir_cutover: IrCutoverConfig,
    /// Use the shared IR coercion engine inside `compute_arg_coercion` (experimental).
    pub ir_coercion_engine: bool,
    /// Build libraries with the legacy multipass path instead of Salsa.
    pub legacy_multipass: bool,
    /// Emit AoSoA cache-locality hint comments at the end of generated Rust.
    pub emit_aosoa_hints: bool,
    /// Write a cache-locality JSON report to this path during codegen.
    pub cache_locality_json: Option<PathBuf>,
    /// Fail the build on IR-vs-analyzer shadow discrepancies (`--ir-shadow-validate`).
    pub shadow_validate_strict: bool,
    /// Do not write Cargo.toml for single-file builds (`--no-generate-cargo-toml`).
    pub skip_cargo_toml_generation: bool,
    /// Override the Windjammer stdlib directory (default: `./std` / bundled).
    pub stdlib_path: Option<PathBuf>,
    /// Override the `windjammer-runtime` crate path used in generated Cargo.toml.
    pub runtime_path: Option<PathBuf>,
    /// Override the `game-host` workspace path for generated Cargo.toml.
    pub game_path: Option<PathBuf>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            ir_cutover: IrCutoverConfig::production(),
            ir_coercion_engine: false,
            legacy_multipass: false,
            emit_aosoa_hints: false,
            cache_locality_json: None,
            shadow_validate_strict: false,
            skip_cargo_toml_generation: false,
            stdlib_path: None,
            runtime_path: None,
            game_path: None,
        }
    }
}

impl CompilerConfig {
    /// Read the process environment. Call only from the CLI driver.
    pub fn from_env() -> Self {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Interpret environment-style variables via an injected lookup (pure; testable).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let on = |k: &str| get(k).is_some_and(|v| v == "1");
        let truthy = |k: &str| get(k).is_some_and(|v| v == "1" || v == "true");
        let path = |k: &str| get(k).map(PathBuf::from);
        Self {
            ir_cutover: IrCutoverConfig {
                ownership: !on("WJ_IR_CUTOVER_DISABLE_OWNERSHIP"),
                clones: !on("WJ_IR_CUTOVER_DISABLE_CLONES"),
                param_types: !on("WJ_IR_CUTOVER_DISABLE_PARAM_TYPES"),
                str_ref: !on("WJ_IR_CUTOVER_DISABLE_STR_REF"),
                call_sites: true,
                locals: !on("WJ_IR_CUTOVER_DISABLE_LOCALS"),
            },
            ir_coercion_engine: on("WJ_IR_COERCION_ENGINE"),
            legacy_multipass: truthy("WJ_LEGACY_MULTIPASS"),
            emit_aosoa_hints: on("WJ_EMIT_AOSOA_HINTS"),
            cache_locality_json: path("WJ_CACHE_LOCALITY_JSON"),
            shadow_validate_strict: false,
            skip_cargo_toml_generation: false,
            stdlib_path: path("WINDJAMMER_STDLIB"),
            runtime_path: path("WINDJAMMER_RUNTIME_PATH"),
            game_path: path("game-host_PATH"),
        }
    }

    pub fn with_ir_cutover(mut self, ir_cutover: IrCutoverConfig) -> Self {
        self.ir_cutover = ir_cutover;
        self
    }

    pub fn with_ir_coercion_engine(mut self, on: bool) -> Self {
        self.ir_coercion_engine = on;
        self
    }

    pub fn with_legacy_multipass(mut self, on: bool) -> Self {
        self.legacy_multipass = on;
        self
    }

    pub fn with_emit_aosoa_hints(mut self, on: bool) -> Self {
        self.emit_aosoa_hints = on;
        self
    }

    pub fn with_cache_locality_json(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_locality_json = Some(path.into());
        self
    }

    pub fn with_shadow_validate_strict(mut self, on: bool) -> Self {
        self.shadow_validate_strict = on;
        self
    }

    pub fn with_skip_cargo_toml_generation(mut self, on: bool) -> Self {
        self.skip_cargo_toml_generation = on;
        self
    }

    pub fn with_stdlib_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.stdlib_path = Some(path.into());
        self
    }

    pub fn with_runtime_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.runtime_path = Some(path.into());
        self
    }

    pub fn with_game_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.game_path = Some(path.into());
        self
    }
}

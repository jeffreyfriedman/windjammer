// Clap definitions for the `wj` binary.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wj")]
#[command(about = "Windjammer - A simple language that transpiles to Rust", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new Windjammer project
    New {
        /// Project name
        #[arg(value_name = "NAME")]
        name: String,

        /// Project template (cli, web, lib, wasm)
        #[arg(short, long, default_value = "cli")]
        template: String,
    },

    /// Build a Windjammer project
    Build {
        /// Path to .wj file or directory
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// Output directory (default: ./build)
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Target platform (rust, javascript, wasm)
        #[arg(short, long, value_name = "TARGET", default_value = "rust")]
        target: String,

        /// Defer drop optimization mode (auto, always, never)
        #[arg(long, value_name = "MODE", default_value = "auto", value_parser = ["auto", "always", "never"])]
        defer_drop: String,

        /// Defer drop threshold in bytes (default: 102400 = 100KB)
        #[arg(long, value_name = "BYTES")]
        defer_drop_threshold: Option<usize>,

        /// Enable tree shaking (dead code elimination)
        #[arg(long)]
        tree_shake: bool,

        /// Generate source maps
        #[arg(long)]
        source_maps: bool,

        /// Run cargo build after transpilation and show errors
        #[arg(long)]
        check: bool,

        /// Show raw Rust errors instead of translated Windjammer errors
        #[arg(long)]
        raw_errors: bool,

        /// Automatically apply fixes for fixable errors
        #[arg(long)]
        fix: bool,

        /// Show verbose error output (includes all notes and suggestions)
        #[arg(short, long)]
        verbose: bool,

        /// Show only error count (suppress detailed output)
        #[arg(short, long)]
        quiet: bool,

        /// Filter errors by file path
        #[arg(long, value_name = "PATH")]
        filter_file: Option<PathBuf>,

        /// Filter errors by type (error, warning)
        #[arg(long, value_name = "TYPE")]
        filter_type: Option<String>,

        /// Library mode: exclude test main() functions from output
        #[arg(long)]
        library: bool,

        /// Auto-generate mod.rs with pub mod declarations and re-exports
        #[arg(long)]
        module_file: bool,

        /// Skip cargo build after transpilation (transpile only)
        #[arg(long)]
        no_cargo: bool,

        /// Disable Rust leakage linter warnings (style, .unwrap(), .iter(), etc.)
        #[arg(long)]
        no_lint: bool,

        /// Skip Cargo.toml generation (use project-maintained manifest)
        #[arg(long)]
        no_generate_cargo_toml: bool,

        /// External crate metadata for cross-crate type inference (NAME=PATH, repeatable)
        #[arg(long, value_name = "NAME=PATH")]
        metadata: Vec<String>,

        /// Output diagnostics as machine-readable JSON (for agent/IDE consumption)
        #[arg(long)]
        json: bool,

        /// Run IR shadow validation (compare solver results against legacy analyzer).
        /// Discrepancies are reported as errors instead of warnings (CI gate).
        #[arg(long)]
        ir_shadow_validate: bool,
    },

    /// Run a Windjammer program: interpreted by default (instant start), compiled when it needs to be
    Run {
        /// A .wj file or a directory; omitted: the project containing the current directory (wj.toml)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Binary to run (`[[bin]]` in wj.toml, cmd/<NAME>/main.wj or src/bin/<NAME>.wj)
        #[arg(long, value_name = "NAME")]
        bin: Option<String>,

        /// Target platform (rust, javascript, wasm); non-rust targets always compile
        #[arg(short, long, value_name = "TARGET", default_value = "rust")]
        target: String,

        /// Interpreter only: never fall back to the compiled path (exit 2 on an unsupported feature)
        #[arg(long, conflicts_with_all = ["compiled", "release"])]
        interpret: bool,

        /// Transpile to Rust and run with cargo (the release path), skipping the interpreter
        #[arg(long)]
        compiled: bool,

        /// Like --compiled, with optimizations
        #[arg(long)]
        release: bool,

        /// Re-run on every source change (restarts long-running programs)
        #[arg(long)]
        watch: bool,

        /// Interpret and apply edits to function bodies while the program runs, keeping its state (preview, ADR-0021)
        #[arg(long, conflicts_with_all = ["compiled", "release", "watch"])]
        hot: bool,

        /// Interpreted runs: thread schedule seed (0 = FIFO run-to-block; N explores another
        /// reproducible interleaving at every synchronisation point)
        #[arg(long, value_name = "N", default_value = "0")]
        sched_seed: u64,

        /// Interpreted runs: perturb HashMap/HashSet iteration order (0 = key order; N = a reproducible
        /// shuffle) to expose programs that depend on Rust's unspecified hash order (ADR-0021 D6)
        #[arg(long, value_name = "N", default_value = "0")]
        hash_seed: u64,

        /// Interpreted runs: `walker` (default) or `resolved` (compiled closures, ADR-0020)
        #[arg(long, value_name = "ENGINE", default_value = "walker")]
        engine: String,

        /// Interpreted runs: do not refuse the program for Rust-leakage lints (W####, such as `.unwrap()`); real
        /// compile errors still refuse it
        #[arg(long)]
        no_lint: bool,

        /// Interpreted runs: no real clock; time starts at a fixed instant and only `sleep` moves it
        #[arg(long)]
        deterministic_time: bool,

        /// Interpreted runs: allow plain http:// client requests (default: the http client refuses)
        #[arg(long)]
        allow_network: bool,

        /// Interpreted runs: answer http client calls from a JSON file of canned responses
        #[arg(long, value_name = "FILE")]
        mock_http: Option<PathBuf>,

        /// Interpreted runs: ignore the project's grants and start from an empty one (only time, random
        /// and console); add authority with --allow (ADR-0021)
        #[arg(long)]
        sandbox: bool,

        /// Interpreted runs: grant an effect, e.g. `--allow net.connect=api.example.com`,
        /// `--allow fs.read=./data/**`, `--allow env.read=HOME`, `--allow process=git` (repeatable)
        #[arg(long = "allow", value_name = "EFFECT[=RESOURCE]")]
        allow: Vec<String>,

        /// Interpreted runs without a [capabilities] table: `warn` (default) runs outside-the-profile calls with a
        /// warning, `enforce` stops them (exit 126), `off` disables the gate
        #[arg(long, value_name = "MODE", env = "WJ_CAPS", default_value = "warn")]
        caps: String,

        /// Defer drop optimization mode (auto, always, never)
        #[arg(long, value_name = "MODE", default_value = "auto", value_parser = ["auto", "always", "never"])]
        defer_drop: String,

        /// Defer drop threshold in bytes (default: 102400 = 100KB)
        #[arg(long, value_name = "BYTES")]
        defer_drop_threshold: Option<usize>,
    },

    /// Start the Windjammerscript REPL (interactive interpreter)
    Repl {},

    /// Run tests
    Test {
        /// Directory or file containing tests (defaults to current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Run only tests matching this pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Show output from passing tests
        #[arg(long)]
        nocapture: bool,

        /// Run tests in parallel (default: true)
        #[arg(long, default_value = "true")]
        parallel: bool,

        /// Output results as JSON for tooling
        #[arg(long)]
        json: bool,

        /// Compile the library under test (default when `src/` has `.wj` files)
        #[arg(long, default_value = "true")]
        library: bool,

        /// Generate scoped `mod.rs` layout after transpile (same as `wj build --module-file`)
        #[arg(long)]
        module_file: bool,

        /// Output directory for library compile (default: temp `{lib}/`)
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Use a pre-built outbound tree instead of recompiling the library
        #[arg(long, value_name = "DIR")]
        use_build_dir: Option<PathBuf>,

        /// Merge `[dependencies]` from project root `Cargo.toml` into the test library crate
        #[arg(long)]
        use_project_cargo: bool,

        /// Use a Cargo path dependency for windjammer-runtime (no recursive copy into temp).
        /// Also the default when `wj.toml` declares a path dep on `windjammer-runtime`.
        #[arg(long)]
        no_runtime_copy: bool,

        /// Force copying windjammer-runtime into the temp tree (overrides path-dep inference)
        #[arg(long)]
        copy_runtime: bool,

        /// Explicit path to windjammer-runtime (with `--no-runtime-copy`)
        #[arg(long, value_name = "PATH")]
        runtime_path: Option<PathBuf>,

        /// Skip auto-generated Cargo.toml for library compile
        #[arg(long)]
        no_generate_cargo_toml: bool,

        /// List discovered test files and functions, then exit without compiling
        #[arg(long)]
        list: bool,

        /// Exit 0 when no tests are found (default: finding none is an error)
        #[arg(long)]
        allow_empty: bool,
        /// Run tests (`@test` / `test_*` in `*_test.wj`) in the interpreter: no cargo, no rustc
        #[arg(long)]
        interpret: bool,
        /// Re-run the tests on every source change
        #[arg(long)]
        watch: bool,
        /// Run only the doctests (```wj examples in `///` doc comments)
        #[arg(long)]
        doc: bool,
        /// Also build and run each doctest through the Rust backend (default: interpreter only)
        #[arg(long)]
        compiled: bool,
    },

    /// Shrink a failing project to a minimal, anonymized reproduction (delta debugging)
    Reduce {
        /// Project directory or .wj file to reduce
        #[arg(value_name = "PATH")]
        path: PathBuf,

        /// Bug oracle: `rustc-error E0308[:regex]`, `panic` (alias `crash`), `miscompile`, or `cmd '<shell>'` (exit 0 = bug present)
        #[arg(long, value_name = "SPEC")]
        oracle: String,

        /// Output directory for the minimal project and repro.md
        #[arg(short, long, value_name = "DIR")]
        output: PathBuf,

        /// Candidates evaluated in parallel
        #[arg(long, default_value_t = 2, value_name = "N")]
        jobs: usize,

        /// Seed for the deterministic candidate order
        #[arg(long, default_value_t = 0, value_name = "N")]
        seed: u64,

        /// Maximum number of oracle runs
        #[arg(long, default_value_t = 5000, value_name = "N")]
        max_steps: usize,

        /// Wall-clock budget for the reduction search, in seconds
        #[arg(long, default_value_t = 3600, value_name = "SECS")]
        time_budget: u64,

        /// Timeout for a single oracle run, in seconds
        #[arg(long, default_value_t = 300, value_name = "SECS")]
        candidate_timeout: u64,

        /// Accept a partially anonymized result when some names cannot be renamed without losing the bug
        #[arg(long)]
        keep_names: bool,

        /// Compiler binary used by the oracles (default: this executable)
        #[arg(long, value_name = "WJ")]
        wj_bin: Option<PathBuf>,

        /// Print progress counts to stderr (never source text)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Format Windjammer code
    Fmt {
        /// Check formatting without applying changes (non-zero exit if changes are needed)
        #[arg(long)]
        check: bool,

        /// Format generated Rust crates with `cargo fmt --all` instead of .wj files
        #[arg(long)]
        rust: bool,

        /// Files or directories to format (default: current directory); `-` reads stdin
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,
    },

    /// Migrate deprecated surface syntax (`println!`, `vec![]`, `format!`, ...) to Windjammer forms
    Fix {
        /// File or directory to rewrite in place
        #[arg(value_name = "PATH")]
        path: PathBuf,

        /// Do not write; exit non-zero if any file would change
        #[arg(long)]
        check: bool,

        /// Also migrate ```windjammer / ```wj code fences in .md files
        #[arg(long)]
        docs: bool,
    },

    /// Run Rust leakage linter on .wj files (W0001-W0004)
    Lint {
        /// File or directory to lint
        #[arg(value_name = "PATH")]
        path: PathBuf,

        /// Fail on warnings (for CI)
        #[arg(long)]
        strict: bool,
    },

    /// Type check without building
    Check {
        /// Path to .wj file or directory (default: current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Output diagnostics as machine-readable JSON (docs/diagnostics-schema.json)
        #[arg(long)]
        json: bool,

        /// Re-check whenever a .wj file changes
        #[arg(long)]
        watch: bool,

        /// Apply machine-applicable fixes (W0007/W0008 rewrites, `.as_str()` removal, ...)
        #[arg(long)]
        fix: bool,
    },

    /// Add a dependency to wj.toml
    Add {
        /// Package name
        #[arg(value_name = "PACKAGE")]
        package: String,

        /// Package version
        #[arg(short, long)]
        version: Option<String>,

        /// Comma-separated list of features
        #[arg(short, long)]
        features: Option<String>,

        /// Local path dependency
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Remove a dependency from wj.toml
    Remove {
        /// Package name
        #[arg(value_name = "PACKAGE")]
        package: String,
    },

    /// Update Windjammer to the latest version
    Update {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,

        /// Force reinstall even if already up to date
        #[arg(long)]
        force: bool,
    },

    /// Show error statistics
    Stats {
        /// Clear statistics
        #[arg(long)]
        clear: bool,

        /// Show detailed statistics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Browse the diagnostics of a file or project (summary by code, filterable)
    Errors {
        /// .wj file or project directory (default: current directory)
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Show only this diagnostic code (e.g. WJ0010, W0002)
        #[arg(long, value_name = "CODE")]
        code: Option<String>,

        /// Show only this severity (error, warning)
        #[arg(long, value_name = "SEVERITY")]
        severity: Option<String>,
    },

    /// Search the offline docs index (stdlib signatures, built-in methods, idioms, verified examples,
    /// error codes); with no query, generate error documentation
    Docs {
        /// Search terms, e.g. `wj docs hashmap entry` (all terms must match)
        #[arg(value_name = "QUERY")]
        query: Vec<String>,

        /// Maximum number of search results
        #[arg(long, default_value_t = 8)]
        limit: usize,

        /// Print search results as JSON
        #[arg(long)]
        json: bool,

        /// Output directory for generated docs
        #[arg(short, long, default_value = "./docs/errors")]
        output: PathBuf,

        /// Format (html, markdown, json)
        #[arg(short, long, default_value = "html")]
        format: String,
    },

    /// Explain inferred ownership, clones and numeric widths for a file (rule IDs and use sites)
    #[command(name = "explain-ownership")]
    ExplainOwnership {
        /// Windjammer source file
        file: PathBuf,

        /// Only this function (`name` or `Type::name`)
        #[arg(long = "fn", value_name = "NAME")]
        function: Option<String>,

        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },

    /// Explain the effects (files, network, environment, processes, time, randomness, FFI) each function performs
    #[command(name = "explain-effects")]
    ExplainEffects {
        /// Windjammer source file or project directory
        path: PathBuf,

        /// Only this function (`name` or `Type::name`)
        #[arg(long = "fn", value_name = "NAME")]
        function: Option<String>,

        /// Machine-readable JSON output
        #[arg(long)]
        json: bool,
    },

    /// Explain an error code
    Explain {
        /// Error code to explain (e.g., WJ0001, E0425)
        code: String,
    },

    /// Generate agent index artifacts for MCP (errors, stdlib, spec JSON)
    AgentIndex {
        /// Output directory
        #[arg(short, long, default_value = "agent_index")]
        output: PathBuf,
    },

    /// Compile .wjsl shader to WGSL (with type checking)
    ShaderCompile {
        /// Path to .wjsl file
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output path for .wgsl file (default: stdout)
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Validate WJSL shader files (transpile to WGSL and check for errors)
    ValidateWjsl {
        /// Path to directory containing .wjsl files
        #[arg(value_name = "DIR")]
        path: PathBuf,
    },

    /// Clean build artifacts and stale temp files
    Clean {
        /// Also clean local target/ directories (deep clean)
        #[arg(long)]
        all: bool,
    },

    /// Manage the shared Cargo target cache (outside application repos)
    Cache {
        #[command(subcommand)]
        action: CacheCommand,
    },

    /// Install wj and plugins to ~/.wj/bin/ and ensure PATH
    #[command(name = "self-install")]
    SelfInstall {
        /// Print what would be installed (binary, plugins, embedded std/ files) without writing anything
        #[arg(long)]
        dry_run: bool,
    },

    /// External plugin subcommand (e.g., wj game, wj web)
    #[command(external_subcommand)]
    Plugin(Vec<String>),
}

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Print the shared (or --verify) Cargo target directory
    Path {
        /// Use the ephemeral verify cache (integration / codegen cargo checks)
        #[arg(long)]
        verify: bool,
    },
    /// Show cache sizes and free disk
    Status,
    /// Reclaim space (old incremental; aggressive when disk is low)
    Prune {
        /// Also clear debug incremental trees when reclaiming
        #[arg(long)]
        aggressive: bool,
    },
    /// Write env.sh / env.ps1 for bare `cargo` invocations
    Setup,
}

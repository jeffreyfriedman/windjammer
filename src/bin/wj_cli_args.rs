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
        #[arg(long, value_name = "MODE", default_value = "auto")]
        defer_drop: String,

        /// Defer drop threshold in bytes (default: 102400 = 100KB)
        #[arg(long, value_name = "BYTES")]
        defer_drop_threshold: Option<usize>,

        /// Minify JavaScript output (JS target only)
        #[arg(long)]
        minify: bool,

        /// Enable tree shaking (dead code elimination)
        #[arg(long)]
        tree_shake: bool,

        /// Generate source maps
        #[arg(long)]
        source_maps: bool,

        /// Include polyfills for older browsers (JS target only)
        #[arg(long)]
        polyfills: bool,

        /// Apply V8 optimizations (JS target only)
        #[arg(long)]
        v8_optimize: bool,

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

        /// Skip Cargo.toml generation (use project-maintained manifest)
        #[arg(long)]
        no_generate_cargo_toml: bool,

        /// External crate metadata for cross-crate type inference (NAME=PATH, repeatable)
        #[arg(long, value_name = "NAME=PATH")]
        metadata: Vec<String>,
    },

    /// Compile and run a Windjammer file
    Run {
        /// Path to .wj file
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Target platform (rust, javascript, wasm)
        #[arg(short, long, value_name = "TARGET", default_value = "rust")]
        target: String,

        /// Interpret directly (Windjammerscript mode) — no compilation, instant execution.
        /// Same .wj source can later be compiled with `wj build` for production.
        #[arg(long)]
        interpret: bool,

        /// Defer drop optimization mode (auto, always, never)
        #[arg(long, value_name = "MODE", default_value = "auto")]
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
    },

    /// Format Windjammer code
    Fmt {
        /// Check formatting without applying changes
        #[arg(long)]
        check: bool,
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
    Check,

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

    /// Interactive error navigator (TUI)
    Errors {
        /// File to check
        file: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "./build")]
        output: PathBuf,
    },

    /// Generate error documentation
    Docs {
        /// Output directory for generated docs
        #[arg(short, long, default_value = "./docs/errors")]
        output: PathBuf,

        /// Format (html, markdown, json)
        #[arg(short, long, default_value = "html")]
        format: String,
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

    /// Install wj and plugins to ~/.wj/bin/ and ensure PATH
    #[command(name = "self-install")]
    SelfInstall,

    /// External plugin subcommand (e.g., wj game, wj web)
    #[command(external_subcommand)]
    Plugin(Vec<String>),
}

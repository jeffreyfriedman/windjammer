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
        /// A .wj file or a directory (omitted: the project containing the current directory, wj.toml), then the program's
        /// arguments. Everything after PATH belongs to the program, flags included (`wj run tool.wj --release`); wj's own
        /// flags go before PATH.
        #[arg(value_name = "PATH [ARGS]...", trailing_var_arg = true, allow_hyphen_values = true)]
        path_and_args: Vec<String>,

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

        /// Interpreted runs: `resolved` (default: compiled closures, ADR-0020; functions outside its subset
        /// run in the tree walker) or `walker` (the tree walker only, the reference semantics)
        #[arg(long, value_name = "ENGINE", default_value = "resolved")]
        engine: String,

        /// Interpreted runs: do not compile hot integer functions to machine code (ADR-0020; the same as
        /// `WJ_JIT=0`); they run in the resolved engine's closures
        #[arg(long)]
        no_jit: bool,

        /// Interpreted runs: `on` (default) or `off`: machine code for hot integer functions (ADR-0020)
        #[arg(long, value_name = "on|off", env = "WJ_JIT", default_value = "on", hide = true)]
        jit: String,

        /// Interpreted runs: closure-engine iterations before a hot loop enters machine code (testing:
        /// 0 makes every loop try at its first iteration)
        #[arg(long, value_name = "N", env = "WJ_JIT_LOOP_HOT", hide = true)]
        jit_loop_hot: Option<u32>,

        /// Interpreted runs: report on stderr which loops run as machine code and why others do not
        #[arg(long, env = "WJ_JIT_DEBUG", hide = true, action = clap::ArgAction::SetTrue, value_parser = clap::builder::FalseyValueParser::new())]
        jit_debug: bool,

        /// Interpreted runs (debug builds only): leave one machine-code check out (mutation tests)
        #[arg(long, value_name = "NAME", env = "WJ_JIT_MUTANT", hide = true)]
        jit_mutant: Option<String>,

        /// Interpreted runs: do not refuse the program for Rust-leakage lints (W####, such as `.unwrap()`); real
        /// compile errors still refuse it
        #[arg(long)]
        no_lint: bool,

        /// Interpreted runs: no real clock; time starts at a fixed instant and only `sleep` moves it
        #[arg(long)]
        deterministic_time: bool,

        /// Interpreted runs: write every effect (file, env, process, network and stdin results, real-clock readings)
        /// to FILE, a replayable log (ADR-0021)
        #[arg(long, value_name = "FILE", conflicts_with = "replay")]
        record: Option<PathBuf>,

        /// Interpreted runs: re-run from an effect log written by --record: every effect is answered from the log and
        /// none is performed; seeds, arguments and the time mode come from the log (ADR-0021)
        #[arg(long, value_name = "FILE")]
        replay: Option<PathBuf>,

        /// Interpreted runs: run in the deterministic simulated world (virtual clock, seeded random numbers and
        /// scheduler, in-memory files, no network); the run is a function of the program and `--seed` (ADR-0021)
        #[arg(long, conflicts_with_all = ["record", "replay"])]
        simulate: bool,

        /// With --simulate: the seed of the world
        #[arg(long, value_name = "N", default_value = "1", requires = "simulate")]
        seed: u64,

        /// With --simulate: probability (0 to 1) that a file or network call fails
        #[arg(long, value_name = "RATE", default_value = "0", requires = "simulate")]
        faults: f64,

        /// Interpreted runs: let `mcp::call` reach an MCP server, as NAME=ENDPOINT: `mock:FILE` (canned results) or
        /// `stdio:COMMAND ARGS` (a child process speaking MCP; it keeps the credentials). Repeatable (ADR-0021)
        #[arg(long = "bind", value_name = "NAME=ENDPOINT")]
        bind: Vec<String>,

        /// Interpreted runs: resource limits `fuel=N,wall=SECS,output=BYTES,calls=N,mem=MB`; exceeding one stops the run
        /// with exit status 124
        #[arg(long, value_name = "LIMITS")]
        limits: Option<String>,

        /// Interpreted runs: write the exit status, the effects used and the resources used to FILE as JSON
        #[arg(long, value_name = "FILE")]
        result_json: Option<PathBuf>,

        /// Interpreted runs: keep the step journals of `durable fn` runs in DIR, so a crashed run resumes where it stopped
        /// (default: in memory)
        #[arg(long, value_name = "DIR")]
        journal: Option<PathBuf>,

        /// Interpreted runs: exit abruptly (status 137) after the N-th journaled step of a `durable fn`, or with `N:mid`
        /// after performing it but before journaling it (tests crash recovery)
        #[arg(long, value_name = "N[:mid]", requires = "journal")]
        crash_after: Option<String>,

        /// Interpreted runs: allow network access, http client and server (same as `--allow net.connect=* --allow net.listen=*`)
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

        /// Interpreted runs without a [capabilities] table: `enforce` (default) stops calls outside the project
        /// directory with exit status 126 and prints the exact fix, `warn` runs them with a warning, `off` disables the gate
        #[arg(long, value_name = "MODE", env = "WJ_CAPS", default_value = "enforce")]
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

    /// MCP tools as typed Windjammer: bindings for code mode (ADR-0021)
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },

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
        /// With --interpret: run each test in the deterministic simulated world (ADR-0021)
        #[arg(long, requires = "interpret")]
        simulate: bool,
        /// With --simulate: the seed of the world
        #[arg(long, value_name = "N", requires = "simulate", conflicts_with = "seeds")]
        seed: Option<u64>,
        /// With --simulate: try every seed in A..B (inclusive); a test fails with the first seed that fails it
        #[arg(long, value_name = "A..B", requires = "simulate")]
        seeds: Option<String>,
        /// With --simulate: probability (0 to 1) that a file or network call fails
        #[arg(long, value_name = "RATE", default_value = "0", requires = "simulate")]
        faults: f64,
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

    /// What the project's dependencies are known to do (per-package capabilities and the lock file)
    Caps {
        /// `show` the effects each dependency performs and whether .wj-capabilities.lock covers them;
        /// `approve` records them in the lock file (review with `show` first)
        #[arg(value_parser = ["show", "approve"])]
        action: String,

        /// Project directory (default: the current directory)
        path: Option<PathBuf>,

        /// Machine-readable JSON output (show)
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

    /// Eject to a standalone Cargo project (the `wj build` output plus vendored path dependencies)
    Eject(EjectArgs),

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
pub enum McpCommand {
    /// Generate a typed Windjammer module for the tools of an MCP server: one function per tool, structs for its arguments and results
    Bind {
        /// Name the tools are bound under (`mcp.NAME.TOOL` is the effect of a call)
        #[arg(value_name = "SERVER")]
        server: String,

        /// The server's `tools/list` result (JSON); `-` reads standard input
        #[arg(long, value_name = "FILE", conflicts_with = "stdio")]
        tools: Option<PathBuf>,

        /// Ask a live server instead: the command that starts it (arguments separated by spaces)
        #[arg(long, value_name = "COMMAND")]
        stdio: Option<String>,

        /// Bind only this tool (repeatable): the program's permission request is the tools it can call
        #[arg(long = "tool", value_name = "NAME")]
        only: Vec<String>,

        /// Write the module here (default: standard output)
        #[arg(long, short, value_name = "FILE")]
        out: Option<PathBuf>,
    },
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

/// Arguments of `wj eject`.
#[derive(clap::Args, Debug)]
pub struct EjectArgs {
    /// Windjammer file or project directory
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output directory (must be absent or empty)
    #[arg(short, long)]
    pub output: PathBuf,

    /// Target platform (rust, wasm)
    #[arg(short, long, default_value = "rust")]
    pub target: String,

    /// Library crate (as `wj build --library`)
    #[arg(long)]
    pub library: bool,

    /// Generate mod.rs re-exports (as `wj build --module-file`)
    #[arg(long)]
    pub module_file: bool,
}

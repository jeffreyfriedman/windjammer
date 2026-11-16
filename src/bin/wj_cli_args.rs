// Clap definitions for the `wj` binary.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wj")]
#[command(about = "Windjammer - A simple language that transpiles to Rust", long_about = None)]
#[command(disable_version_flag = true)]
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

        /// Let integer arithmetic wrap on overflow in release builds (default: it panics, as in debug builds
        /// and the interpreter). Same as `[build] overflow_checks = false` in wj.toml.
        #[arg(long)]
        unchecked_arithmetic: bool,

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
        /// A .wj file, a directory, or `-` (the program read from stdin) (omitted: the project containing the current
        /// directory, wj.toml), then the program's
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

        /// Interpret, then build compiled and, when the program only reads stdin and writes to the console, run the
        /// compiled program again and compare its output and exit status (ADR-0033). Reports on stderr. The interpreted output streams as the program prints. Stdin is read
        /// once for both runs; when no input arrives within 2 s it is taken as empty. When the two runs agree it prints `wj verify: OK` (with the
        /// program's own exit status) and exits 0, whatever that status is; a difference or a failed build exits non-zero
        #[arg(long, conflicts_with_all = ["compiled", "release", "watch", "hot", "record", "replay"])]
        verify: bool,

        /// How diagnostics and failed runs report on stderr: `human` (default); `agent`, one compact line per diagnostic
        /// (location, code, message, first help, fixes) for tools and coding agents; or `json` (interpreted runs: one
        /// last line in the diagnostics schema with `stack` and `locals`, ADR-0036). Program stdout and the exit status
        /// do not change. `WJ_ERROR_FORMAT` sets the default for every command
        #[arg(long, value_name = "FORMAT", value_parser = ["human", "agent", "json"])]
        error_format: Option<String>,

        /// Interpreted runs: also start a compiled build of the same content in the background (lowest priority,
        /// offline, one at a time, skipped on a busy machine); the next interpreted run reports a failed build (ADR-0033)
        #[arg(long, conflicts_with_all = ["compiled", "release", "verify"])]
        speculate: bool,

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

        /// Interpreted runs: leave the bindings of the failing frame out of the error report (they are redacted when
        /// they look like secrets, and this drops them altogether)
        #[arg(long)]
        no_locals: bool,

        /// Interpreted runs (internal, hidden from help): run every program listed in FILE (one path per line) under the
        /// tree walker and the resolved engine in this one process and print a JSON line per program and engine (exit
        /// status, stdout, stderr); the drift fuzz step's batch mode (ADR-0038 C4)
        #[arg(long, value_name = "FILE", hide = true)]
        pair_list: Option<PathBuf>,

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

    /// Start the Windjammerscript REPL (interactive interpreter): a persistent session whose input continues on the next line until it parses, with `:history` and `!N` to recall inputs; it warns when the compiled build would reject an input and runs it anyway
    Repl {},

    /// Evaluate Windjammer and print the result: `wj eval 'strings.to_upper("hi")'`, `wj eval 'let x = 6; x * 7'`. A `;` or newline inside a string or a block does not split the input.
    /// The common std modules are imported (strings, math, json, time, fs, env, HashMap, HashSet); runs interpreted.
    Eval {
        /// An expression, or statements separated by `;` or newlines whose last one, if an expression, is printed
        #[arg(value_name = "CODE")]
        code: String,
    },

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

    /// Step through a recorded (`wj run --record LOG`) or simulated run deterministically: breakpoints, stepping,
    /// going back, and the values of bindings at any step, from a script of commands (ADR-0035)
    Replay {
        /// `LOG PROGRAM`: an effect log written by `wj run --record` and the program it recorded; with --simulate, only `PROGRAM`
        #[arg(value_name = "LOG PROGRAM", num_args = 1..=2, required = true)]
        paths: Vec<PathBuf>,

        /// Debugger commands, one per line (`break LINE [if EXPR]`, `continue`, `step [N]`, `next`, `finish`, `back [N]`,
        /// `print EXPR`, `watch NAME`, `locals`, `where`); `-` or no script reads them from standard input
        #[arg(long, value_name = "FILE")]
        script: Option<PathBuf>,

        /// Print the events as JSON Lines, one object per line (for agents and scripts)
        #[arg(long)]
        json: bool,

        /// Debug the run of the deterministic simulated world instead of a recorded one (as `wj run --simulate`)
        #[arg(long)]
        simulate: bool,

        /// With --simulate: the seed of the world
        #[arg(long, value_name = "N", default_value = "1", requires = "simulate")]
        seed: u64,

        /// With --simulate: probability (0 to 1) that a file or network call fails
        #[arg(long, value_name = "RATE", default_value = "0", requires = "simulate")]
        faults: f64,

        /// Grant a capability as the recording did, `EFFECT[=RESOURCE]` (repeatable); nothing is performed either way
        #[arg(long = "allow", value_name = "EFFECT[=RESOURCE]")]
        allow: Vec<String>,

        /// Start stopped at the statement that answers the K-th event of the journal (K counts from 1; a recorded run only)
        #[arg(long = "at-effect", value_name = "K", conflicts_with = "simulate", value_parser = clap::value_parser!(u64).range(1..))]
        at_effect: Option<u64>,
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

    /// Migrate deprecated surface syntax (`println!`, `vec![]`, `format!`, ...) to Windjammer forms and add the `mut` of changed locals (WJ0051)
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

        /// Diagnostic format: `human` (default) or `agent`, one compact line per diagnostic (location, code, message,
        /// first help, fixes) for tools and coding agents; `WJ_ERROR_FORMAT=agent` sets it for every command
        #[arg(long, value_name = "FORMAT", value_parser = ["human", "agent"])]
        error_format: Option<String>,
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

    /// Generate the signature surface of a Rust crate dependency (ADR-0026) from its source
    #[command(name = "rust-surface")]
    RustSurface {
        /// Crate name as in wj.toml `[dependencies]`
        #[arg(value_name = "CRATE")]
        crate_name: String,

        /// Only these functions (repeatable); the cache copy always has every function
        #[arg(long = "fn", value_name = "NAME", conflicts_with = "all_pub")]
        functions: Vec<String>,

        /// Every public free function that maps (the default)
        #[arg(long)]
        all_pub: bool,

        /// Write an editable copy to rust/<crate>.wji (source: commit it, it wins over the generated one)
        #[arg(long)]
        write: bool,

        /// Project directory (default: the current directory)
        #[arg(long, value_name = "DIR")]
        path: Option<PathBuf>,
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

        /// Only remove legacy `*.wj.meta` sidecars next to sources under the current directory
        #[arg(long)]
        sidecars: bool,

        /// Remove stale `wj-*` scratch directories from the OS temp dir (dry run unless --yes)
        #[arg(long)]
        tmp: bool,

        /// With --tmp: only directories older than this many hours
        #[arg(long, default_value_t = 6)]
        older_than_hours: u64,

        /// With --tmp: actually delete (default is a dry run)
        #[arg(long)]
        yes: bool,
    },

    /// Manage the shared Cargo target cache (outside application repos)
    Cache {
        #[command(subcommand)]
        action: CacheCommand,
    },

    /// Show this wj's identity: version, commit, commit date, dirty flag, build profile, build date
    Version {
        /// Machine-readable output
        #[arg(long)]
        json: bool,
    },

    /// Find every wj on PATH, flag stale or conflicting builds and a missing shared cache, and print the fixes (exit 1 on problems)
    Doctor {
        /// Machine-readable output
        #[arg(long)]
        json: bool,

        /// Only check that the wj PATH resolves first was built from HEAD of the enclosing compiler checkout (merge gate)
        #[arg(long)]
        expect_head: bool,

        /// Also fail (exit 1) on warnings, such as a wj built from an older commit than the enclosing checkout
        #[arg(long)]
        strict: bool,

        /// Print the toolchain pin line for this wj (its source key), ready to paste under `[toolchain]` in wj.toml
        #[arg(long)]
        pin: bool,
    },

    /// Atomically install this wj (or --from another build) to ~/.cargo/bin/wj, keeping a timestamped backup
    #[command(name = "install-dev")]
    InstallDev {
        /// Destination (default: $CARGO_HOME/bin/wj, else ~/.cargo/bin/wj)
        #[arg(long, value_name = "PATH")]
        to: Option<PathBuf>,

        /// Binary to install (default: the running wj)
        #[arg(long, value_name = "PATH")]
        from: Option<PathBuf>,

        /// Install even if it is older than what is installed
        #[arg(long)]
        force: bool,

        /// Print what would happen without writing anything
        #[arg(long)]
        dry_run: bool,
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

        /// Also delete what a running build holds or wrote within the last hour (normally left alone)
        #[arg(long)]
        force: bool,
    },
    /// Pre-build the runtime for programs that use no network/async code (first compiled build becomes warm)
    Warm,
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

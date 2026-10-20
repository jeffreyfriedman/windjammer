//! CLI port: running files/projects and test files in the interpreter, without cargo or rustc.
//!
//! This is the only module that touches process-level concerns (stdout/stderr text, exit codes,
//! big-stack threads). The evaluator itself stays pure.

use super::engine::Interpreter;
use super::error::{ErrorKind, RuntimeError};
use super::value::Value;
use std::path::{Path, PathBuf};

/// Exit status conventions: 0 ok, 101 panic (like Rust), N for `process::exit(N)`,
/// 2 for "unsupported in the interpreter", 1 for everything else.
pub fn exit_code_for(e: &RuntimeError) -> i32 {
    match &e.kind {
        ErrorKind::Panic => 101,
        ErrorKind::Exit(n) => *n,
        ErrorKind::Unsupported => 2,
        ErrorKind::Semantic => 1,
        ErrorKind::CapabilityDenied => 126,
    }
}

/// Find the entry `.wj` file for a path: a file, or a project directory (`main.wj`, `src/main.wj`, or the
/// default binary of its `wj.toml`). Discovery is `module_system::project`, shared with the compiled `wj run`.
pub fn entry_file(path: &Path) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().unwrap_or_default();
    crate::module_system::project::resolve_run_target(Some(path), &cwd, None).map(|t| t.entry)
}

/// Stack of the interpreter thread; recursion is limited by the bytes actually used (not a frame count),
/// so the limit does not depend on debug/release frame sizes.
/// Name of the big-stack thread; an interpreter created on it bounds recursion by stack bytes.
pub(crate) const BIG_STACK_THREAD: &str = "wj-interpreter";
pub(crate) const STACK_BYTES: usize = 4 << 30;
pub(crate) const STACK_HEADROOM: usize = 256 << 20;

/// Run `f` on a thread with a large stack (deeply recursive programs).
pub fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .name(BIG_STACK_THREAD.to_string())
        .stack_size(STACK_BYTES)
        .spawn(f)
        .expect("spawn interpreter thread")
        .join()
        .unwrap_or_else(|_| panic!("interpreter thread crashed"))
}

/// Knobs of a run (CLI flags): scheduler seed and deterministic time.
#[derive(Clone, Default)]
pub struct RunOptions {
    pub sched_seed: u64,
    /// `--hash-seed N`: permutes HashMap/HashSet iteration order (0 = key order).
    pub hash_seed: u64,
    pub deterministic_time: bool,
    /// `--allow-network`: plain http:// client requests are allowed.
    pub allow_network: bool,
    /// `--mock-http FILE`: canned responses for the http client (takes precedence).
    pub mock_http: Option<PathBuf>,
    /// `--engine=resolved`: compiled closure engine for the functions it supports.
    pub engine_resolved: bool,
    /// `--sandbox`: only `time`, `random` and `console` unless `--allow`ed (ADR-0021).
    pub sandbox: bool,
    /// `--allow EFFECT[=RESOURCE]` (repeatable).
    pub allow: Vec<String>,
    /// `--caps enforce|warn|off` (env `WJ_CAPS`): what the local profile does when no grant is given (default: enforce).
    pub caps_mode: String,
    /// `--record LOG`: run live and write every effect to `LOG` (ADR-0021 E3).
    pub record: Option<PathBuf>,
    /// `--replay LOG`: answer every effect from `LOG`, perform none; seeds and arguments come from the log's header.
    pub replay: Option<PathBuf>,
    /// `--no-jit` / `WJ_JIT=0`: no machine code for hot integer functions (ADR-0020).
    pub no_jit: bool,
}

/// Execute a program; program stdout goes to stdout, diagnostics to stderr. Returns the exit code.
pub fn run_path(path: &Path, args: Vec<String>) -> i32 {
    run_path_with(path, args, RunOptions::default())
}

pub fn run_path_with(path: &Path, args: Vec<String>, opts: RunOptions) -> i32 {
    let entry = match entry_file(path) {
        Ok(e) => e,
        Err(m) => {
            eprintln!("error: {}", m);
            return 1;
        }
    };
    match run_entry(&entry, args, EntryOptions { run: opts, ..EntryOptions::default() }) {
        RunOutcome::Exited(code) => code,
        RunOutcome::NeedsCompiled(_) => unreachable!("fallback disabled"),
    }
}

/// Result of an interpreted run.
#[derive(Debug)]
pub enum RunOutcome {
    /// The program ran (or failed) in the interpreter; this is its exit status.
    Exited(i32),
    /// The program uses a registry-unsupported feature, detected at load time before anything ran
    /// (see `preflight.rs`); the caller should run it compiled. Only returned when `fallback` is set.
    NeedsCompiled(RuntimeError),
}

/// How `run_entry` runs a program.
#[derive(Default)]
pub struct EntryOptions {
    /// Return [`RunOutcome::NeedsCompiled`] on a preflight hit instead of printing the error.
    pub fallback: bool,
    /// Bridge library and the `extern fn`s it provides (ADR-0023), built by the CLI.
    pub host: Option<(PathBuf, Vec<super::host_bridge::ExternSig>)>,
    /// Apply source edits to the running program (ADR-0024).
    pub hot: bool,
    /// Scheduler seed, deterministic time, network policy.
    pub run: RunOptions,
}

/// Load and run `entry` in the interpreter.
pub fn run_entry(entry: &Path, args: Vec<String>, opts: EntryOptions) -> RunOutcome {
    let entry = entry.to_path_buf();
    with_big_stack(move || {
        let EntryOptions { fallback, host, hot, run: opts } = opts;
        let mut it = Interpreter::new();
        it.guard_stack();
        it.hash_seed = opts.hash_seed;
        it.set_sched_seed(opts.sched_seed);
        it.set_deterministic_time(opts.deterministic_time);
        it.set_engine_resolved(opts.engine_resolved);
        it.set_jit(!opts.no_jit);
        if let Some(p) = &opts.mock_http {
            match super::builtins::std_http::HttpMode::mock_from_file(p) {
                Ok(m) => it.set_http_mode(m),
                Err(e) => {
                    eprintln!("error: {e}");
                    return RunOutcome::Exited(1);
                }
            }
        } else if opts.allow_network {
            it.set_http_mode(super::builtins::std_http::HttpMode::Real);
        }
        match super::caps::gate_for_run(&entry, opts.sandbox, &opts.allow, opts.allow_network, &opts.caps_mode) {
            Ok(gate) => it.set_caps(gate),
            Err(m) => {
                eprintln!("error: {m}");
                return RunOutcome::Exited(1);
            }
        }
        // a net.connect grant (wj.toml or --allow) turns the http port on; the grant limits which hosts
        if opts.mock_http.is_none() && !opts.allow_network && it.caps_grant_network() {
            it.set_http_mode(super::builtins::std_http::HttpMode::Real);
        }
        it.set_args(args);
        if let Err(m) = setup_effect_port(&mut it, &opts, &entry) {
            eprintln!("error: {m}");
            return RunOutcome::Exited(1);
        }
        if let Some((lib, sigs)) = host {
            match super::host_bridge::HostLib::open(&lib, &sigs) {
                Ok(h) => it.attach_host(h),
                Err(m) => {
                    eprintln!("error: {}", m);
                    return RunOutcome::Exited(1);
                }
            }
        }
        if let Err(e) = it.load_program_at(&entry) {
            if fallback && super::preflight::is_preflight(&e) {
                return RunOutcome::NeedsCompiled(e);
            }
            eprintln!("{}", e.render());
            return RunOutcome::Exited(exit_code_for(&e));
        }
        if hot {
            it.enable_hot_reload();
        }
        let code = match it.call_main() {
            Ok(_) => 0,
            Err(e) => {
                if !matches!(e.kind, ErrorKind::Exit(_)) {
                    eprintln!("{}", e.render());
                }
                exit_code_for(&e)
            }
        };
        it.flush();
        RunOutcome::Exited(code)
    })
}

/// `--record` / `--replay`: install the effect port. A replay takes the seeds, the time mode and the arguments from the
/// log's header, because they are part of what the recorded run was a function of.
pub fn setup_effect_port(it: &mut Interpreter, opts: &RunOptions, entry: &Path) -> Result<(), String> {
    use super::effect_port::{log, EffectPort};
    use rand::{RngCore, SeedableRng};
    if opts.record.is_some() && opts.replay.is_some() {
        return Err("--record and --replay cannot be used together".into());
    }
    let program = log::digest(&std::fs::read(entry).map_err(|e| format!("cannot read {}: {e}", entry.display()))?);
    if let Some(path) = &opts.record {
        let rng_seed = rand::rngs::OsRng.next_u64();
        let header = log::Header {
            program,
            compiler: env!("CARGO_PKG_VERSION").to_string(),
            sched_seed: opts.sched_seed,
            sim_seed: None,
            hash_seed: opts.hash_seed,
            rng_seed,
            deterministic_time: opts.deterministic_time,
            args: it.args.clone(),
        };
        it.rng = Some(rand::rngs::StdRng::seed_from_u64(rng_seed));
        it.set_port(EffectPort::record(path, header).map_err(|e| format!("cannot write {}: {e}", path.display()))?);
    } else if let Some(path) = &opts.replay {
        let port = EffectPort::replay(path, &program)?;
        let h = port.header.clone();
        it.hash_seed = h.hash_seed;
        it.set_sched_seed(h.sched_seed);
        it.set_deterministic_time(h.deterministic_time);
        it.set_args(h.args);
        it.rng = Some(rand::rngs::StdRng::seed_from_u64(h.rng_seed));
        it.set_port(port);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub file: PathBuf,
    pub name: String,
    pub passed: bool,
    pub message: String,
}

/// One test to run, as discovered by `wj test`'s shared discovery (`test_runner::test_discovery`).
#[derive(Debug, Clone)]
pub struct TestSpec {
    pub file: PathBuf,
    /// The function name.
    pub name: String,
    /// Name shown in reports (`mod::name` / `Type::name`), identical to the compiled listing.
    pub display_name: String,
    /// Inline `mod` path the function lives in (empty at the top level).
    pub mods: Vec<String>,
    /// `Some(type)` for a receiver-less associated function `Type::name()`.
    pub assoc: Option<String>,
}

/// Run the given tests (each against a fresh interpreter, files loaded once per test).
pub fn run_specs(specs: Vec<TestSpec>) -> Vec<TestResult> {
    with_big_stack(move || {
        let mut results = Vec::new();
        for spec in specs {
            let mut it = Interpreter::new_capturing();
            it.guard_stack();
            // the resolved engine, as `wj run` (ADR-0020); the walker runs what it does not compile
            it.set_engine_resolved(true);
            // tests are sandboxed like `wj run` (ADR-0021): the project's [capabilities], else the local profile
            let scratch = super::caps::Scratch::new();
            match super::caps::gate_for_run(&spec.file, false, &[], false, "") {
                Ok(gate) => it.set_caps(gate.map(|g| {
                    let g = g.for_tests();
                    match &scratch {
                        Some(s) => g.with_scratch(s.path()),
                        None => g,
                    }
                })),
                Err(m) => {
                    results.push(TestResult { file: spec.file, name: spec.display_name, passed: false, message: m });
                    continue;
                }
            }
            let outcome = match it.load_test_program(&spec.file) {
                Ok(mut path) => {
                    path.extend(spec.mods.iter().cloned());
                    it.run_test(&path, spec.assoc.as_deref(), &spec.name)
                }
                Err(e) => Err(e),
            };
            let (passed, message) = match outcome {
                Ok(()) => (true, String::new()),
                Err(e) => (false, e.render()),
            };
            results.push(TestResult { file: spec.file, name: spec.display_name, passed, message });
        }
        results
    })
}

/// Print a cargo-test-like report. Returns the process exit code.
pub fn report_tests(results: &[TestResult]) -> i32 {
    for r in results {
        println!("test {}::{} ... {}", r.file.display(), r.name, if r.passed { "ok" } else { "FAILED" });
    }
    let failed: Vec<_> = results.iter().filter(|r| !r.passed).collect();
    for r in &failed {
        println!("\n---- {} ----\n{}", r.name, r.message);
    }
    println!(
        "\ntest result: {}. {} passed; {} failed (interpreted, no cargo)",
        if failed.is_empty() { "ok" } else { "FAILED" },
        results.len() - failed.len(),
        failed.len()
    );
    if failed.is_empty() { 0 } else { 1 }
}

impl Interpreter {
    /// Run one test function (all cases for `@test_cases`).
    pub fn run_test(&mut self, mods: &[String], assoc: Option<&str>, name: &str) -> Result<(), RuntimeError> {
        let mut module = 0;
        for m in mods {
            module = *self.modules[module]
                .children
                .get(m)
                .ok_or_else(|| RuntimeError::new(ErrorKind::Semantic, format!("no module {} for test {}", m, name)))?;
        }
        let found = match assoc {
            Some(ty) => self.methods.get(&(super::register::base_type_name(ty), name.to_string())).cloned(),
            None => self.modules[module].funcs.get(name).cloned(),
        };
        let f = found.ok_or_else(|| RuntimeError::new(ErrorKind::Semantic, format!("no test named {}", name)))?;
        if f.decorators.iter().any(|d| d.name == "property_test") {
            return Err(RuntimeError::new(ErrorKind::Unsupported, "`@property_test` (generated inputs) is not interpreted; use `wj test`"));
        }
        let cases = f.decorators.iter().find(|d| d.name == "test_cases");
        let mut calls: Vec<Vec<Value>> = Vec::new();
        if let Some(d) = cases {
            for (_, e) in &d.arguments {
                let v = self.eval(e).map_err(|c| self.ctl_to_error(c))?;
                for case in self.to_items(v).map_err(|c| self.ctl_to_error(c))? {
                    calls.push(match case {
                        Value::Tuple(t) => t.to_vec(),
                        other => vec![other],
                    });
                }
            }
        } else {
            calls.push(Vec::new());
        }
        for args in calls {
            let r = self.call_func(&f, args, None);
            self.finish(r)?;
        }
        Ok(())
    }

    pub(crate) fn ctl_to_error(&mut self, c: super::error::Ctl) -> RuntimeError {
        match c {
            super::error::Ctl::Err(e) => self.annotate(*e),
            _ => RuntimeError::new(ErrorKind::Semantic, "unexpected control flow"),
        }
    }
}

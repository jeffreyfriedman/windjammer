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
    }
}

/// Find the entry `.wj` file for a path: a file, or a project directory (`main.wj` / `src/main.wj`).
pub fn entry_file(path: &Path) -> Result<PathBuf, String> {
    if path.is_file() {
        return Ok(path.to_path_buf());
    }
    if path.is_dir() {
        for cand in ["main.wj", "src/main.wj", "src/lib.wj"] {
            let p = path.join(cand);
            if p.is_file() {
                return Ok(p);
            }
        }
        return Err(format!("no main.wj or src/main.wj found in {}", path.display()));
    }
    Err(format!("File not found: {}", path.display()))
}

/// Stack of the interpreter thread; recursion is limited by the bytes actually used (not a frame count),
/// so the limit does not depend on debug/release frame sizes.
pub(crate) const STACK_BYTES: usize = 4 << 30;
pub(crate) const STACK_HEADROOM: usize = 256 << 20;

/// Run `f` on a thread with a large stack (deeply recursive programs).
pub fn with_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(f)
        .expect("spawn interpreter thread")
        .join()
        .unwrap_or_else(|_| panic!("interpreter thread crashed"))
}

/// Execute a program; program stdout goes to stdout, diagnostics to stderr. Returns the exit code.
pub fn run_path(path: &Path, args: Vec<String>) -> i32 {
    let entry = match entry_file(path) {
        Ok(e) => e,
        Err(m) => {
            eprintln!("error: {}", m);
            return 1;
        }
    };
    with_big_stack(move || {
        let mut it = Interpreter::new();
        it.guard_stack();
        it.set_args(args);
        if let Err(e) = it.load_entry_file(&entry) {
            eprintln!("{}", e.render());
            return exit_code_for(&e);
        }
        match it.call_main() {
            Ok(_) => 0,
            Err(e) => {
                if !matches!(e.kind, ErrorKind::Exit(_)) {
                    eprintln!("{}", e.render());
                }
                exit_code_for(&e)
            }
        }
    })
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub file: PathBuf,
    pub name: String,
    pub passed: bool,
    pub message: String,
}

fn is_test_file(p: &Path) -> bool {
    p.extension().map(|e| e == "wj").unwrap_or(false)
        && p.file_stem().map(|s| s.to_string_lossy().ends_with("_test")).unwrap_or(false)
}

pub fn discover_test_files(path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if !n.starts_with('.') && n != "target" && n != "build" {
                    walk(&p, out);
                }
            } else if is_test_file(&p) {
                out.push(p);
            }
        }
    }
    if path.is_file() {
        out.push(path.to_path_buf());
    } else {
        walk(path, &mut out);
    }
    out
}

/// Run every `@test` / `test_*` function in the given test files. Each test sees a fresh interpreter.
pub fn run_tests(path: &Path, filter: Option<&str>) -> Vec<TestResult> {
    let files = discover_test_files(path);
    let filter = filter.map(|s| s.to_string());
    with_big_stack(move || {
        let mut results = Vec::new();
        for f in files {
            // names are discovered once, then each test runs against its own interpreter
            let mut probe = Interpreter::new_capturing();
            probe.guard_stack();
            if let Err(e) = probe.load_entry_file(&f) {
                results.push(TestResult { file: f.clone(), name: "<load>".into(), passed: false, message: e.render() });
                continue;
            }
            let names = probe.test_function_names();
            for name in names {
                if let Some(flt) = &filter {
                    if !name.contains(flt.as_str()) {
                        continue;
                    }
                }
                let mut it = Interpreter::new_capturing();
                it.guard_stack();
                if it.load_entry_file(&f).is_err() {
                    continue;
                }
                match it.run_test(&name) {
                    Ok(()) => results.push(TestResult { file: f.clone(), name, passed: true, message: String::new() }),
                    Err(e) => results.push(TestResult { file: f.clone(), name, passed: false, message: e.render() }),
                }
            }
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
    /// Names of test functions in the root module: `@test`, `@test_cases(...)` or `test_*`.
    pub fn test_function_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.modules[0]
            .funcs
            .values()
            .filter(|f| {
                f.decorators.iter().any(|d| d.name == "test" || d.name == "test_cases") || f.name.starts_with("test_")
            })
            .filter(|f| f.name != "main")
            .map(|f| f.name.clone())
            .collect();
        names.sort();
        names
    }

    /// Run one test function (all cases for `@test_cases`).
    pub fn run_test(&mut self, name: &str) -> Result<(), RuntimeError> {
        let f = self.modules[0]
            .funcs
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(ErrorKind::Semantic, format!("no test named {}", name)))?;
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

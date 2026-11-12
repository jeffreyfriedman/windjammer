#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Runtime errors explain themselves (ADR-0036): a failed interpreted run names the failing expression, the
//! Windjammer call stack and the bindings of the failing frame, in text and in the diagnostics schema as JSON.
//!
//! The oracle for the values is the program text (the arguments written in `main`), not the interpreter's own
//! formatting. The parity tests pin that the extra detail never changes stdout or the exit status.
//! Regenerate goldens with `UPDATE_GOLDEN=1 cargo test --test runtime_error_report_test`.

use serde_json::Value;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Run {
    stdout: String,
    stderr: String,
    code: i32,
}

fn run_in(dir: &std::path::Path, file: &str, flags: &[&str], env: &[(&str, &str)]) -> Run {
    let mut c = Command::new(WJ);
    c.current_dir(dir).args(["run", "--interpret", "--no-lint"]).args(flags).arg(file);
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output().unwrap();
    Run { stdout: String::from_utf8_lossy(&o.stdout).into(), stderr: String::from_utf8_lossy(&o.stderr).into(), code: o.status.code().unwrap_or(-1) }
}

/// Run `src` as `p.wj` in a scratch directory.
fn run(src: &str, flags: &[&str]) -> Run {
    run_env(src, flags, &[])
}

fn run_env(src: &str, flags: &[&str], env: &[(&str, &str)]) -> Run {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("p.wj"), src).unwrap();
    run_in(dir.path(), "p.wj", flags, env)
}

/// `main` -> `outer(7)` -> `inner(7, "calc")`, where `inner` fails with `fail`.
fn program(fail: &str) -> String {
    format!(
        "fn inner(n: int, label: string) -> int {{\n    let items = [10, 20, 30]\n    let doubled = n * 2\n    {fail}\n}}\n\nfn outer(n: int) -> int {{\n    inner(n, \"calc\") + 1\n}}\n\nfn main() {{\n    println(\"start\")\n    println(\"${{outer(7)}}\")\n}}\n"
    )
}

/// (name, failing statement, text the message must contain)
const FAILURES: &[(&str, &str, &str)] = &[
    ("index", "items[n]", "index out of bounds"),
    ("division", "items[0] / (n - 7)", "divide by zero"),
    ("overflow", "let big = 250_u8\n    (big + (n as u8)) as int", "overflow"),
    ("unwrap", "let o: Option<int> = None\n    o.unwrap()", "None"),
    ("expect", "let o: Option<int> = None\n    o.expect(\"need a value\")", "need a value"),
    ("panic", "panic(\"boom\")", "boom"),
    ("assert", "assert_eq(doubled, 15)", "left: 14"),
];

fn engines() -> Vec<Vec<&'static str>> {
    vec![vec![], vec!["--engine", "walker"], vec!["--no-jit"]]
}

#[test]
fn every_failure_kind_shows_stack_and_locals_in_every_engine() {
    for (name, fail, msg) in FAILURES {
        for flags in engines() {
            let r = run(&program(fail), &flags);
            let ctx = format!("{name} {flags:?}\n{}", r.stderr);
            assert_eq!(r.code, 101, "{ctx}");
            assert_eq!(r.stdout, "start\n", "{ctx}");
            assert!(r.stderr.contains(msg), "message: {ctx}");
            assert!(r.stderr.contains("0: `inner` at p.wj:"), "innermost frame: {ctx}");
            assert!(r.stderr.contains("1: `outer` at p.wj:"), "caller frame: {ctx}");
            assert!(r.stderr.contains("2: `main` at p.wj:"), "main frame: {ctx}");
            assert!(r.stderr.contains("locals in `inner`:"), "locals header: {ctx}");
            assert!(r.stderr.contains("  n = 7"), "parameter value: {ctx}");
            assert!(r.stderr.contains("  label = \"calc\""), "string parameter: {ctx}");
            assert!(r.stderr.contains("  items = [10, 20, 30]"), "vec local: {ctx}");
        }
    }
}

#[test]
fn the_primary_span_is_the_failing_line_with_a_caret() {
    let r = run(&program("items[n]"), &[]);
    assert!(r.stderr.contains("panicked at p.wj:4:"), "{}", r.stderr);
    assert!(r.stderr.contains(" 4 |     items[n]"), "source excerpt: {}", r.stderr);
}

#[test]
fn json_is_a_diagnostics_report_with_stack_and_locals() {
    for flags in engines() {
        let mut f = vec!["--error-format", "json"];
        f.extend(flags.iter());
        let r = run(&program("items[n]"), &f);
        assert_eq!(r.code, 101);
        let line = r.stderr.lines().last().unwrap();
        let v: Value = serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {}", r.stderr));
        assert_eq!(v["schema_version"], 2);
        assert_eq!(v["command"], "run");
        assert_eq!(v["success"], false);
        assert_eq!(v["exit_code"], 101);
        assert_eq!(v["summary"]["errors"], 1);
        let d = &v["diagnostics"][0];
        assert_eq!(d["code"], "WJ0170");
        assert_eq!(d["severity"], "error");
        assert_eq!(d["category"], "runtime");
        assert!(d["message"].as_str().unwrap().contains("index out of bounds"));
        assert_eq!(d["primary_span"]["line_start"], 4);
        assert!(d["primary_span"]["file"].as_str().unwrap().ends_with("p.wj"));
        assert!(d["primary_span"]["byte_end"].as_u64() > d["primary_span"]["byte_start"].as_u64());
        let stack: Vec<&str> = d["stack"].as_array().unwrap().iter().map(|f| f["function"].as_str().unwrap()).collect();
        assert_eq!(stack, ["inner", "outer", "main"], "{flags:?}");
        assert_eq!(d["stack"][1]["span"]["line_start"], 8, "the call of inner is on line 8");
        let locals = d["locals"].as_array().unwrap();
        let get = |n: &str| locals.iter().find(|l| l["name"] == n).unwrap_or_else(|| panic!("no local {n}: {locals:?}"))["value"].clone();
        assert_eq!(get("n"), "7");
        assert_eq!(get("items"), "[10, 20, 30]");
        assert_eq!(get("doubled"), "14");
    }
}

#[test]
fn locals_that_look_like_secrets_are_redacted_by_name_and_by_origin() {
    let src = "fn login(password: string, user: string, key: string) -> int {\n    let items = [1]\n    items[3]\n}\n\nfn main() {\n    println(\"go\")\n    println(\"${login(\"hunter2\", \"ada\", \"s3cr3t-from-env-0042\")}\")\n}\n";
    for flags in [vec![], vec!["--error-format", "json"], vec!["--engine", "walker"]] {
        let r = run_env(src, &flags, &[("WJ_TEST_API_CREDENTIAL", "s3cr3t-from-env-0042")]);
        assert_eq!(r.code, 101);
        assert!(!r.stderr.contains("hunter2"), "password leaked ({flags:?}): {}", r.stderr);
        assert!(!r.stderr.contains("s3cr3t-from-env-0042"), "env value leaked ({flags:?}): {}", r.stderr);
        assert!(r.stderr.contains("ada"), "a harmless local stays visible ({flags:?}): {}", r.stderr);
        assert!(r.stderr.contains("<redacted>"));
    }
}

#[test]
fn no_locals_drops_the_bindings_but_keeps_the_stack() {
    for flags in [vec!["--no-locals"], vec!["--no-locals", "--engine", "walker"]] {
        let r = run(&program("items[n]"), &flags);
        assert_eq!(r.code, 101);
        assert!(!r.stderr.contains("locals in"), "{}", r.stderr);
        assert!(!r.stderr.contains("[10, 20, 30]"), "{}", r.stderr);
        assert!(r.stderr.contains("0: `inner` at p.wj:"), "{}", r.stderr);
    }
    let r = run(&program("items[n]"), &["--no-locals", "--error-format", "json"]);
    let v: Value = serde_json::from_str(r.stderr.lines().last().unwrap()).unwrap();
    assert_eq!(v["diagnostics"][0]["locals"].as_array().unwrap().len(), 0);
    assert_eq!(v["diagnostics"][0]["stack"].as_array().unwrap().len(), 3);
}

/// The gate of ADR-0036: the extra detail is stderr only. stdout and the exit status are those of a plain run,
/// and the Rust-shaped head of the message (up to `stack backtrace:`) is identical.
#[test]
fn the_detail_never_changes_stdout_or_the_exit_status() {
    for (name, fail, _) in FAILURES {
        let src = program(fail);
        let plain = run(&src, &["--no-locals"]);
        for flags in [vec![], vec!["--error-format", "json"], vec!["--error-format", "json", "--no-locals"]] {
            let r = run(&src, &flags);
            assert_eq!((&r.stdout, r.code), (&plain.stdout, plain.code), "{name} {flags:?}");
        }
        let rich = run(&src, &[]);
        let head = |s: &str| s.split("stack backtrace:").next().unwrap().to_string();
        assert_eq!(head(&rich.stderr).replace(['\r'], ""), head(&plain.stderr), "{name}: head of stderr");
    }
    // a clean run is unaffected too
    let ok = run("fn main() {\n    println(\"fine\")\n}\n", &["--error-format", "json"]);
    assert_eq!((ok.stdout.as_str(), ok.code, ok.stderr.as_str()), ("fine\n", 0, ""));
}

#[test]
fn main_returning_err_is_reported_as_json_and_unchanged_as_text() {
    let src = "fn main() -> Result<(), string> {\n    println(\"before\")\n    Err(\"no input\")\n}\n";
    let text = run(src, &[]);
    assert_eq!((text.stdout.as_str(), text.code), ("before\n", 1));
    assert!(text.stderr.contains("Error: \"no input\""), "{}", text.stderr);
    let j = run(src, &["--error-format", "json"]);
    assert_eq!((j.stdout.as_str(), j.code), ("before\n", 1));
    let v: Value = serde_json::from_str(j.stderr.lines().last().unwrap()).unwrap();
    assert_eq!(v["diagnostics"][0]["code"], "WJ0171");
    assert_eq!(v["exit_code"], 1);
    assert!(v["diagnostics"][0]["message"].as_str().unwrap().contains("no input"));
}

/// 1-based line of the first line of `src` that contains `needle`.
fn line_of(src: &str, needle: &str) -> u64 {
    src.lines().position(|l| l.contains(needle)).map(|i| i as u64 + 1).unwrap_or_else(|| panic!("{needle} not in source"))
}

fn json_report(r: &Run) -> Value {
    serde_json::from_str(r.stderr.lines().last().unwrap()).unwrap()
}

#[test]
fn main_err_from_try_has_the_span_of_the_try_and_the_locals() {
    let src = "fn load(n: int) -> Result<int, string> {\n    if n < 0 {\n        return Err(\"negative\")\n    }\n    Ok(n)\n}\n\nfn main() -> Result<(), string> {\n    let a = load(2)?\n    println(\"a = ${a}\")\n    let b = load(-1)?\n    println(\"b = ${b}\")\n    Ok(())\n}\n";
    let want = line_of(src, "load(-1)?");
    for engine in ["walker", "resolved"] {
        let j = run(src, &["--error-format", "json", "--engine", engine]);
        assert_eq!(j.code, 1, "{engine}: {}", j.stderr);
        let d = &json_report(&j)["diagnostics"][0];
        assert_eq!(d["code"], "WJ0171", "{engine}");
        assert_eq!(d["primary_span"]["line_start"], want, "{engine}: {d}");
        assert_eq!(d["stack"][0]["span"]["line_start"], want, "{engine}: {d}");
        let locals = d["locals"].as_array().unwrap();
        let a = locals.iter().find(|l| l["name"] == "a").unwrap_or_else(|| panic!("{engine}: no local `a` in {d}"));
        assert_eq!(a["value"], "2", "{engine}");
        assert!(locals.iter().all(|l| l["name"] != "b"), "{engine}: `b` was never bound: {d}");
    }
}

#[test]
fn main_err_from_a_tail_err_has_the_span_of_that_expression() {
    let src = "fn main() -> Result<(), string> {\n    let path = \"in.txt\"\n    println(\"reading ${path}\")\n    Err(\"no input\")\n}\n";
    let want = line_of(src, "Err(\"no input\")");
    for engine in ["walker", "resolved"] {
        let j = run(src, &["--error-format", "json", "--engine", engine]);
        assert_eq!(j.code, 1, "{engine}: {}", j.stderr);
        let d = &json_report(&j)["diagnostics"][0];
        assert_eq!(d["primary_span"]["line_start"], want, "{engine}: {d}");
        assert!(d["locals"].as_array().unwrap().iter().any(|l| l["name"] == "path" && l["value"] == "\"in.txt\""), "{engine}: {d}");
    }
}

#[test]
fn an_unknown_error_format_is_refused() {
    let r = run("fn main() {}\n", &["--error-format", "xml"]);
    assert_ne!(r.code, 0);
    assert!(r.stderr.contains("human") && r.stderr.contains("json"), "{}", r.stderr);
}

// ---------------------------------------------------------------- goldens

fn golden_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime_error_golden")
}

fn check_golden(name: &str, ext: &str, got: &str) {
    let p = golden_dir().join(format!("{name}.{ext}"));
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&p, got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&p).unwrap_or_else(|_| panic!("missing golden {}; run with UPDATE_GOLDEN=1", p.display()));
    assert_eq!(got, want, "golden {name}.{ext} differs (UPDATE_GOLDEN=1 regenerates)");
}

#[test]
fn goldens() {
    for name in ["index_out_of_bounds", "assert_eq_sides", "division_by_zero", "main_err"] {
        let file = format!("{name}.wj");
        let human = run_in(&golden_dir(), &file, &[], &[]);
        check_golden(name, "stderr", &human.stderr);
        let json = run_in(&golden_dir(), &file, &["--error-format", "json"], &[]);
        let mut v: Value = serde_json::from_str(json.stderr.lines().last().unwrap()).unwrap();
        v["windjammer_version"] = Value::String("X".into());
        check_golden(name, "json", &(serde_json::to_string_pretty(&v).unwrap() + "\n"));
    }
}

/// The bindings of a failing frame are the same whichever engine ran it, including machine code that bailed out
/// and was re-run in the closure engine (ADR-0036): a binding of a block that already ended is not among them.
#[test]
fn locals_match_the_walkers_across_engines_and_the_jit() {
    let src = "fn k(n: int) -> int {\n    let a = n + 1\n    let b = {\n        let t = n * 2\n        t + 1\n    }\n    let mut acc = 0\n    if a > 0 {\n        let u = a * 2\n        acc = acc + u\n    }\n    for q in 0..3 {\n        let w = q + n\n        acc = acc + w\n    }\n    acc + 100 / (b - 15) + a\n}\n\nfn main() {\n    let mut j = 0\n    let mut tot = 0\n    while j < 200 {\n        tot = tot + k(j + 100)\n        j = j + 1\n    }\n    println(\"${tot}\")\n    println(\"${k(7)}\")\n}\n";
    let names = |flags: &[&str]| -> Vec<String> {
        let r = run(src, flags);
        assert_eq!(r.code, 101, "{flags:?}: {}", r.stderr);
        let d = &json_report(&r)["diagnostics"][0];
        d["locals"].as_array().unwrap().iter().map(|l| format!("{}={}", l["name"].as_str().unwrap(), l["value"].as_str().unwrap())).collect()
    };
    let walker = names(&["--error-format", "json", "--engine", "walker"]);
    for t in ["t", "u", "w", "q"] {
        assert!(walker.iter().all(|l| !l.starts_with(&format!("{t}="))), "walker shows `{t}`: {walker:?}");
    }
    for flags in [
        vec!["--error-format", "json", "--engine", "resolved", "--no-jit"],
        vec!["--error-format", "json", "--engine", "resolved"],
    ] {
        assert_eq!(names(&flags), walker, "{flags:?}");
    }
}

#![cfg(feature = "jit")]

//! The Cranelift tier never runs a sink (ADR-0020 step 7, ADR-0021): only functions whose every
//! value is an `int` and whose calls go to such functions are jitted, so a function that can reach
//! an effect -- here `fs::exists` and `println`, taken only after the function is hot -- keeps
//! running in the engine that consults the capability gate. With `--caps enforce` the late sink call
//! is denied exactly as without the jit.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SRC: &str = r#"use std::fs
fn pure(n: int) -> int {
    n * 3 + 1
}
fn step(n: int) -> int {
    let x = pure(n) % 7
    if n == 150 {
        println("late print ${x}")
    }
    if n == 200 {
        if fs::exists(".") { return 1 }
    }
    x
}
fn main() {
    let mut total = 0
    let mut i = 0
    while i < 300 {
        total += step(i)
        i += 1
    }
    println("total ${total}")
}
"#;

fn run(dir: &std::path::Path, extra: &[&str]) -> (String, String, Option<i32>) {
    let p = dir.join("m.wj");
    std::fs::write(&p, SRC).unwrap();
    let o = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).args(extra).arg(&p).current_dir(dir).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned(), o.status.code())
}

#[test]
fn a_function_reaching_a_sink_is_never_jitted() {
    let tmp = tempfile::TempDir::new().unwrap();
    let jit = run(tmp.path(), &["--caps", "enforce", "--sandbox"]);
    let nojit = run(tmp.path(), &["--caps", "enforce", "--sandbox", "--no-jit"]);
    // the late print ran (the function was not replaced by machine code) and the late sink was denied
    assert!(jit.0.contains("late print"), "stdout: {}\nstderr: {}", jit.0, jit.1);
    assert!(jit.1.contains("capability denied"), "stderr: {}", jit.1);
    assert_eq!(jit.2, Some(126));
    assert_eq!(jit, nojit, "the jit changed the outcome");
}

#[test]
fn wj_jit_0_and_no_jit_agree_with_the_default() {
    let tmp = tempfile::TempDir::new().unwrap();
    let p = tmp.path().join("fib.wj");
    std::fs::write(&p, "fn fib(n: int) -> int {\n    if n < 2 { return n }\n    fib(n - 1) + fib(n - 2)\n}\nfn main() {\n    println(\"${fib(25)}\")\n}\n").unwrap();
    let out = |env: Option<&str>, flag: Option<&str>| {
        let mut c = Command::new(WJ);
        c.args(["run", "--interpret", "--no-lint"]);
        if let Some(f) = flag {
            c.arg(f);
        }
        c.arg(&p);
        match env {
            Some(v) => c.env("WJ_JIT", v),
            None => c.env_remove("WJ_JIT"),
        };
        let o = c.output().unwrap();
        (String::from_utf8_lossy(&o.stdout).into_owned(), o.status.code())
    };
    let base = out(None, None);
    assert_eq!(base, ("75025\n".to_string(), Some(0)));
    assert_eq!(out(Some("0"), None), base);
    assert_eq!(out(Some("off"), None), base);
    assert_eq!(out(None, Some("--no-jit")), base);
    let bad = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).arg(&p).env("WJ_JIT", "maybe").output().unwrap();
    assert_ne!(bad.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("WJ_JIT"));
}

/// Machine-code loops (ADR-0020 round 3) never run a sink either: a loop whose body can reach one is
/// outside the lowering subset (it has no calls), so it stays in the closure engine and the capability
/// gate sees the late sink. Every loop tries machine code at its first iteration here.
#[test]
fn a_loop_reaching_a_sink_stays_in_the_closures() {
    let tmp = tempfile::TempDir::new().unwrap();
    let src = "use std::fs\nfn main() {\n    let mut total = 0\n    let mut i = 0\n    while i < 3000 {\n        total += i % 7\n        if i == 2500 {\n            println(\"late print ${total}\")\n        }\n        if i == 2900 {\n            if fs::exists(\".\") { total += 1 }\n        }\n        i += 1\n    }\n    let mut j = 0\n    while j < 3000 {\n        total += j % 3\n        j += 1\n    }\n    println(\"total ${total}\")\n}\n";
    let p = tmp.path().join("loop.wj");
    std::fs::write(&p, src).unwrap();
    let run = |extra: &[&str], debug: bool| {
        let mut c = Command::new(WJ);
        c.args(["run", "--interpret", "--no-lint", "--caps", "enforce", "--sandbox"]).args(extra).arg(&p).current_dir(tmp.path());
        c.env("WJ_JIT_LOOP_HOT", "0").env_remove("WJ_JIT");
        if debug {
            c.env("WJ_JIT_DEBUG", "1");
        }
        let o = c.output().unwrap();
        (String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned(), o.status.code())
    };
    let jit = run(&[], false);
    let nojit = run(&["--no-jit"], false);
    assert!(jit.0.contains("late print"), "stdout: {}\nstderr: {}", jit.0, jit.1);
    assert!(jit.1.contains("capability denied"), "stderr: {}", jit.1);
    assert_eq!(jit.2, Some(126));
    assert_eq!(jit, nojit, "machine code changed the outcome");
    // the sink loop was refused; with the sink gone the same shape runs as machine code
    let dbg = run(&[], true).1;
    assert!(dbg.lines().any(|l| l.starts_with("[jit] loop") && l.contains("loop.wj:5") && l.contains("outside the subset")), "{dbg}");
    let pure = src.replace("            println(\"late print ${total}\")\n", "            total += 2\n").replace("            if fs::exists(\".\") { total += 1 }\n", "            total += 3\n");
    std::fs::write(&p, pure).unwrap();
    let dbg = run(&[], true).1;
    assert!(dbg.lines().any(|l| l.starts_with("[jit] loop") && l.contains("loop.wj:5: compiled")), "{dbg}");
}

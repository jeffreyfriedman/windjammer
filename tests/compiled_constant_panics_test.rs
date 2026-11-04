//! A constant expression that overflows or divides by zero panics at run time (SPEC-NUM-002..006); the
//! generated crate must not be rejected by rustc's deny-by-default `arithmetic_overflow` and
//! `unconditional_panic` lints. Independent oracle: the specified outcome (stdout so far, exit status 101,
//! a message containing `overflow` / `zero`).

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn run_compiled(body: &str) -> (String, String, Option<i32>) {
    let d = Scratch::new("const-panic");
    let f = d.join("prog.wj");
    std::fs::write(&f, format!("fn main() {{\n{body}\n}}\n")).unwrap();
    let o = Command::new(WJ).arg("run").arg(&f).current_dir(d.path()).output().unwrap();
    (String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string(), o.status.code())
}

#[test]
fn constant_overflow_builds_and_panics_at_run_time() {
    let (out, err, code) = run_compiled("    let a = 9223372036854775807\n    println(\"before\")\n    let b = a + 1\n    println(\"${b}\")");
    assert!(!err.contains("this arithmetic operation will overflow"), "rustc rejected the program:\n{err}");
    assert_eq!(out, "before\n", "{err}");
    assert_eq!(code, Some(101), "{err}");
    assert!(err.contains("overflow"), "{err}");
}

#[test]
fn constant_remainder_by_zero_builds_and_panics_at_run_time() {
    let (out, err, code) = run_compiled("    println(\"before\")\n    let r = 10 % 0\n    println(\"${r}\")");
    assert!(!err.contains("this operation will panic at runtime"), "rustc rejected the program:\n{err}");
    assert_eq!(out, "before\n", "{err}");
    assert_eq!(code, Some(101), "{err}");
}

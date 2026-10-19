#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]
//! `..` / `..=` have lower precedence than every binary operator (as in Rust): `outer + 1..n` is
//! `(outer + 1)..n`. Interpreted, compiled and formatted forms must agree with the hand-derived output.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SRC: &str = r#"fn has_duplicates(values: Vec<int>) -> bool {
    for outer in 0..values.len() {
        for inner in outer + 1..values.len() {
            if values[outer] == values[inner] {
                return true
            }
        }
    }
    false
}

fn sum_to(n: int) -> int {
    let mut t: int = 0
    for i in 1..n + 1 {
        t = t + i
    }
    t
}

fn pairs(n: int) -> int {
    let mut c: int = 0
    for i in 0..n {
        for j in i * 1 + 1..=n {
            c = c + 1
        }
    }
    c
}

fn main() {
    println(has_duplicates([1, 2, 1]))
    println(has_duplicates([1, 2, 3]))
    println(sum_to(4))
    println(pairs(3))
    for k in 2 + 1..5 - 1 {
        println(k)
    }
}
"#;

const EXPECT: &str = "true\nfalse\n10\n6\n3\n";

fn out(c: &mut Command, what: &str) -> String {
    let o = c.output().unwrap();
    assert!(o.status.success(), "{what} failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).to_string()
}

fn file() -> (TempDir, std::path::PathBuf) {
    let d = TempDir::new().unwrap();
    let f = d.path().join("r.wj");
    fs::write(&f, SRC).unwrap();
    (d, f)
}

#[test]
fn range_bounds_take_whole_additive_expressions_interpreted() {
    let (_d, f) = file();
    assert_eq!(out(Command::new(WJ).args(["run", "--interpret"]).arg(&f), "interpreter"), EXPECT);
}

#[test]
fn range_bounds_take_whole_additive_expressions_compiled() {
    let (d, f) = file();
    let o = d.path().join("out");
    out(Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).arg("--output").arg(&o), "wj build");
    let bin = d.path().join("bin");
    out(Command::new("rustc").arg(o.join("r.rs")).arg("-o").arg(&bin).args(["--edition", "2021", "-A", "warnings"]), "rustc");
    assert_eq!(out(&mut Command::new(&bin), "binary"), EXPECT);
}

#[test]
fn formatter_keeps_range_bounds_unparenthesised() {
    let (_d, f) = file();
    out(Command::new(WJ).arg("fmt").arg(&f), "wj fmt");
    assert_eq!(fs::read_to_string(&f).unwrap(), SRC);
}

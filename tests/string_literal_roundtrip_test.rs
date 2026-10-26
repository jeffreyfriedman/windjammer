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
//! Gate: string literal content is never changed silently. The same program is formatted
//! (`wj fmt` must leave it byte-identical), interpreted, compiled to Rust, and (when `go` / `node` are
//! installed) compiled to Go and JavaScript; every run must print the hand-derived text.
//! Rules: SPEC-LEX-009 (`$$` is a literal `$`, `${` interpolates), SPEC-LEX-008 (escapes).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SRC: &str = r#"fn js() -> string {
    "function qa(s) { return $(s); } // $$$$ in source"
}

fn main() {
    let x = 7
    println("a$b")
    println("cost $$5")
    println("$$$$")
    println("$${x}")
    println("${x}")
    println("$\{x}")
    println("$")
    println("end$")
    println("say \"hi\" \\ done")
    println("é ñ 日本 😀")
    println("tab\there")
    println("${js()}")
}
"#;

const EXPECT: &str = "a$b\ncost $5\n$$\n${x}\n7\n${x}\n$\nend$\nsay \"hi\" \\ done\né ñ 日本 😀\ntab\there\nfunction qa(s) { return $(s); } // $$ in source\n";

fn tool_present(name: &str) -> bool {
    Command::new(name).arg(if name == "go" { "version" } else { "--version" }).output().is_ok_and(|o| o.status.success())
}

fn project() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("s.wj");
    fs::write(&f, SRC).unwrap();
    (dir, f)
}

fn stdout(c: &mut Command, what: &str) -> String {
    let o = c.output().unwrap();
    assert!(o.status.success(), "{what} failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).to_string()
}

#[test]
fn formatter_leaves_string_literals_byte_identical() {
    let (_d, f) = project();
    stdout(Command::new(WJ).arg("fmt").arg(&f), "wj fmt");
    assert_eq!(fs::read_to_string(&f).unwrap(), SRC);
}

#[test]
fn interpreter_prints_literal_content_exactly() {
    let (_d, f) = project();
    assert_eq!(stdout(Command::new(WJ).args(["run", "--interpret"]).arg(&f), "interpreter"), EXPECT);
}

#[test]
fn rust_backend_prints_literal_content_exactly() {
    let (d, f) = project();
    let out = d.path().join("out");
    stdout(Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).arg("--output").arg(&out), "wj build");
    let bin = d.path().join("bin");
    stdout(Command::new("rustc").arg(out.join("s.rs")).arg("-o").arg(&bin).args(["--edition", "2021", "-A", "warnings"]), "rustc");
    assert_eq!(stdout(&mut Command::new(&bin), "compiled binary"), EXPECT);
}

#[test]
fn javascript_backend_prints_literal_content_exactly() {
    if !tool_present("node") {
        eprintln!("skipped: node not installed");
        return;
    }
    let (d, f) = project();
    let out = d.path().join("out");
    stdout(Command::new(WJ).args(["build", "--target", "js", "--no-cargo"]).arg(&f).arg("--output").arg(&out), "wj build js");
    assert_eq!(stdout(Command::new("node").arg("main.mjs").current_dir(&out), "node"), EXPECT);
}

#[test]
fn go_backend_prints_literal_content_exactly() {
    if !tool_present("go") {
        eprintln!("skipped: go not installed");
        return;
    }
    let (d, f) = project();
    let out = d.path().join("out");
    stdout(Command::new(WJ).args(["build", "--target", "go", "--no-cargo"]).arg(&f).arg("--output").arg(&out), "wj build go");
    assert_eq!(stdout(Command::new("go").args(["run", "."]).current_dir(&out), "go run"), EXPECT);
}

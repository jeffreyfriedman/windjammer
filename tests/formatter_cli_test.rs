#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! `wj fmt` command-line behavior: exit codes, path arguments, stdin/stdout.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

const UGLY: &str = "fn main(){\nlet x=1\n}\n";
const PRETTY: &str = "fn main() {\n    let x = 1\n}\n";

fn wj() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wj"))
}

#[test]
fn check_fails_on_unformatted_and_passes_after_format() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("a.wj");
    fs::write(&file, UGLY).unwrap();

    let check = wj().args(["fmt", "--check"]).arg(&file).output().unwrap();
    assert!(!check.status.success(), "--check must fail for unformatted input");
    assert_eq!(fs::read_to_string(&file).unwrap(), UGLY, "--check must not modify files");

    let fmt = wj().arg("fmt").arg(&file).output().unwrap();
    assert!(fmt.status.success());
    assert_eq!(fs::read_to_string(&file).unwrap(), PRETTY);

    let check = wj().args(["fmt", "--check"]).arg(&file).output().unwrap();
    assert!(check.status.success(), "--check must pass on formatted input");
}

#[test]
fn directory_argument_formats_all_wj_files_recursively() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("sub")).unwrap();
    fs::write(tmp.path().join("a.wj"), UGLY).unwrap();
    fs::write(tmp.path().join("sub/b.wj"), UGLY).unwrap();
    fs::write(tmp.path().join("notes.txt"), UGLY).unwrap();

    assert!(wj().arg("fmt").arg(tmp.path()).status().unwrap().success());
    assert_eq!(fs::read_to_string(tmp.path().join("a.wj")).unwrap(), PRETTY);
    assert_eq!(fs::read_to_string(tmp.path().join("sub/b.wj")).unwrap(), PRETTY);
    assert_eq!(fs::read_to_string(tmp.path().join("notes.txt")).unwrap(), UGLY);
}

#[test]
fn dash_formats_stdin_to_stdout() {
    let mut child = wj()
        .args(["fmt", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(UGLY.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap(), PRETTY);
}

#[test]
fn wj_toml_fmt_table_controls_indent() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("wj.toml"), "[fmt]\nindent = 2\n").unwrap();
    let file = tmp.path().join("a.wj");
    fs::write(&file, UGLY).unwrap();
    assert!(wj().arg("fmt").arg(&file).status().unwrap().success());
    assert_eq!(fs::read_to_string(&file).unwrap(), "fn main() {\n  let x = 1\n}\n");
}

#[test]
fn missing_path_is_an_error() {
    let status = wj().args(["fmt", "/definitely/not/here.wj"]).status().unwrap();
    assert!(!status.success());
}

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
//! `wj run --compiled` runs the program in the caller's working directory (relative paths in the program
//! resolve there), exactly as `wj run --interpret` does.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SRC: &str = "use std::fs\n\nfn main() {\n    match fs.read_to_string(\"input.txt\") {\n        Ok(text) => println(\"read: ${text}\"),\n        Err(e) => println(\"error: ${e}\"),\n    }\n    match fs.write(\"made.txt\", \"created\") {\n        Ok(_) => println(\"wrote\"),\n        Err(e) => println(\"error: ${e}\"),\n    }\n}\n";

fn run(mode: &str) -> String {
    let dir = TempDir::new().unwrap();
    let work = dir.path().join("workdir");
    fs::create_dir_all(&work).unwrap();
    fs::write(work.join("input.txt"), "hello").unwrap();
    fs::write(work.join("prog.wj"), SRC).unwrap();
    let o = Command::new(WJ).args(["run", mode, "prog.wj"]).current_dir(&work).output().unwrap();
    assert!(o.status.success(), "wj run {mode} failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(work.join("made.txt").exists(), "wj run {mode}: relative write did not land in the caller's directory");
    String::from_utf8_lossy(&o.stdout).to_string()
}

#[test]
fn compiled_run_keeps_the_callers_working_directory() {
    let compiled = run("--compiled");
    assert!(compiled.contains("read: hello"), "relative path did not resolve in the caller's directory:\n{compiled}");
    assert_eq!(compiled, run("--interpret"), "compiled and interpreted runs print different bytes");
}

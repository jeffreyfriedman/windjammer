//! The calls a build-slot gate written in Windjammer makes, under `wj run --interpret`: `env::args`
//! after `--`, `fs::create_dir` as an atomic lock (the second create fails), `fs::write`/`read_to_string`
//! of the owner's pid, `process::pid`, `process::run_inherit` (a `Result` of the child's exit code, and
//! an error for a missing program) and `process::exit` with that code. An interpreter that did not
//! implement one of them used to return nothing for it, so such a gate polled forever without a word;
//! a standard-library function the interpreter lacks must instead stop the run with a clear error.

use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const GATE: &str = r#"use std::env
use std::fs
use std::process

fn main() {
    let args = env::args()
    println("args ${args.len()} ${args[1]} ${args[2]}")
    let d = "slots/0"
    let _ = fs::create_dir_all("slots")
    println("first ${fs::create_dir(d).is_ok()} second ${fs::create_dir(d).is_ok()}")
    let _ = fs::write(d + "/pid", process::pid().to_string())
    let owner = fs::read_to_string(d + "/pid").unwrap_or("").trim().to_string()
    println("pid recorded ${owner == process::pid().to_string()}")
    let ok = match process::run_inherit("sh", ["-c", "exit 0"]) {
        Ok(c) => c,
        Err(_) => -1,
    }
    let missing = match process::run_inherit("no-such-program-for-the-gate", []) {
        Ok(_) => "ran",
        Err(_) => "error",
    }
    println("run ok ${ok} missing ${missing}")
    let _ = fs::remove_dir_all(d)
    println("freed ${!fs::exists(d)}")
    let code = match process::run_inherit(args[1], [args[2]]) {
        Ok(c) => c,
        Err(_) => 127,
    }
    process::exit(code)
}
"#;

fn project(main: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("main.wj"), main).unwrap();
    std::fs::write(
        dir.path().join("wj.toml"),
        "[package]\nname = \"gate\"\nversion = \"0.1.0\"\n\n[capabilities]\nfs.read = [\"./**\"]\nfs.write = [\"./**\"]\nprocess = [\"*\"]\n",
    )
    .unwrap();
    dir
}

#[test]
fn a_build_slot_gate_runs_interpreted() {
    let dir = project(GATE);
    let o = Command::new(WJ)
        .args(["run", "--interpret", "--no-lint", "main.wj", "--", "false", "x"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    let err = String::from_utf8_lossy(&o.stderr);
    for line in ["args 3 false x", "first true second false", "pid recorded true", "run ok 0 missing error", "freed true"] {
        assert!(out.contains(line), "missing `{line}` in:\n{out}\n{err}");
    }
    // the command's exit status is the gate's
    assert_eq!(o.status.code(), Some(1), "{out}\n{err}");
}

#[test]
fn a_missing_standard_function_is_an_error_not_a_silent_value() {
    let dir = project("use std::fs\n\nfn main() {\n    let r = fs::make_dirs_please(\"x\")\n    println(\"got\")\n}\n");
    let o = Command::new(WJ).args(["run", "--interpret", "--no-lint", "main.wj"]).current_dir(dir.path()).output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_ne!(o.status.code(), Some(0), "{err}");
    assert!(!String::from_utf8_lossy(&o.stdout).contains("got"), "the run went on after the missing call");
    assert!(err.contains("make_dirs_please") && err.contains("not implemented by the interpreter"), "{err}");
}

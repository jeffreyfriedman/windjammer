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

//! A lone script (no wj.toml) declares a sibling file module (`mod dep`) and calls `dep::f` with one argument; dep.wj
//! declares `f` with two. The script is its own crate root, so `dep::f` resolves to that declaration and the call is
//! WJ0013 (wrong number of arguments) in `wj check`, `wj run --interpret` (which never runs it) and `wj build` (which
//! stops before generating code). The control with the right argument count checks clean and runs.

use std::path::Path;
use std::process::{Command, Output};

const DEP: &str = "pub fn f(x: int, y: int) -> int {\n    x + 1\n}\n";

fn script(dir: &Path, call: &str) {
    std::fs::write(dir.join("prog.wj"), format!("mod dep\n\nfn main() {{\n    let x = {call}\n    println(\"${{x}}\")\n}}\n")).unwrap();
    std::fs::write(dir.join("dep.wj"), DEP).unwrap();
}

fn wj(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(args)
        .current_dir(dir)
        .env("WJ_CARGO_TARGET_ROOT", dir.join(".cache"))
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn assert_arity_error(what: &str, o: &Output) {
    let t = text(o);
    assert!(!o.status.success(), "{what} accepted a call with too few arguments:\n{t}");
    assert!(t.contains("[WJ0013]") && t.contains("prog.wj:4:"), "{what}: no WJ0013 at prog.wj:4:\n{t}");
    assert!(t.contains("takes 2 arguments but 1 was supplied"), "{what}: wrong message:\n{t}");
    for leak in ["error[E0", "rustc --explain", "main.rs"] {
        assert!(!t.contains(leak), "{what}: rustc text leaked (`{leak}`):\n{t}");
    }
}

#[test]
fn sibling_module_call_with_too_few_arguments_is_wj0013_everywhere() {
    let d = tempfile::tempdir().unwrap();
    script(d.path(), "dep::f(1)");
    assert_arity_error("wj check", &wj(d.path(), &["check", "prog.wj"]));
    let run = wj(d.path(), &["run", "--interpret", "prog.wj"]);
    assert_arity_error("wj run --interpret", &run);
    assert!(!String::from_utf8_lossy(&run.stdout).contains('2'), "the interpreter ran the call:\n{}", text(&run));
    assert_arity_error("wj build", &wj(d.path(), &["build", "prog.wj", "--no-cargo", "-o", "out"]));
}

#[test]
fn sibling_module_call_with_the_declared_arguments_checks_and_runs() {
    let d = tempfile::tempdir().unwrap();
    script(d.path(), "dep::f(1, 2)");
    let check = wj(d.path(), &["check", "prog.wj"]);
    assert!(check.status.success(), "the correct call was rejected:\n{}", text(&check));
    let run = wj(d.path(), &["run", "--interpret", "prog.wj"]);
    assert!(run.status.success() && String::from_utf8_lossy(&run.stdout).trim() == "2", "{}", text(&run));
}

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

//! C5 category 5: consuming trait methods on generic parameters are decided by the solver
//! (OWN-CONSUMING-METHOD), not left to the analyzer's generic-parameter default.

use std::process::Command;

fn explain(src: &str) -> String {
    let dir = tempfile::TempDir::new().unwrap();
    let f = dir.path().join("a.wj");
    std::fs::write(&f, src).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).arg("explain-ownership").arg(&f).output().unwrap();
    String::from_utf8_lossy(&o.stdout).to_string()
}

#[test]
fn into_on_generic_param_is_owned_by_rule() {
    let out = explain("pub fn label<T: Into<string>>(t: T) -> string {\n    t.into()\n}\n");
    assert!(out.contains("param t: T  ->  owned"), "{out}");
    assert!(out.contains("OWN-CONSUMING-METHOD"), "{out}");
}

#[test]
fn into_on_non_generic_param_gets_no_rule() {
    let out = explain("pub fn f(t: int) -> int {\n    t.into()\n}\n");
    assert!(!out.contains("OWN-CONSUMING-METHOD"), "{out}");
}

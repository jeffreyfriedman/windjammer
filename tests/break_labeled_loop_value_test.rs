#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! `break 'a: loop { .. }`: a lifetime followed by `:` after `break` starts a labeled loop that is the break's
//! value; it is not the break's target label (found by the spec-grammar sentence generator).

use std::process::Command;

#[test]
fn a_labeled_loop_can_be_the_value_of_break() {
    let dir = tempfile::TempDir::new().unwrap();
    let f = dir.path().join("main.wj");
    let src = "fn main() {\n    let x = loop {\n        break 'inner: loop {\n            break 'inner 5\n        }\n    }\n    println(\"${x}\")\n}\n";
    std::fs::write(&f, src).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["run", "--interpret"]).arg(&f).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&o.stdout), "5\n", "{}", String::from_utf8_lossy(&o.stderr));
}

//! A WJ0003 "owned string, found borrowed" from the compiled build points at the value that conflicts (the field
//! initializer / the pushed value), with the whole-line fallback only when the identifier is not unique on its line.
//! Checked through `wj run --verify`, which reports the compiled build's errors.

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

use std::process::Command;
use tempfile::TempDir;

fn verify(src: &str) -> String {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("main.wj"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["run", "--verify", "main.wj"])
        .current_dir(dir.path())
        .stdin(std::process::Stdio::null())
        .env("WJ_ALLOW_TOOLCHAIN_MISMATCH", "1")
        .output()
        .unwrap();
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
}

#[test]
fn borrowed_slice_stored_in_a_struct_is_reported_at_the_value() {
    let text = verify(
        "struct Edit {\n    pos: int,\n    old: string,\n}\n\nfn make(line: string, off: int, total: int) -> Edit {\n    let t = line[off..total]\n    Edit { pos: off, old: t }\n}\n\nfn main() {\n    let e = make(\"hello\", 1, 3)\n    println(\"${e.old}\")\n}\n",
    );
    assert!(text.contains("main.wj:8:27"), "primary span must be the `t` at 8:27, not a declaration line:\n{text}");
}

#[test]
fn borrowed_loop_variable_pushed_into_a_list_is_reported_at_the_push() {
    let text = verify(
        "fn main() {\n    let mut exits: Vec<string> = []\n    for d in [\"n\", \"s\"] {\n        exits.push(d)\n    }\n    println(\"${exits.len()}\")\n}\n",
    );
    assert!(text.contains("main.wj:4:20"), "primary span must be the pushed `d`:\n{text}");
}

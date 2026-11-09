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
//! A module constant is a string constant because its declaration says `string`, never because of its name.
//! `GREETING: string` lowers to `&'static str` and is owned where a `String` is needed; `SCOPE_LIMIT: int` is an
//! integer whatever its prefix. Oracle: rustc accepts the generated Rust.

use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

const SRC: &str = "\
pub const GREETING: string = \"hello\"
pub const SCOPE_LIMIT: int = 3

pub struct Note {
    pub text: string,
    pub tags: Vec<string>,
}

pub fn notes() -> Vec<string> {
    let mut v = Vec::new()
    v.push(GREETING)
    v
}

pub fn note() -> Note {
    Note { text: GREETING, tags: vec![GREETING, GREETING] }
}

pub fn limits() -> Vec<int> {
    let mut v = Vec::new()
    v.push(SCOPE_LIMIT)
    v
}
";

#[test]
fn string_constants_are_decided_by_declaration_not_name() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("consts.wj");
    std::fs::write(&file, SRC).unwrap();
    let out = dir.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&file).output().unwrap();
    assert!(b.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&b.stderr));
    let rs = std::fs::read_to_string(out.join("consts.rs")).unwrap();
    assert!(!rs.contains("SCOPE_LIMIT.to_string()"), "an int constant was treated as a string by its name:\n{rs}");
    let r = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "--out-dir"])
        .arg(dir.path())
        .arg(out.join("consts.rs"))
        .output()
        .unwrap();
    assert!(r.status.success(), "generated Rust does not type-check:\n{}\n--- generated\n{rs}", String::from_utf8_lossy(&r.stderr));
}

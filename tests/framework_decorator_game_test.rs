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
//! `@game` (a registered framework decorator, docs/spec/attributes.md) gives the struct a zero-value `Default` (no conflicting
//! second impl). Oracle: rustc accepts a program that starts from `World::default()`.

use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[test]
fn game_decorator_generates_default_with_zero_fields() {
    let src = "@game\npub struct World {\n    pub tick: int,\n    pub speed: float,\n    pub paused: bool,\n    pub name: string,\n    pub scores: Vec<int>,\n}\n\npub fn start() -> World {\n    let w = World::default()\n    w\n}\n";
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("world.wj");
    std::fs::write(&file, src).unwrap();
    let out = dir.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&file).output().unwrap();
    assert!(b.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&b.stderr));
    let rs = std::fs::read_to_string(out.join("world.rs")).unwrap();
    let r = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "--out-dir"])
        .arg(dir.path())
        .arg(out.join("world.rs"))
        .output()
        .unwrap();
    assert!(r.status.success(), "{}\n--- generated\n{rs}", String::from_utf8_lossy(&r.stderr));
}

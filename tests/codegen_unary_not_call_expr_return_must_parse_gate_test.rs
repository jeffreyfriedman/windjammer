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

//! Bare `!false` (unary-not of literal) as an impl-trait method return expression
//! must parse under `--module-file`.
//!
//! Regression: after `let _ = name`, newline `!false` was misread as macro `name!`
//! (`Expected (, [, or { after macro name!`). Binding first works: `let ok = false; !ok`.
//! `if !expr { ... }` was fine; bare return was not.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn codegen_unary_not_literal_impl_return_must_parse_under_module_file() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let src_dir = root.join("src");
    fs::create_dir_all(src_dir.join("ports")).unwrap();
    fs::create_dir_all(src_dir.join("adapters")).unwrap();

    fs::write(
        src_dir.join("mod.wj"),
        "mod ports\nmod adapters\n",
    )
    .unwrap();
    fs::write(src_dir.join("ports").join("mod.wj"), "mod access\n").unwrap();
    fs::write(
        src_dir.join("ports").join("access.wj"),
        r#"
pub trait AccessPort {
    fn writes_allowed(self, name: string) -> bool
}
"#,
    )
    .unwrap();
    fs::write(src_dir.join("adapters").join("mod.wj"), "mod seed_access\n").unwrap();
    fs::write(
        src_dir.join("adapters").join("seed_access.wj"),
        r#"
use crate::ports::access::AccessPort

pub struct SeedAccessPort {}

impl AccessPort for SeedAccessPort {
    fn writes_allowed(self, name: string) -> bool {
        let _ = name
        !false
    }
}
"#,
    )
    .unwrap();

    let out = root.join("out");
    fs::create_dir_all(&out).unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src_dir.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("run wj");
    assert!(
        build.status.success(),
        "wj --module-file must parse bare `!false` impl return. stderr=\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
}

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
    feature = "integration_tests",
))]

//! FAILING REPRO — `while i < parts.len()` + `parts[i]` + `i = i + 1` emits
//! `i += 1 as i32` while `i` is promoted to `usize` → E0277 (`wj-dotenv` parse).
//!
//! Inverse of P3.304 (i32 += 1 as usize). WDB-231 tip-out gate is product-file
//! only; this is the library multipass shape.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::collections::HashMap
use std::strings

pub fn parse(content: string) -> HashMap<string, string> {
    let mut map = HashMap::new()
    let lines = strings.split_lines(content)
    for line in lines {
        let trimmed = strings.trim(line)
        if trimmed.len() == 0 {
            continue
        }
        let parts = strings.split(trimmed, "=")
        if parts.len() < 2 {
            continue
        }
        let key = strings.trim(parts[0])
        let mut value_parts = Vec::new()
        let mut i = 1
        while i < parts.len() {
            value_parts.push(parts[i])
            i = i + 1
        }
        let value = strings.join(value_parts, "=")
        if key.len() > 0 {
            map.insert(key, strings.trim(value))
        }
    }
    map
}
"#;

#[test]
fn module_file_vec_index_loop_must_not_add_i32_to_usize() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let bad = generated.contains("+= 1 as i32")
        || generated.contains("+= 1_i32")
        || generated.contains("i += 1 as i32");
    assert!(
        !bad,
        "RED P3.311: vec-index loop must not emit i += 1 as i32 onto usize:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (dotenv-shaped index loop):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}

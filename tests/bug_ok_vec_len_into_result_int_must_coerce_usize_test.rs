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

//! FAILING REPRO — `Ok(hits.len())` into `Result<int, string>` emits bare usize
//! (`expected i64, found usize` / E0308).
//!
//! Product: `wj-regex::match_count` — tip 0.50.0.
//! Related GREEN: bare `filter(...).len()` return as `int` (`wj-glob::match_count`);
//! wrapping the same `.len()` in `Ok(...)` loses the int coercion.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
pub fn count_items(items: Vec<string>) -> Result<int, string> {
    Ok(items.len())
}
"#;

#[test]
fn ok_vec_len_into_result_int_must_coerce_usize() {
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
        "wj build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.681 emit:\n{generated}");

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(
        check.status.success(),
        "cargo check must accept Ok(vec.len()) as Result<int,_> (usize→i64):\n{err}\nemit:\n{generated}"
    );
}

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

//! P3.575 / ecosystem `wj-csv`: `let headers = rows[0]` then later `rows[i]`
//! must clone into the owned let — not `let headers: Vec<String> = &rows[0]`
//! (E0308).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
pub fn parse_with_headers(rows: Vec<Vec<string>>) -> Result<(Vec<string>, Vec<Vec<string>>), string> {
    if rows.len() == 0 {
        return Err("empty")
    }
    let headers = rows[0]
    let mut data = Vec::new()
    let mut i = 1
    while i < rows.len() {
        data.push(rows[i])
        i = i + 1
    }
    Ok((headers, data))
}
"#;

#[test]
fn vec_index_reuse_must_clone_into_owned_let() {
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
        "P3.575 transpile failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.575 lib.rs:\n{rs}");

    assert!(
        !rs.contains("let headers: Vec<String> = &rows[0]")
            && !rs.contains("let headers = &rows[0]"),
        "P3.575 RED: owned let of indexed Vec must not bind shared ref:\n{rs}"
    );
    assert!(
        rs.contains("rows[0].clone()")
            || rs.contains("(&rows[0]).clone()")
            || rs.contains("rows[0 as usize].clone()"),
        "P3.575 RED: reuse of rows after index must clone into owned headers:\n{rs}"
    );

    let check = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3575_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    assert!(
        check.status.success(),
        "P3.575 RED: vec index reuse into owned let must cargo-check:\n{rs}\n{err}"
    );
}

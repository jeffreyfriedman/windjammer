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

//! P3.614: `for pair in headers` with `let key = pair.0` / `return pair.1` must
//! cargo-check. Demoting to `&Vec<(String,String)>` then moving fields → E0507;
//! star-deref `*pair.0 == "…"` → E0277 (`str` vs `&str`).
//!
//! Preferred: keep Owned `Vec` when the loop moves non-Copy fields (not WDB-412
//! compare-only). Alternate: borrowed for-in + `.clone()` on field extract.
//! Do not reshape apps to `for (key, value) in headers` only.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn borrowed_for_in_tuple_field_must_clone_into_owned() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.wj"),
        r#"
fn headers_meta(headers: Vec<(string, string)>) -> (string, string) {
    let mut origin = ""
    let mut client_key = ""
    for pair in headers {
        let key = pair.0
        let value = pair.1
        if key == "Origin" {
            origin = value
            continue
        }
        if key == "X-Client-Key" {
            client_key = value
        }
    }
    (origin, client_key)
}

fn client_key_from_headers(headers: Vec<(string, string)>) -> string {
    for pair in headers {
        if pair.0 == "X-Forwarded-For" {
            return pair.1
        }
    }
    "anonymous"
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("out");
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
        "P3.614 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.614 emit:\n{rs}");

    assert!(
        !rs.contains("*pair.0") && !rs.contains("*key ==") && !rs.contains("if *key"),
        "P3.614 RED: must not star-deref string tuple fields into str == &str:\n{rs}"
    );

    let owned_vec = rs.contains("headers: Vec<(") && !rs.contains("headers: &Vec<(");
    let cloned = rs.contains("pair.0.clone()") || rs.contains("pair.1.clone()");
    assert!(
        owned_vec || cloned,
        "P3.614 RED: keep Owned Vec when moving pair.N, or clone under &Vec for-in:\n{rs}"
    );

    let ck = Command::new("cargo")
        .args([
            "check",
            "--offline",
            "--manifest-path",
            out.join("Cargo.toml").to_str().unwrap(),
        ])
        .env(
            "CARGO_TARGET_DIR",
            tmp.path().join("ck").to_str().unwrap(),
        )
        .output()
        .expect("cargo check");
    assert!(
        ck.status.success(),
        "P3.614 RED: cargo-check failed:\n{}\n{rs}",
        String::from_utf8_lossy(&ck.stderr)
    );
}

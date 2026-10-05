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

//! Product tip-out: isolate `qs_get_literal_into_demoted_key_must_not_string_from`
//! is GREEN, but `apps/wj-notes-api` still emits
//! `qs_get(query, String::from("pretty"))` into path-dep `key: &str` (E0308).
//! Do not reshape the app.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn notes_api_product_qs_get_literal_must_not_string_from() {
    let app = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo parent")
        .join("windjammer-ecosystem/apps/wj-notes-api");
    if !app.join("src").exists() {
        eprintln!("skip: notes-api src not at {}", app.display());
        return;
    }

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .current_dir(&app)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build notes-api");
    assert!(
        build.status.success(),
        "notes-api product transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let api_rs = fs::read_to_string(out.join("domain").join("api.rs")).unwrap_or_default();
    eprintln!(
        "product qs_get sites:\n{}",
        api_rs
            .lines()
            .filter(|l| l.contains("qs_get"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        !api_rs.contains("String::from(\"pretty\")")
            && !api_rs.contains("\"pretty\".to_string()"),
        "product qs_get key &str must not own the pretty literal (isolate GREEN, product RED):\n{api_rs}"
    );
    assert!(
        !api_rs.contains("String::from(\"encoding\")")
            && !api_rs.contains("\"encoding\".to_string()"),
        "product qs_get key &str must not own the encoding literal:\n{api_rs}"
    );
    // P3.666 tip 20:06 also owns "q" / "limit" keys the same way.
    assert!(
        !api_rs.contains("String::from(\"q\")")
            && !api_rs.contains("\"q\".to_string()"),
        "product qs_get key &str must not own the q literal:\n{api_rs}"
    );
    assert!(
        !api_rs.contains("String::from(\"limit\")")
            && !api_rs.contains("\"limit\".to_string()"),
        "product qs_get key &str must not own the limit literal:\n{api_rs}"
    );
}

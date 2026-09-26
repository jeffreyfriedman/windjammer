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

//! Product tip-out: isolate `notes_api_owned_into_demoted_str_must_auto_borrow` is
//! GREEN, but `apps/wj-notes-api` still emits `log_tagged(level, …, message)`,
//! `parse_level(probe)`, `slugify(title)` into path-dep `&str` (E0308).
//! Do not reshape the app.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn notes_api_product_src_must_auto_borrow_demoted_str() {
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
    let config_rs = fs::read_to_string(out.join("domain").join("config.rs")).unwrap_or_default();
    let store_rs = fs::read_to_string(out.join("domain").join("store.rs")).unwrap_or_default();
    eprintln!(
        "product api log_tagged sites:\n{}",
        api_rs
            .lines()
            .filter(|l| l.contains("log_tagged"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        api_rs.contains("log_tagged(&level") || api_rs.contains("log_tagged(& level"),
        "product log_tagged must borrow owned level (isolate GREEN, product RED):\n{api_rs}"
    );
    assert!(
        config_rs.contains("parse_level(&probe") || config_rs.contains("parse_level(& probe"),
        "product parse_level must borrow owned probe:\n{config_rs}"
    );
    assert!(
        store_rs.contains("slugify(&title") || store_rs.contains("slugify(& title"),
        "product slugify must borrow owned title:\n{store_rs}"
    );
}

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

//! Product tip-out: `apps/wj-notes-api` emits `json::to_string(&mut payload)`
//! and `json::to_string(&mut notes)` (E0596). Do not reshape the app.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn notes_api_product_json_to_string_must_not_mut_borrow() {
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
        "product json::to_string sites:\n{}",
        api_rs
            .lines()
            .filter(|l| l.contains("to_string"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        !api_rs.contains("to_string(&mut payload)")
            && !api_rs.contains("to_string(& mut payload)")
            && !api_rs.contains("to_string(&mut notes)")
            && !api_rs.contains("to_string(& mut notes)"),
        "product json.to_string must not mut-borrow payload/notes (E0596):\n{api_rs}"
    );
}

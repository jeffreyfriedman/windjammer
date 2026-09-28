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

//! Product tip-out for P3.524. Adapter `find_char` stays `text: &String`
//! while `split_header_line(line: &str)` passes `line` (E0308). Domain
//! `find_char` already `text: &str`. Do not reshape notes-api.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn notes_api_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-notes-api");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-notes-api");
            if uncle.join("src").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[test]
fn product_adapter_find_char_text_must_not_be_string_ref() {
    let app = notes_api_app().unwrap_or_else(|| {
        panic!(
            "notes-api src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

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

    let http_rs = fs::read_to_string(out.join("adapters").join("http_server.rs")).unwrap_or_default();
    let sig = http_rs
        .lines()
        .find(|l| l.contains("fn find_char("))
        .unwrap_or("")
        .to_string();
    eprintln!("product adapter find_char: {sig}");

    assert!(
        !sig.contains("text: &String"),
        "P3.524 RED: adapter find_char text must be &str (split_header_line passes &str):\n{sig}"
    );
}

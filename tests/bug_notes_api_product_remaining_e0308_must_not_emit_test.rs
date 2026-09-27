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

//! Product tip-out for the four remaining `wj-notes-api` E0308s on tip p3486.
//! Do not reshape the app. Isolates: owned `&mut query`, if-int→u16,
//! demoted Vec early-return, empty lits into owned String.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn notes_api_product_remaining_e0308_must_not_emit() {
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
        "product remaining sites:\n{}",
        api_rs
            .lines()
            .filter(|l| {
                l.contains("note_get_reply")
                    || l.contains("error_from_message(status")
                    || l.contains("return notes")
                    || l.contains("app.handle(")
            })
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        !api_rs.contains("&mut query") && !api_rs.contains("& mut query"),
        "product note_get_reply must not pass &mut query into String:\n{api_rs}"
    );
    assert!(
        !api_rs.contains("404_i32") && !api_rs.contains("400_i32"),
        "product if-int status must be u16, not i32:\n{api_rs}"
    );
    let truncate_demoted = api_rs.contains("notes: &Vec<") || api_rs.contains("notes: & Vec<");
    if truncate_demoted {
        assert!(
            api_rs.contains("notes.clone()")
                || api_rs.contains("notes.to_vec()")
                || api_rs.contains("return notes.to_owned()"),
            "product truncate_notes demoted &Vec must clone on return:\n{api_rs}"
        );
    }
    assert!(
        !api_rs.contains("app.handle(&method, path, \"\", \"\", \"\""),
        "product handle owned String formals must own empty literals:\n{api_rs}"
    );
}

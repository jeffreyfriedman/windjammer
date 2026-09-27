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

//! Cross-module private `truncate_notes(notes: Vec<Note>) -> Vec<Note>` must
//! cargo-check when `Note` has string fields.
//!
//! Same-file `pub fn` stays owned. Product `domain/note.wj` + private
//! `truncate_notes` emits `notes: &Vec<Note>` then `return notes` (E0308).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn string_note_vec_early_return_must_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod note\npub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("note.wj"),
        r#"
pub struct Note {
    pub id: int,
    pub uid: string,
    pub title: string,
    pub body: string,
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
        r#"
use crate::domain::note::Note

fn list_notes() -> Vec<Note> {
    Vec::new()
}

fn truncate_notes(notes: Vec<Note>, limit: int) -> Vec<Note> {
    if limit <= 0 {
        return notes
    }
    let mut out = Vec::new()
    let mut i = 0
    while i < notes.len() && out.len() < limit {
        out.push(notes[i])
        i = i + 1
    }
    out
}

pub fn list_limited(limit: int) -> Vec<Note> {
    let notes = list_notes()
    truncate_notes(notes, limit)
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("out");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("domain").join("api.rs")).unwrap_or_else(|_| {
        let mut acc = String::new();
        if let Ok(entries) = fs::read_dir(out.join("domain")) {
            for e in entries.flatten() {
                if e.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                    acc.push_str(&fs::read_to_string(e.path()).unwrap_or_default());
                }
            }
        }
        acc
    });
    eprintln!("generated:\n{rs}");

    let demoted = rs.contains("notes: &Vec<") || rs.contains("notes: & Vec<");
    if demoted {
        assert!(
            rs.contains("notes.clone()")
                || rs.contains("notes.to_vec()")
                || rs.contains("return notes.to_owned()"),
            "demoted &Vec<Note> must clone on owned return:\n{rs}"
        );
    }

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "string-field Note Vec early-return must cargo-check.\n{rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

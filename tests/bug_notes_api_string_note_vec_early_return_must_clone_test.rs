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

//! `Vec<Note>` with string fields + index scan + `return notes` must cargo-check.
//!
//! P3.489 isolate used `Note { id: int }` (Copy) and stayed owned + `notes.clone()`.
//! Product `truncate_notes` demotes to `&Vec<Note>` then `return notes` (E0308).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn string_note_vec_early_return_must_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::write(
        &src,
        r#"
struct Note {
    id: int,
    uid: string,
    title: string,
    body: string,
}

pub fn truncate_notes(notes: Vec<Note>, limit: int) -> Vec<Note> {
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

pub fn list_limited(notes: Vec<Note>, limit: int) -> Vec<Note> {
    truncate_notes(notes, limit)
}
"#,
    )
    .unwrap();

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

    let rs = fs::read_to_string(out.join("t.rs"))
        .or_else(|_| fs::read_to_string(out.join("lib.rs")))
        .unwrap_or_else(|_| {
            let mut acc = String::new();
            if let Ok(entries) = fs::read_dir(&out) {
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

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

//! Multipass demotes `Vec<Custom>` helper formals to `&Vec<T>` then call sites
//! emit `helper(notes.clone())` (owned) → E0308.
//!
//! Dogfood: `wj-notes-api` list `?q=` filter; product inlines the loop instead of
//! `filter_notes_by_regex(notes, pattern)`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn owned_vec_custom_filter_helper_must_not_demote_and_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir");

    fs::write(
        src.join("domain").join("note.wj"),
        r#"
pub struct Note {
    pub title: string,
    pub body: string,
}
"#,
    )
    .unwrap();

    fs::write(
        src.join("domain").join("filter.wj"),
        r#"
use crate::domain::note::Note

pub fn filter_notes(notes: Vec<Note>, needle: string) -> Vec<Note> {
    let mut out = Vec::new()
    let mut i = 0
    while i < notes.len() {
        let note = notes[i]
        if note.title == needle {
            out.push(note)
        }
        i = i + 1
    }
    out
}

pub fn apply(notes: Vec<Note>, needle: string) -> Vec<Note> {
    filter_notes(notes, needle)
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("gen");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("domain").to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("domain").join("filter.rs"))
        .or_else(|_| fs::read_to_string(out.join("filter.rs")))
        .expect("read filter.rs");

    let demoted_formal = generated.contains("notes: &Vec<Note>")
        || generated.contains("notes: &Vec<");
    let owned_into_borrowed_call = generated.contains("filter_notes(notes.clone()")
        || generated.contains("filter_notes(notes.clone(),");

    assert!(
        !(demoted_formal && owned_into_borrowed_call),
        "RED: owned Vec<Note> filter helper demoted to &Vec while call site clones owned.\ngenerated:\n{generated}"
    );

    assert!(
        !demoted_formal || generated.contains("filter_notes(notes)") || generated.contains("filter_notes(&notes)"),
        "RED: if Vec formal demotes to &Vec, call sites must borrow consistently (not notes.clone()).\ngenerated:\n{generated}"
    );

    let cargo_toml = out.join("Cargo.toml");
    if cargo_toml.exists() {
        let check = Command::new("cargo")
            .current_dir(&out)
            .args(["check", "--quiet"])
            .output()
            .expect("cargo check");
        assert!(
            check.status.success(),
            "RED: Vec<Note> filter helper must cargo-check.\ngenerated:\n{generated}\nstderr:\n{}",
            String::from_utf8_lossy(&check.stderr)
        );
    }
}

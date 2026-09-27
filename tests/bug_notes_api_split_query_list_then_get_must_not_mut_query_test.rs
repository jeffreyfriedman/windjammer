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

//! Product `dispatch` splits `query` from the path, clones it in the list arm
//! (`query_wants_pretty` + `list_notes_for_query`), then GET-one emits
//! `note_get_reply(..., &mut query)` into `query: String` (E0308).
//!
//! Smaller reuse isolates emit `query.clone()` (GREEN). This fixture matches
//! the hexagonal app: `NotesApp` + store + `split_path_query` + list-then-get.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn split_query_list_then_get_must_not_mut_query() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("domain")).expect("mkdir domain");
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();
    fs::write(src.join("domain").join("mod.wj"), "pub mod api\n").unwrap();
    fs::write(
        src.join("domain").join("api.wj"),
        r#"
use std::json

struct Note {
    uid: string,
    title: string,
    body: string,
}

struct NoteStore {
    n: int,
}

impl NoteStore {
    fn list(self) -> Vec<Note> {
        Vec::new()
    }

    fn fetch(self, id: int) -> Option<Note> {
        if id < 1 {
            return None
        }
        None
    }
}

struct NotesApp {
    store: NoteStore,
}

fn own(value: string) -> string {
    value
}

fn split_path_query(full: string) -> (string, string) {
    (full, "q=1")
}

fn query_wants_pretty(query: string) -> bool {
    let query = own(query)
    query == "pretty=1"
}

fn listish(notes: Vec<Note>, query: string) -> string {
    let query = own(query)
    if notes.len() == 0 {
        return query
    }
    query
}

fn query_wants_base64(query: string) -> bool {
    let query = own(query)
    query == "encoding=base64"
}

pub fn note_get_reply(note: Note, if_none_match: string, query: string) -> string {
    let want_base64 = query_wants_base64(query)
    if if_none_match == note.uid {
        return ""
    }
    match json.to_string(note) {
        Ok(text) => {
            if want_base64 {
                return text
            }
            text
        },
        Err(_) => "",
    }
}

impl NotesApp {
    pub fn dispatch(self, path: string, if_none_match: string) -> string {
        let split = split_path_query(path)
        let path = split.0
        let query = split.1
        if path == "/notes" {
            let want_pretty = query_wants_pretty(query)
            let _listed = listish(self.store.list(), query)
            if want_pretty {
                return "pretty"
            }
            return "list"
        }
        match self.store.fetch(1) {
            Some(note) => note_get_reply(note, if_none_match, query),
            None => "",
        }
    }
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

    let api_rs = fs::read_to_string(out.join("domain").join("api.rs")).unwrap_or_default();
    eprintln!("api.rs:\n{api_rs}");
    assert!(
        !api_rs.contains("&mut query") && !api_rs.contains("& mut query"),
        "split query list-then-get must not pass &mut query into String formal:\n{api_rs}"
    );
    let mut_note_without_binding = api_rs.contains("&mut note")
        && !api_rs.contains("Some(mut note)")
        && !api_rs.contains("let mut note");
    assert!(
        !mut_note_without_binding,
        "json.to_string demote must not emit &mut note on an immutable match binding:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "split query list-then-get must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

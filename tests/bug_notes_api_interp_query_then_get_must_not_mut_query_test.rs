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

//! Nested product `dispatch` (route_match + HashMap params + method match)
//! still emits `Some(note) => note_get_reply(&mut note, …)` (E0596).
//!
//! P3.511 shallow `match fetch` now emits `Some(mut note)` (GREEN). Product
//! GET-one is this nested graph and still has immutable `Some(note)` plus
//! `&mut query`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn interp_query_then_get_must_not_mut_query() {
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
    n: int,
}

fn own(value: string) -> string {
    value
}

fn split_path_query(full: string) -> (string, string) {
    (full, "q=1")
}

fn qs_get(query: string, key: string) -> Option<string> {
    if key == "pretty" {
        return None
    }
    None
}

fn query_wants_pretty(query: string) -> bool {
    let query = own(query)
    match qs_get(query, "pretty") {
        None => false,
        Some(text) => text == "1",
    }
}

fn list_notes_for_query(notes: Vec<Note>, query: string) -> string {
    let query = own(query)
    match qs_get("${query}", "q") {
        None => {
            if notes.len() == 0 {
                return query
            }
            query
        },
        Some(pattern) => pattern,
    }
}

fn query_wants_base64(query: string) -> bool {
    let query = own(query)
    query == "encoding=base64"
}

pub fn note_get_reply(note: Note, if_none_match: string, query: string) -> string {
    let mut held: Vec<string> = Vec::new()
    held.push(if_none_match)
    let inm = held[0]
    let want_base64 = query_wants_base64(query)
    if inm == note.uid {
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

enum HttpMethod {
    GET,
    POST,
}

fn route_match(pat: string, path: string) -> Option<HashMap<string, string>> {
    if path == pat {
        return Some(HashMap::new())
    }
    None
}

fn parse_positive_int(text: string) -> Option<int> {
    if text == "0" {
        return None
    }
    Some(1)
}

impl NotesApp {
    pub fn dispatch(self, method: HttpMethod, path: string, if_none_match: string) -> string {
        self.n = self.n + 1
        let split = split_path_query(path)
        let path = split.0
        let query = split.1
        match route_match("/notes", path) {
            Some(_) => {
                match method {
                    HttpMethod::GET => {
                        let want_pretty = query_wants_pretty(query)
                        let _listed = list_notes_for_query(self.store.list(), query)
                        if want_pretty {
                            return "pretty"
                        }
                        "list"
                    },
                    _ => "no",
                }
            },
            None => match route_match("/notes/:id", path) {
                Some(params) => match params.get("id") {
                    Some(id_text) => match parse_positive_int(own(id_text)) {
                        Some(id) => match method {
                            HttpMethod::GET => match self.store.fetch(id) {
                                Some(note) => note_get_reply(note, if_none_match, query),
                                None => "",
                            },
                            _ => "no",
                        },
                        None => "",
                    },
                    None => "",
                },
                None => "",
            },
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
        "nested dispatch must not pass &mut query into String:\n{api_rs}"
    );
    let mut_note_without_binding = api_rs.contains("&mut note")
        && !api_rs.contains("Some(mut note)")
        && !api_rs.contains("let mut note");
    assert!(
        !mut_note_without_binding,
        "nested route_match GET-one must mut-bind note before &mut note:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "interp query then GET-one must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

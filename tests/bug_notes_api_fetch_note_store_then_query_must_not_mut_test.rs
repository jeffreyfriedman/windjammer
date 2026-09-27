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

//! Product GET-one is `fetch_note(self.store, id)` (free fn) then
//! `note_get_reply(note, if_none_match, query)`. Emits `Some(note)` +
//! `&mut note` (E0596). Sibling POST/PUT still use `self.store`.
//!
//! P3.513 uses `self.store.fetch(id)` — same E0596, different call shape.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn fetch_note_store_then_query_must_not_mut() {
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

enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

struct Note {
    uid: string,
    title: string,
    body: string,
}

struct NoteInput {
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

    fn create(self, title: string, body: string) -> Result<Note, string> {
        Ok(Note { uid: "1", title: title, body: body })
    }

    fn update(self, id: int, title: string, body: string) -> Result<Note, string> {
        if id < 1 {
            return Err("note not found")
        }
        Ok(Note { uid: "1", title: title, body: body })
    }

    fn delete(self, id: int) -> Result<int, string> {
        if id < 1 {
            return Err("note not found")
        }
        Ok(id)
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

fn fetch_note(store: NoteStore, id: int) -> Option<Note> {
    if id < 1 {
        return None
    }
    let _ = store.n
    None
}

fn query_wants_pretty(query: string) -> bool {
    let query = own(query)
    query == "pretty=1"
}

fn list_notes_for_query(notes: Vec<Note>, query: string) -> Result<Vec<Note>, string> {
    let query = own(query)
    if query == "bad" {
        return Err("bad")
    }
    Ok(notes)
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

impl NotesApp {
    pub fn dispatch(self, method: HttpMethod, path: string, body: string, if_none_match: string) -> string {
        self.n = self.n + 1
        let split = split_path_query(path)
        let path = split.0
        let query = split.1
        match route_match("/notes", path) {
            Some(_) => {
                match method {
                    HttpMethod::GET => {
                        let want_pretty = query_wants_pretty(query)
                        match list_notes_for_query(self.store.list(), query) {
                            Ok(notes) => {
                                match json.to_string(notes) {
                                    Ok(text) => {
                                        if want_pretty {
                                            return text
                                        }
                                        text
                                    },
                                    Err(_) => "",
                                }
                            },
                            Err(_) => "",
                        }
                    },
                    HttpMethod::POST => {
                        let input: NoteInput = match json.parse_string(body) {
                            Ok(value) => value,
                            Err(_) => return "400",
                        }
                        match self.store.create(input.title, input.body) {
                            Ok(_) => "201",
                            Err(_) => "400",
                        }
                    },
                    _ => "404",
                }
            },
            None => match route_match("/notes/:id", path) {
                Some(params) => match params.get("id") {
                    Some(id_text) => match parse_positive_int(own(id_text)) {
                        Some(id) => match method {
                            HttpMethod::GET => match fetch_note(self.store, id) {
                                Some(note) => note_get_reply(note, if_none_match, query),
                                None => "404",
                            },
                            HttpMethod::PUT => {
                                let input: NoteInput = match json.parse_string(body) {
                                    Ok(value) => value,
                                    Err(_) => return "400",
                                }
                                match self.store.update(id, input.title, input.body) {
                                    Ok(_) => "200",
                                    Err(_) => "404",
                                }
                            },
                            HttpMethod::DELETE => match self.store.delete(id) {
                                Ok(_) => "204",
                                Err(_) => "404",
                            },
                            _ => "404",
                        },
                        None => "404",
                    },
                    None => "404",
                },
                None => "404",
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
        "fetch_note(self.store) GET-one must not pass &mut query into String:\n{api_rs}"
    );
    let mut_note_without_binding = api_rs.contains("&mut note")
        && !api_rs.contains("Some(mut note)")
        && !api_rs.contains("let mut note");
    assert!(
        !mut_note_without_binding,
        "fetch_note(self.store) must mut-bind note before &mut note:\n{api_rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "fetch_note store + query must cargo-check.\n{api_rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

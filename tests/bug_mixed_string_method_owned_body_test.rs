//! When `dispatch` consumes JSON body after comparing method/path, call-site
//! ownership must match the emitted formal (owned `String` vs `&str`).
//!
//! Ecosystem `wj-notes-api` `NotesApi::serve` → `dispatch` (GET/POST/PUT/DELETE).

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn serve_dispatch_body_call_matches_emitted_formal() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "domain/store.wj",
        r#"
use std::collections::HashMap

pub struct Note {
    pub id: int,
    pub title: string,
    pub body: string,
}

pub struct NoteStore {
    next_id: int,
    notes: HashMap<int, Note>,
}

impl NoteStore {
    pub fn new() -> NoteStore {
        NoteStore { next_id: 1, notes: HashMap::new() }
    }

    pub fn create(self, title: string, body: string) -> Result<Note, string> {
        let id = self.next_id
        self.next_id = self.next_id + 1
        let note = Note { id: id, title: title, body: body }
        self.notes.insert(id, note)
        match self.notes.get(id) {
            Some(stored) => Ok(stored),
            None => Err("create failed"),
        }
    }

    pub fn get(self, id: int) -> Option<Note> {
        self.notes.get(id)
    }

    pub fn list(self) -> Vec<Note> {
        let mut result = Vec::new()
        for (_id, note) in self.notes {
            result.push(note)
        }
        result
    }

    pub fn update(self, id: int, title: string, body: string) -> Result<Note, string> {
        match self.notes.get(id) {
            Some(_) => {
                let note = Note { id: id, title: title, body: body }
                self.notes.insert(id, note)
                match self.notes.get(id) {
                    Some(stored) => Ok(stored),
                    None => Err("update failed"),
                }
            },
            None => Err("note not found"),
        }
    }

    pub fn delete(self, id: int) -> Result<(), string> {
        match self.notes.remove(id) {
            Some(_) => Ok(()),
            None => Err("note not found"),
        }
    }
}
"#,
    );
    test.add_file(
        "domain/api.wj",
        r#"
use std::json

use crate::domain::Note
use crate::domain::NoteStore

pub struct HttpReply {
    pub status: u16,
    pub body: string,
}

pub struct HttpRequest {
    pub method: string,
    pub path: string,
    pub body: string,
}

struct NoteInput {
    title: string,
    body: string,
}

pub struct NotesApi {
    store: NoteStore,
}

impl NotesApi {
    pub fn new(store: NoteStore) -> NotesApi {
        NotesApi { store: store }
    }

    pub fn serve(self, request: HttpRequest) -> HttpReply {
        let method = request.method
        let path = request.path
        let body = request.body
        self.dispatch(method, path, body)
    }

    fn dispatch(self, method: string, path: string, body: string) -> HttpReply {
        if path == "/notes" {
            if method == "GET" {
                let notes = self.store.list()
                return match json.to_string(notes) {
                    Ok(text) => HttpReply { status: 200, body: text },
                    Err(_) => HttpReply { status: 500, body: "fail" },
                }
            }
            if method == "POST" {
                let input: NoteInput = match json.parse_string(body) {
                    Ok(value) => value,
                    Err(_) => return HttpReply { status: 400, body: "bad" },
                }
                return match self.store.create(input.title, input.body) {
                    Ok(note) => match json.to_string(note) {
                        Ok(text) => HttpReply { status: 201, body: text },
                        Err(_) => HttpReply { status: 500, body: "fail" },
                    },
                    Err(_) => HttpReply { status: 400, body: "bad" },
                }
            }
            return HttpReply { status: 404, body: "missing" }
        }
        if method == "PUT" {
            let input: NoteInput = match json.parse_string(body) {
                Ok(value) => value,
                Err(_) => return HttpReply { status: 400, body: "bad" },
            }
            match self.store.update(1, input.title, input.body) {
                Ok(note) => match json.to_string(note) {
                    Ok(text) => HttpReply { status: 200, body: text },
                    Err(_) => HttpReply { status: 500, body: "fail" },
                },
                Err(_) => HttpReply { status: 404, body: "missing" },
            }
        } else if method == "DELETE" {
            match self.store.delete(1) {
                Ok(_) => HttpReply { status: 204, body: "" },
                Err(_) => HttpReply { status: 404, body: "missing" },
            }
        } else {
            HttpReply { status: 404, body: "missing" }
        }
    }
}
"#,
    );
    test.add_file("domain/mod.wj", "pub mod store\npub mod api\n");
    test.add_file("mod.wj", "pub mod domain\n");

    let map = test
        .compile()
        .expect("notes-api-shaped dispatch should compile");
    let rs = map.get("domain/api.rs").expect("domain/api.rs generated");

    let dispatch_owns_body = rs.contains("fn dispatch(&mut self, method: &str, path: &str, body: String)");
    let serve_borrows_body = rs.contains("self.dispatch(&method, &path, &body)");
    assert!(
        !(dispatch_owns_body && serve_borrows_body),
        "serve borrowed &body into dispatch's owned String formal:\n{rs}"
    );
}

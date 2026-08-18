//! HashMap::get of a non-Copy value in a mutating match must not emit `.copied()`.
//! Ecosystem `wj-notes-api` `NoteStore::update` hits this in library multipass.

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
fn hashmap_get_noncopy_match_not_copied() {
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
    notes: HashMap<int, Note>,
}

impl NoteStore {
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
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map.get("domain/store.rs").expect("store.rs generated");

    assert!(
        !rs.contains(".copied()"),
        "HashMap.get of non-Copy Note must not use .copied():\n{rs}"
    );
    assert!(
        rs.contains(".cloned()") || rs.contains("to_owned()") || rs.contains(".clone()"),
        "expected Clone-based ownership of HashMap.get payload:\n{rs}"
    );
}

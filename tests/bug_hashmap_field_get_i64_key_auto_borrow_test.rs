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

//! FAILING REPRO — `HashMap<i64, T>` field access `.get(id)` must auto-borrow key.
//!
//! Ecosystem `wj-notes-api` `NoteStore::get` emits `self.notes.get(id)`; rustc E0308:
//! expected `&i64`, found `i64`. Single-file helpers work (`regression_hashmap_i64_key_lookup_test.wj`);
//! struct-field call sites in multipass apps regress on wj 0.50.0.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const NOTE_STORE: &str = include_str!("fixtures/library_multipass/note_store_hashmap_field_get.wj");

const NOTE_LOOKUP: &str = r#"
use crate::note_store::{NoteStore}

pub fn lookup_id(store: NoteStore, id: int) -> int {
    match store.get(id) {
        Some(note) => note.id,
        None => 0,
    }
}
"#;

#[test]
fn hashmap_field_get_i64_key_must_auto_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod note_store
pub mod note_lookup
"#,
    );
    test.add_file("note_store.wj", NOTE_STORE);
    test.add_file("note_lookup.wj", NOTE_LOOKUP);

    let map = test
        .compile()
        .expect("HashMap field get compile should succeed");
    let store = map.get("note_store.rs").expect("note_store.rs");

    let bad_emit = store.contains(".get(id)") && !store.contains(".get(&id)");
    assert!(
        !bad_emit,
        "RED: HashMap<i64, _>.get(i64) must borrow key at field call site; emitted:\n{store}"
    );
    assert!(
        store.contains(".get(&id)") || store.contains(".get(id.clone())"),
        "HashMap field get must borrow or clone key; emitted:\n{store}"
    );
    test.cargo_check()
        .expect("borrowed HashMap key must cargo-check");
}

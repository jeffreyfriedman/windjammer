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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn hashmap_field_get_i64_key_must_auto_borrow() {
    let generated = test_utils::compile_single(
        r#"
use std::collections::HashMap

pub struct Note {
    id: int,
    title: string,
}

pub struct NoteStore {
    notes: HashMap<int, Note>,
}

impl NoteStore {
    pub fn get(self, id: int) -> Option<Note> {
        self.notes.get(id)
    }
}
"#,
    );

    assert!(
        generated.contains(".get(&id)") || generated.contains(".get(id.clone())"),
        "HashMap<i64, _>.get(i64) must borrow key at field call site; emitted:\n{generated}"
    );
    assert!(
        !generated.contains(".get(id)\n") || generated.contains(".get(&id)"),
        "bare .get(id) without borrow fails rustc:\n{generated}"
    );
}

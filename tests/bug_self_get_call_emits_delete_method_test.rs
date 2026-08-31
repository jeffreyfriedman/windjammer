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

//! FAILING REPRO — `self.get(id)` must not codegen as `self.delete(id)` when both methods exist.
//!
//! Ecosystem `wj-notes-api` `NoteStore::create` calls `self.get(id)` after insert; emitted Rust
//! calls `self.delete(id)` instead → E0308 (`Result<(), String>` vs `Option<_>`).
//! Workaround: free function `lookup_note(notes, id)` instead of `self.get`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const NOTE_STORE: &str = include_str!("fixtures/library_multipass/note_store_self_get_call.wj");

#[test]
fn self_get_call_must_not_emit_delete_method() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod note_store
"#,
    );
    test.add_file("note_store.wj", NOTE_STORE);

    let map = test
        .compile()
        .expect("NoteStore get/delete compile should succeed");
    let store = map.get("note_store.rs").expect("note_store.rs");

    assert!(
        !store.contains("match self.delete(id)"),
        "RED: create/update must call self.get not self.delete; emitted:\n{store}"
    );
    assert!(
        store.contains("match self.get(id)") || store.contains(".get(id)"),
        "expected self.get(id) call in create; emitted:\n{store}"
    );
    test.cargo_check()
        .expect("self.get after insert must cargo-check");
}

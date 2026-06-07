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
))]

/// TDD Tests: HashMap.remove(key) should auto-add & for key argument.
/// Vec.remove(index) should NOT add & for index argument.
///
/// Bug 1: `self.chunks.remove(pos)` generates `self.chunks.remove(pos)` but
///         HashMap::remove takes `&Q`, so it should be `self.chunks.remove(&pos)`.
///
/// Bug 2: `codepoints.remove(&pos)` generates `codepoints.remove(&pos)` but
///         Vec::remove takes `usize`, so it should be `codepoints.remove(pos)`.
#[path = "common/test_utils.rs"]
mod test_utils;
#[allow(unused_imports)]
use test_utils::compile_single;

/// HashMap.remove(key) should auto-borrow the key.
#[test]
fn test_hashmap_remove_auto_borrows_key() {
    let input = r#"
use std::collections::HashMap

struct ChunkMap {
    chunks: HashMap<(i32, i32, i32), bool>,
}

impl ChunkMap {
    pub fn new() -> ChunkMap {
        ChunkMap { chunks: HashMap::new() }
    }

    pub fn remove_chunk(self, pos: (i32, i32, i32)) {
        self.chunks.remove(pos)
    }
}
"#;
    let output = compile_single(input);
    // HashMap::remove takes &Q, so the generated code should pass &pos
    assert!(
        output.contains("self.chunks.remove(&pos)"),
        "HashMap.remove should auto-borrow key with &.\nGenerated:\n{}",
        output
    );
}

/// HashMap.remove with a custom key type where the key comes from Vec iteration (owned).
/// This matches the game bug: for id in to_remove { self.timers.remove(id) }
/// where id is owned TimerId from iterating a Vec<TimerId>.
#[test]
fn test_hashmap_remove_owned_custom_key_auto_borrows() {
    let input = r#"
use std::collections::HashMap

struct TimerId {
    value: u32,
}

struct TimerManager {
    timers: HashMap<TimerId, f32>,
}

impl TimerManager {
    pub fn new() -> TimerManager {
        TimerManager { timers: HashMap::new() }
    }

    pub fn clear_finished(self) {
        let mut to_remove = Vec::new()
        for (id, _val) in self.timers {
            to_remove.push(id)
        }
        for id in to_remove {
            self.timers.remove(id)
        }
    }
}
"#;
    let output = compile_single(input);
    // HashMap::remove takes &Q, so when id is owned TimerId from Vec iteration,
    // the generated code should pass &id
    assert!(
        output.contains(".remove(&id)"),
        "HashMap.remove should auto-borrow owned key from loop iteration.\nGenerated:\n{}",
        output
    );
}

/// Vec.remove(index) should NOT add & to the index.
#[test]
fn test_vec_remove_no_ref_on_index() {
    let input = r#"
struct TextBuffer {
    codepoints: Vec<char>,
}

impl TextBuffer {
    pub fn new() -> TextBuffer {
        TextBuffer { codepoints: Vec::new() }
    }

    pub fn delete_at(self, pos: usize) {
        self.codepoints.remove(pos)
    }
}
"#;
    let output = compile_single(input);
    // Vec::remove takes usize, so no & should be added
    assert!(
        !output.contains("codepoints.remove(&pos)"),
        "Vec.remove should NOT add & to index.\nGenerated:\n{}",
        output
    );
    assert!(
        output.contains("codepoints.remove(pos)"),
        "Vec.remove should pass index directly.\nGenerated:\n{}",
        output
    );
}

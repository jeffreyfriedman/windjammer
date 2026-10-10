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
    feature = "codegen_tests",
))]

//!
//!   `self.entries.remove(&idx as usize)` → E0606 (can't cast `&i32` as usize).
//! Source WJ: `self.entries.remove(idx as usize)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Entry {
    pub key: string,
}

pub struct Board {
    pub entries: Vec<Entry>,
}

pub fn remove_at(board: Board, idx: i32) -> Board {
    let mut b = board
    if idx >= 0 {
        b.entries.remove(idx as usize)
    }
    b
}
"#;

const SRC_SELF_METHOD: &str = r#"
pub struct BlackboardEntry {
    pub key: string,
}

pub struct Blackboard {
    pub entries: Vec<BlackboardEntry>,
}

impl Blackboard {
    fn find_index(self, key: string) -> i32 {
        let mut i = 0
        while i < self.entries.len() {
            if self.entries[i].key == key {
                return i as i32
            }
            i = i + 1
        }
        -1
    }

    pub fn remove(self, key: string) {
        let idx = self.find_index(key)
        if idx >= 0 {
            self.entries.remove(idx as usize)
        }
    }
}
"#;

#[test]
fn module_file_vec_remove_cast_must_not_borrow_idx() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&idx as usize") || rs.contains("remove(&idx");
    assert!(
        !bad,
        "RED: Vec::remove emitted &idx as usize:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_self_entries_remove_cast_must_not_borrow_idx() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_SELF_METHOD);
    let map = test.compile().expect("self-method compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("self-method MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&idx as usize") || rs.contains("remove(&idx");
    assert!(
        !bad,
        "RED: self.entries.remove emitted &idx as usize:\n{rs}"
    );
    test.cargo_check().expect("self-method cargo-check");
}


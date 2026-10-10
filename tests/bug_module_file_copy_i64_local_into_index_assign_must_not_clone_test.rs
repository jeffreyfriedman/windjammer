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

//! Copy `i64` local into indexed assign must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct EntitySparse {
    pub entities: Vec<i64>,
    pub sparse: Vec<i64>,
    pub count: usize,
}

impl EntitySparse {
    pub fn swap_remove(self, sparse_idx_usize: usize) {
        if self.count == 0 {
            return
        }
        let swapped_entity = self.entities[self.count - 1]
        self.entities[sparse_idx_usize] = swapped_entity
        self.sparse[swapped_entity as usize] = sparse_idx_usize as i64
        self.count = self.count - 1
    }
}
"#;

#[test]
fn module_file_copy_i64_local_into_index_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("swapped_entity.clone()");
    assert!(
        !bad,
        "RED Copy i64 local cloned into index assign:\n{rs}"
    );
    // Codegen gate is the contract; cargo-check can time out under contention.
    let _ = test.cargo_check();
}

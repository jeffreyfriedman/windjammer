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

//! WDB-462: Copy **newtype** from `HashMap.keys()` into `Vec::push` must not `.clone()`.
//!
//! Product `state_machine/machine.rs` `all_state_ids`:
//!   `for id in self.states.keys() { result.push(id.clone()) }`
//! WJ uses bare `result.push(id)` (`StateId` is `u32` newtype, Copy).
//! Distinct from WDB-457 (owned Copy newtype **local** into push), WDB-459
//! (Copy **struct** local into push), WDB-431 (Copy u64 **field** into push).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct StateId {
    pub value: u32,
}

impl StateId {
    pub fn new(value: u32) -> StateId {
        StateId { value: value }
    }

    pub fn value(self) -> u32 {
        self.value
    }
}

pub struct StateMachine {
    pub states: Map<StateId, i32>,
}

impl StateMachine {
    pub fn new() -> StateMachine {
        StateMachine { states: Map::new() }
    }

    pub fn all_state_ids(self) -> Vec<StateId> {
        let mut result: Vec<StateId> = Vec::new()
        for id in self.states.keys() {
            result.push(id)
        }
        result
    }
}
"#;

#[test]
fn module_file_copy_newtype_hashmap_keys_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("id.clone()");
    assert!(
        !bad,
        "Copy newtype from HashMap.keys() cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

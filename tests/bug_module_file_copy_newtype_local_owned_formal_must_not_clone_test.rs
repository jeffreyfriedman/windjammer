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

//! DB-458: Copy **newtype** local into owned method formal must not `.clone()`.
//!
//! Product `ecs/component_storage.rs` list_component_ids_for_entity:
//!   `if self.has(entity, comp_id.clone()) { result.push(comp_id.clone()) }`
//! WJ uses bare `comp_id` for both `has` and `push`.
//! Distinct from DB-457 (Copy newtype into **Vec::push** with field reuse),
//! DB-431 (Copy u64 **field** into insert/push), and DB-393 (i32 formal field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct ComponentId {
    pub index: u32,
}

impl ComponentId {
    pub fn new(index: u32) -> ComponentId {
        ComponentId { index: index }
    }
}

pub struct Registry {
    pub arrays_len: u32,
}

impl Registry {
    pub fn has(self, entity: i64, component_id: ComponentId) -> bool {
        let _ = entity
        component_id.index < self.arrays_len
    }

    pub fn list_component_ids_for_entity(self, entity: i64) -> Vec<ComponentId> {
        let mut result: Vec<ComponentId> = Vec::new()
        let mut i: u32 = 0
        while i < self.arrays_len {
            let comp_id = ComponentId::new(i)
            if self.has(entity, comp_id) {
                result.push(comp_id)
            }
            i = i + 1
        }
        result
    }
}
"#;

#[test]
fn module_file_copy_newtype_local_owned_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("comp_id.clone()");
    assert!(
        !bad,
        "Copy newtype local cloned into owned method formal:\n{rs}"
    );
    let _ = test.cargo_check();
}

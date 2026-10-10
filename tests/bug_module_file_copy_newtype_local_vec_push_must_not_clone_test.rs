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

//! Copy **newtype** local into `Vec::push` must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct LightId {
    pub value: i32,
}

impl LightId {
    pub fn new(value: i32) -> LightId {
        LightId { value: value }
    }

    pub fn value(self) -> i32 {
        self.value
    }
}

pub struct LightManager {
    pub light_ids: Vec<LightId>,
    pub next_id: i32,
}

impl LightManager {
    pub fn new() -> LightManager {
        LightManager { light_ids: vec![], next_id: 0 }
    }

    pub fn add_light(self) -> LightId {
        let id = LightId::new(self.next_id)
        self.next_id = self.next_id + 1
        self.light_ids.push(id)
        LightId::new(id.value())
    }
}
"#;

#[test]
fn module_file_copy_newtype_local_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("id.clone()");
    assert!(
        !bad,
        "RED Copy newtype local cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

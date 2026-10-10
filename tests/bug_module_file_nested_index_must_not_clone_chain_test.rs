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

//! nested index Copy field must not emit
//! `].clone().bindings[j].clone().binding_type.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum BindingType {
    Read,
    Write,
}

pub struct Binding {
    pub binding_type: BindingType,
}

pub struct Pass {
    pub bindings: Vec<Binding>,
    pub name: string,
}

pub fn is_read(t: BindingType) -> bool {
    match t {
        BindingType::Read => true,
        BindingType::Write => false,
    }
}

pub struct Graph {
    pub passes: Vec<Pass>,
}

impl Graph {
    pub fn first_is_read(self, idx: usize, bi: usize) -> bool {
        is_read(self.passes[idx].bindings[bi].binding_type)
    }
}
"#;

#[test]
fn module_file_nested_index_must_not_clone_chain() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().bindings")
        || rs.contains("].clone().binding_type")
        || rs.contains(".binding_type.clone()");
    assert!(
        !bad,
        "RED nested index Copy field clone chain:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

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

//! DB-430: Copy unit-enum **field** of a local/loop binding must not `.clone()`
//! into an owned formal.
//!
//! Product `rendering/shader_graph_builder.rs` / `shader_graph_compiler.rs`:
//!   `is_storage_write(binding.binding_type.clone())`
//! WJ is `is_storage_write(binding.binding_type)`.
//! Distinct from DB-375 (nested index clone **chain**
//! `].clone().bindings[j].clone().binding_type.clone()`), DB-402 (`HostType::F32.clone()`
//! path), and DB-392 (`Direction::PosX.clone()`).

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

pub fn is_write(t: BindingType) -> bool {
    match t {
        BindingType::Write => true,
        BindingType::Read => false,
    }
}

pub fn any_write(bindings: Vec<Binding>) -> bool {
    for binding in bindings {
        if is_write(binding.binding_type) {
            return true
        }
    }
    false
}
"#;

#[test]
fn module_file_copy_enum_field_into_owned_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("binding_type.clone()");
    assert!(
        !bad,
        "Copy enum field cloned into owned formal:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

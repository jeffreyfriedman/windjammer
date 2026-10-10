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
//!   `emit_node_instructions(..., buf.clone())` / `emit_instruction(buf.clone(), …)`
//!   with `buf: &mut Vec<f32>` → E0308 (expected `&mut Vec`, found `Vec`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn emit_instruction(buf: Vec<f32>, opcode: f32) {
    buf.push(opcode)
}

pub fn emit_tree(buf: Vec<f32>) {
    emit_instruction(buf, 1.0)
    emit_instruction(buf, 2.0)
}
"#;

#[test]
fn module_file_mut_vec_must_not_receive_owned_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("buf.clone()")
        || rs.contains("emit_instruction(buf.clone()");
    assert!(
        !bad,
        "RED: mut Vec received owned buf.clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}


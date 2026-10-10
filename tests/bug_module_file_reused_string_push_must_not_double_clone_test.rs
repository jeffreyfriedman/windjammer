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

//! a `string` local pushed into a `Vec` and then used again must not
//! `.clone().clone()`. One clone into `push` is enough.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn compile(ids: Vec<int>) -> string {
    let mut node_vars = Vec::new()
    let mut code = ""
    for id in ids {
        let var_name = format!("node_{}", id)
        node_vars.push(var_name)
        code = code + format!("    let {} = 1;\n", var_name)
    }
    let _kept = node_vars
    code
}
"#;

#[test]
fn module_file_reused_string_push_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED reused string double-cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}

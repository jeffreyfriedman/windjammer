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
))]

//! WDB-297: owned `Vec<string>` CLI formal must not receive demoted `&args`.
//!
//! Product: tip `wave1_cli` emits `wave1_*(&args)` while callees keep owned
//! `args: Vec<String>` (WDB-291/292/294/295). Signature-driven clone/move into
//! owned formals when the binding is reused.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn scale_status_main(args: Vec<string>) -> int {
    let mut owned = args
    owned.push("status".to_string())
    owned.len() as int
}

pub fn run(args: Vec<string>) -> int {
    let n = scale_status_main(args)
    let _ = args.len()
    n
}
"#;

#[test]
fn wdb297_module_file_owned_cli_vec_must_not_receive_ref_args() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-297 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    let owned_formal = rs.contains("fn scale_status_main(args: Vec<String>");
    assert!(
        owned_formal,
        "WDB-297: expected owned Vec formal for scale_status_main:\n{rs}"
    );
    let bad = rs.contains("scale_status_main(&args)");
    assert!(
        !bad,
        "WDB-297 RED: owned Vec CLI formal received &args (must clone/move):\n{rs}"
    );
    // Prefer clone when reused after owned call
    assert!(
        rs.contains("scale_status_main(args.clone())") || rs.contains("scale_status_main(args)"),
        "WDB-297: expected owned/clone call into scale_status_main:\n{rs}"
    );
    test.cargo_check().expect("WDB-297 cargo-check");
}

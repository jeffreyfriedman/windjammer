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
//! Product: tip `wave1_cli` emits `wave1_*(&args)` while relational callees keep
//! owned `args: Vec<String>` (WDB-291/292/294/295). Signature-driven: clone/move
//! into owned formals (or demote the formal consistently).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// Mutating the Vec keeps the formal owned (cannot demote to &Vec).
const MOD: &str = r#"
pub mod cli {
    pub fn scale_status_main(args: Vec<string>) -> int {
        let mut owned = args
        owned.push("status".to_string())
        owned.len() as int
    }
}

pub mod dispatch {
    use crate::cli::scale_status_main

    pub fn run(args: Vec<string>) -> int {
        // Reuse args after the call — requires clone into owned formal.
        let n = scale_status_main(args)
        let _ = args.len()
        n
    }
}
"#;

#[test]
fn wdb297_module_file_owned_cli_vec_must_not_receive_ref_args() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    let map = test.compile().expect("WDB-297 compile");
    let text = map
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    let owned_formal = text.contains("fn scale_status_main(args: Vec<String>")
        || text.contains("scale_status_main(args: Vec<String>");
    assert!(
        owned_formal,
        "WDB-297: expected owned Vec formal for scale_status_main:\n{text}"
    );
    let bad = text.contains("scale_status_main(&args)");
    assert!(
        !bad,
        "WDB-297 RED: owned Vec CLI formal received &args (must clone/move):\n{text}"
    );
    test.cargo_check().expect("WDB-297 cargo-check");
}

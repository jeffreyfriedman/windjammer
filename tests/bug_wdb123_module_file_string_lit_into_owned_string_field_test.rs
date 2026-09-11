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

//! WDB-123: string literal into owned `string` struct field must auto-own (regression).
//!
//! WindjammerDB CQ-C5 product `gen/` was stale with bare `"BFS"` into `String` fields
//! (~180× E0308). Fresh cargo-wj / tip isolate already emits `String::from("BFS")`.
//!
//! Gate: multipass must keep auto-own (GREEN on 0.50.0). Product fix = re-transpile stale gen.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod row
"#;

const ROW: &str = r#"
pub struct BenchRow {
    pub algorithm: string,
    pub run_nanos: u64,
}

pub fn bfs_row(nanos: u64) -> BenchRow {
    BenchRow { algorithm: "BFS", run_nanos: nanos }
}

pub fn named_row(name: string, nanos: u64) -> BenchRow {
    BenchRow { algorithm: name, run_nanos: nanos }
}

pub fn cap() -> BenchRow {
    bfs_row(42)
}
"#;

fn wdb123_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("row.wj", ROW);
    test
}

#[test]
fn wdb123_module_file_string_lit_into_owned_string_field_must_auto_own() {
    let mut test = wdb123_fixture();
    let map = test
        .compile()
        .expect("WDB-123 multipass compile should succeed (codegen may still be wrong)");
    let rs = map.get("row.rs").expect("row.rs must be generated");

    let owned = rs.contains("algorithm: String::from(\"BFS\")")
        || rs.contains("algorithm: \"BFS\".to_string()");
    assert!(
        owned,
        "WDB-123 REGRESSION: owned string field lit must auto-own. Got:\n{rs}"
    );

    test.cargo_check()
        .expect("WDB-123: string lit into owned string field must cargo-check");
}

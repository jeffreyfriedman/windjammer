#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — hexagonal `(Row, T)` column-chain helpers must cargo-check without
//! demoting `Row` to `&mut Row` while still returning owned `Row` (LedgerKit
//! `postgres_row_mapping.wj` / `"col" + ""` workaround).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const ROW: &str = include_str!("fixtures/library_multipass/row_col_chain.wj");
const ADAPTER: &str = include_str!("fixtures/library_multipass/postgres_parse_payment.wj");

fn assert_row_formal_matches_tuple_return(rs: &str) {
    let demoted_mut = rs.contains("fn col_string") && rs.contains("&mut Row");
    let returns_owned = rs.contains("-> (Row, String)") || rs.contains("-> (Row, string)");
    assert!(
        !(demoted_mut && returns_owned),
        "RED: col_string must not take &mut Row while returning owned Row. Got:\n{rs}"
    );
}

#[test]
fn row_col_chain_same_file_must_not_demote_mut_row_and_return_owned() {
    // Single-file tip is GREEN; multipass hexagonal gate below remains RED.
    let rs = test_utils::compile_single(ROW);
    assert_row_formal_matches_tuple_return(&rs);
}

#[test]
fn hexagonal_postgres_row_col_chain_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod row\n");
    project.add_file("domain/row.wj", ROW);
    project.add_file("adapters/mod.wj", "pub mod postgres\n");
    project.add_file("adapters/postgres.wj", ADAPTER);

    let map = project
        .compile()
        .expect("hexagonal row-col chain compile should succeed");
    let row_key = map
        .keys()
        .find(|k| k.ends_with("row.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing row.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let row_rs = map.get(&row_key).expect("row.rs");
    assert_row_formal_matches_tuple_return(row_rs);

    project
        .cargo_check()
        .expect("hexagonal (Row, T) chain without +\"\" must cargo-check");
}

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

//! FAILING REPRO — `Vec<string>` elements must own ints and demoted `&str` locals
//! (LedgerKit `amount + ""` / `tenant_id + ""` query-param workarounds).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const INT_VEC: &str = include_str!("fixtures/library_multipass/vec_string_int_element.wj");
const PORT: &str = include_str!("fixtures/library_multipass/query_vec_string_port.wj");
const ADAPTER: &str = include_str!("fixtures/library_multipass/postgres_query_vec_params.wj");

#[test]
fn vec_string_int_element_must_own_via_to_string() {
    let (rs, ok) = test_utils::compile_single_check(INT_VEC);
    assert!(
        ok,
        "RED: int in Vec<string> must cargo-check (emit .to_string()). Generated:\n{rs}"
    );
    assert!(
        rs.contains("amount.to_string()")
            || rs.contains("amount.to_owned()")
            || rs.contains("format!")
            || rs.contains("String::from"),
        "RED: int element must be owned as String. Got:\n{rs}"
    );
}

#[test]
fn hexagonal_query_vec_string_params_must_own_demoted_str() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod ports\n");
    project.add_file("domain/ports.wj", PORT);
    project.add_file("adapters/mod.wj", "pub mod postgres\n");
    project.add_file("adapters/postgres.wj", ADAPTER);

    let map = project
        .compile()
        .expect("hexagonal query Vec compile should succeed");
    let adapter_key = map
        .keys()
        .find(|k| k.ends_with("postgres.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing postgres.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    let adapter_rs = map.get(&adapter_key).expect("postgres.rs");
    assert!(
        adapter_rs.contains("vec!["),
        "adapter must build params vec; got:\n{adapter_rs}"
    );

    project
        .cargo_check()
        .expect("hexagonal Vec<string> params without +\"\" must cargo-check");
}

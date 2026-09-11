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

//! WDB-135: free/associated fn with owned `string` formal receives bare `"lit"` →
//! E0308 (`expected String, found &str`).
//!
//! WindjammerDB CQ-C5 (`wave1_opt_hardware_port.rs`):
//!   `relational_df_provider_new("wave1_opt_point", 1, 2)` while formal is `String`
//!
//! Distinct from WDB-123 (struct field lit). Expected: `String::from("…")` / `.to_string()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod provider
pub mod use_prov
"#;

const PROVIDER: &str = r#"
pub struct DfProvider {
    pub table_name: string,
    pub table_id: u32,
}

pub fn provider_new(table_name: string, table_id: u32) -> DfProvider {
    DfProvider { table_name: table_name, table_id: table_id }
}
"#;

const USE_PROV: &str = r#"
use crate::provider::provider_new

pub fn make() -> crate::provider::DfProvider {
    provider_new("wave1_opt_point", 1)
}

pub fn cap() -> u32 {
    make().table_id
}
"#;

fn wdb135_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("provider.wj", PROVIDER);
    test.add_file("use_prov.wj", USE_PROV);
    test
}

#[test]
fn wdb135_module_file_owned_string_formal_must_own_string_literal() {
    let test = wdb135_fixture();
    let map = test
        .compile()
        .expect("WDB-135 multipass compile should succeed (codegen may still be wrong)");
    let use_rs = map.get("use_prov.rs").expect("use_prov.rs");

    let owns = use_rs.contains("String::from(\"wave1_opt_point\")")
        || use_rs.contains("\"wave1_opt_point\".to_string()")
        || use_rs.contains("to_owned()");
    let bare = use_rs.contains("provider_new(\"wave1_opt_point\"")
        && !owns;

    eprintln!("WDB-135 use_prov.rs:\n{use_rs}");
    eprintln!("owns={owns} bare={bare}");

    if bare {
        panic!(
            "WDB-135 RED: owned string formal must auto-own string literal. \
             Product: relational_df_provider_new(\"wave1_opt_point\", …)."
        );
    }

    test.cargo_check().expect(
        "WDB-135 RED: String::from lit into owned formal must cargo-check.",
    );
}

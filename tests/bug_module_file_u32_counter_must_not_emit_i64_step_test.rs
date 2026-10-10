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

//! DB-200: `u32` loop counter must not emit `i += 1_i64`.
//!
//! Product residual (~2× u32←i64 + E0277), tip-out/gen df_provider:
//!   `let mut i = 0; while (i as u32) < column_count { …; i += 1_i64 }`
//! → expected `u32`, found `i64`. Literal suffix must match binding type
//! (`1` / `1_u32`) or counter should stay `i64` consistently.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod provider
"#;

const PROVIDER: &str = r#"
pub struct ColMeta {
    pub ordinal: int,
    pub name: string,
}

pub fn provider_new(column_count: int) -> Vec<ColMeta> {
    let mut columns: Vec<ColMeta> = Vec::new()
    let mut i = 0
    while i < column_count {
        columns.push(ColMeta { ordinal: i, name: "c" })
        i = i + 1
    }
    columns
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("provider.wj", PROVIDER);
    test
}

#[test]
fn module_file_u32_counter_must_not_emit_i64_step() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let rs = map.get("provider.rs").expect("provider.rs");
    eprintln!("provider.rs:\n{rs}");

    // Product shape: u32 binding stepped with 1_i64
    let has_u32_i = rs.contains("let mut i: u32") || rs.contains("let mut i = 0") && rs.contains("as u32");
    let bad = rs.contains("i += 1_i64") || rs.contains("i = i + 1_i64");
    if has_u32_i && bad {
        panic!(
            "u32 counter stepped with _i64. \
             Product: relational_df_provider_new i += 1_i64. Got:\n{rs}"
        );
    }
}

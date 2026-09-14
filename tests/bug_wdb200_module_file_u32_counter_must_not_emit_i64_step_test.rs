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

//! WDB-200: `u32` loop counter must not emit `i += 1_i64`.
//!
//! Product residual (~2× u32←i64 + E0277), tip-out/gen df_provider:
//!   `let mut i = 0; while (i as u32) < column_count { …; i += 1_i64 }`
//! → expected `u32`, found `i64`. Literal suffix must match binding type
//! (`1` / `1_u32`) or counter should stay `i64` consistently.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

fn wdb200_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("provider.wj", PROVIDER);
    test
}

#[test]
fn wdb200_module_file_u32_counter_must_not_emit_i64_step() {
    let test = wdb200_fixture();
    let map = test
        .compile()
        .expect("WDB-200 multipass compile should succeed");
    let rs = map.get("provider.rs").expect("provider.rs");
    eprintln!("WDB-200 provider.rs:\n{rs}");

    // Product shape: u32 binding stepped with 1_i64
    let has_u32_i = rs.contains("let mut i: u32") || rs.contains("let mut i = 0") && rs.contains("as u32");
    let bad = rs.contains("i += 1_i64") || rs.contains("i = i + 1_i64");
    if has_u32_i && bad {
        panic!(
            "WDB-200 RED: u32 counter stepped with _i64. \
             Product: relational_df_provider_new i += 1_i64. Got:\n{rs}"
        );
    }
}

#[test]
fn wdb200_tip_out_df_provider_must_not_step_u32_with_i64() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let prov = if tip.join("relational_df_provider_port.rs").exists() {
        tip.join("relational_df_provider_port.rs")
    } else {
        gen.join("relational/relational_df_provider_port.rs")
    };
    if !prov.exists() {
        eprintln!("WDB-200: skip — df_provider missing");
        return;
    }
    let text = std::fs::read_to_string(&prov).expect("provider");
    // Residual: u32-typed or u32-cast loop with explicit 1_i64 step.
    let bad = text.contains("fn relational_df_provider_new")
        && text.contains("i += 1_i64");
    eprintln!("WDB-200 tip-out/gen bad={} path={}", bad, prov.display());
    // Also enforce on product gen path
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational/relational_df_provider_port.rs");
    if gen.exists() {
        let gen_text = std::fs::read_to_string(&gen).unwrap_or_default();
        let gen_bad = gen_text.contains("i += 1_i64");
        eprintln!("WDB-200 product gen bad={}", gen_bad);
        assert!(
            !gen_bad,
            "WDB-200 RED: product gen df_provider steps with 1_i64. {}",
            gen.display()
        );
    }
    assert!(
        !bad,
        "WDB-200 RED: tip-out/product df_provider steps counter with 1_i64. {}",
        prov.display()
    );
}

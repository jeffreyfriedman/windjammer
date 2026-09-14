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

//! WDB-209: `catalog_push_column` must take owned `CatalogColumnBinding`, not `&mut`.
//!
//! Product residual (~44×), gen binder lag vs tip-out:
//!   gen: `col: &mut CatalogColumnBinding` while call sites pass owned `catalog_col(...)`
//!   tip: `col: CatalogColumnBinding` (owned). Prefer tip-out → gen sync.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod binder
"#;

const BINDER: &str = r#"
pub struct CatalogColumnBinding {
    pub logical: string,
    pub physical: string,
}

pub struct CatalogTableBinding {
    pub columns: Vec<CatalogColumnBinding>,
}

fn catalog_push_column(table: CatalogTableBinding, col: CatalogColumnBinding) -> CatalogTableBinding {
    let mut out = table
    out.columns.push(col)
    out
}

pub fn catalog_col(logical: string, physical: string) -> CatalogColumnBinding {
    CatalogColumnBinding { logical: logical, physical: physical }
}
"#;

fn wdb209_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("binder.wj", BINDER);
    test
}

#[test]
fn wdb209_multipass_catalog_push_column_col_must_stay_owned() {
    let test = wdb209_fixture();
    let map = test
        .compile()
        .expect("WDB-209 multipass compile should succeed");
    let binder_rs = map.get("binder.rs").expect("binder.rs");
    eprintln!("WDB-209 binder.rs:\n{binder_rs}");
    let bad = binder_rs.contains("col: &mut CatalogColumnBinding")
        || binder_rs.contains("col: &CatalogColumnBinding");
    assert!(
        !bad,
        "WDB-209 RED: catalog_push_column must keep owned col (Vec::push consumes). Got:\n{binder_rs}"
    );
    assert!(
        binder_rs.contains("out.columns.push(col)") && !binder_rs.contains("push(col.clone())"),
        "WDB-209: push must move col, not clone from demoted formal. Got:\n{binder_rs}"
    );
    test.cargo_check()
        .expect("WDB-209: owned col formal must cargo-check");
}

#[test]
fn wdb209_product_binder_must_not_demote_catalog_col_to_mut_ref() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("relational_sql_binder_port.rs"),
        gen.join("relational/relational_sql_binder_port.rs"),
        gen.join("relational_module_file/relational_sql_binder_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("binder");
        let bad = text.contains("fn catalog_push_column(table: CatalogTableBinding, col: &mut CatalogColumnBinding)")
            || text.contains("col: &mut CatalogColumnBinding) -> CatalogTableBinding");
        eprintln!("WDB-209 bad={} path={}", bad, path.display());
        if path.to_string_lossy().contains("/gen/") {
            assert!(
                !bad,
                "WDB-209 RED: product gen demotes catalog_push_column col to &mut. {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-209: binder missing");
}

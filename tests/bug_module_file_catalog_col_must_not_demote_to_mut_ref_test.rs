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

//!
//!   gen: `col: &mut CatalogColumnBinding` while call sites pass owned `catalog_col(...)`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("binder.wj", BINDER);
    test
}

#[test]
fn multipass_catalog_push_column_col_must_stay_owned() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let binder_rs = map.get("binder.rs").expect("binder.rs");
    eprintln!("binder.rs:\n{binder_rs}");
    let bad = binder_rs.contains("col: &mut CatalogColumnBinding")
        || binder_rs.contains("col: &CatalogColumnBinding");
    assert!(
        !bad,
        "RED: catalog_push_column must keep owned col (Vec::push consumes). Got:\n{binder_rs}"
    );
    assert!(
        binder_rs.contains("out.columns.push(col)") && !binder_rs.contains("push(col.clone())"),
        "push must move col, not clone from demoted formal. Got:\n{binder_rs}"
    );
    test.cargo_check()
        .expect("owned col formal must cargo-check");
}


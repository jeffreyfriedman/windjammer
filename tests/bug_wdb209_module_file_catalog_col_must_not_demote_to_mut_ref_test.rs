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

use std::path::PathBuf;

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

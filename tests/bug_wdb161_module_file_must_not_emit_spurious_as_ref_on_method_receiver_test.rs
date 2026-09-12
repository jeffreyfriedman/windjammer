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

//! WDB-161: full relational `--module-file` must not emit `.as_ref()` on CatalogResolver.
//!
//! Product evidence (tip cold `transpile_relational_module_file`):
//!   `gen/relational_module_file/relational_sql_binder_port.rs` contains
//!   `resolver.as_ref().has_table(...)` → E0599 (~40 in full layers check).
//!
//! Tip-cluster of the same `.wj` alone emits `resolver.has_table(...)` (GREEN).
//! Full multipass of the relational slice still invents `.as_ref()` (RED).
//!
//! Gate A: minimal multipass must not invent `.as_ref()` (regression).
//! Gate B: if product `relational_module_file` binder is present, it must not contain `.as_ref()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod catalog
pub mod bind
"#;

const CATALOG: &str = r#"
pub trait CatalogResolver {
    fn has_table(self, name: string) -> bool
}

pub struct RelationalCatalogResolver {
    pub name: string,
}

impl CatalogResolver for RelationalCatalogResolver {
    fn has_table(self, name: string) -> bool {
        self.name == name
    }
}

pub fn make_resolver() -> RelationalCatalogResolver {
    RelationalCatalogResolver { name: "t" }
}
"#;

const BIND: &str = r#"
use crate::catalog::CatalogResolver
use crate::catalog::RelationalCatalogResolver
use crate::catalog::make_resolver

pub fn resolve_column(resolver: RelationalCatalogResolver, table: string, logical: string) -> bool {
    resolver.has_table(table) && logical.len() > 0
}

pub fn resolve_select(resolver: RelationalCatalogResolver, table: string, n: int) -> bool {
    if !resolver.clone().has_table(table.clone()) {
        return false
    }
    let mut i = 0
    while i < n {
        let ok = resolve_column(resolver.clone(), table.clone(), "c0")
        if !ok {
            return false
        }
        i = i + 1
    }
    true
}

pub fn cap() -> bool {
    resolve_select(make_resolver(), "t", 3)
}
"#;

fn wdb161_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("catalog.wj", CATALOG);
    test.add_file("bind.wj", BIND);
    test
}

#[test]
fn wdb161_module_file_clone_method_receiver_must_not_emit_as_ref() {
    let mut test = wdb161_fixture();
    let map = test
        .compile()
        .expect("WDB-161 multipass compile should succeed (codegen may still be wrong)");
    let bind_rs = map.get("bind.rs").expect("bind.rs");

    eprintln!("WDB-161 minimal bind.rs:\n{bind_rs}");

    assert!(
        !bind_rs.contains(".as_ref()"),
        "WDB-161: minimal multipass must not invent .as_ref() on CatalogResolver receivers."
    );

    test.cargo_check().expect(
        "WDB-161: minimal CatalogResolver receivers must cargo-check without .as_ref().",
    );
}

#[test]
fn wdb161_product_full_relational_module_file_binder_must_not_emit_as_ref() {
    // Product gen is gitignored but present during WindjammerDB dogfooding sessions.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // windjammer/
    path.push("windjammerdb/crates/wdb-layers/gen/relational_module_file/relational_sql_binder_port.rs");

    if !path.exists() {
        eprintln!(
            "WDB-161: skip product gate — {} missing (run transpile_relational_module_file.sh)",
            path.display()
        );
        return;
    }

    let text = std::fs::read_to_string(&path).expect("read product binder gen");
    let as_ref_count = text.matches(".as_ref()").count();
    eprintln!(
        "WDB-161 product binder {} as_ref_count={}",
        path.display(),
        as_ref_count
    );

    assert!(
        as_ref_count == 0,
        "WDB-161 RED: full relational --module-file still emits .as_ref() ({as_ref_count}×) in {}. \
         Tip-cluster of the same binder alone is clean — tip must not invent .as_ref() in full multipass. \
         Product: resolver.as_ref().has_table → E0599.",
        path.display()
    );
}

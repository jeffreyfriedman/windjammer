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

//! WDB-161: demoted `&Resolver` + `resolver.clone().has_table` must not emit `.as_ref()`.
//!
//! Product `resolve_select(resolver: RelationalCatalogResolver, …)` tip-demotes to
//! `&RelationalCatalogResolver`, then rewrites:
//!   `.wj`: `resolver.clone().has_table(...)`
//!   tip:  `resolver.as_ref().has_table(...)` → E0599 (~40).
//!
//! After demotion, method call must be `resolver.has_table(...)` (already `&self`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

pub fn resolve_select(resolver: RelationalCatalogResolver, table: string) -> bool {
    if !resolver.clone().has_table(table.clone()) {
        return false
    }
    let ok = resolve_column(resolver.clone(), table.clone(), "c0")
    ok
}

pub fn cap() -> bool {
    resolve_select(make_resolver(), "t")
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

    eprintln!("WDB-161 bind.rs:\n{bind_rs}");

    let demoted = bind_rs.contains("resolver: &RelationalCatalogResolver")
        || bind_rs.contains("resolver:&RelationalCatalogResolver");

    if bind_rs.contains(".as_ref()") {
        panic!(
            "WDB-161 RED: tip rewrote demoted resolver.clone().has_table → .as_ref().has_table. \
             Product: relational_sql_binder_port resolve_select → E0599 (~40)."
        );
    }

    if demoted {
        // Borrowed formal: trait &self method must call through resolver directly.
        assert!(
            bind_rs.contains("resolver.has_table(") || bind_rs.contains("(*resolver).has_table("),
            "WDB-161 RED: demoted &Resolver must call has_table without .as_ref(). Got:\n{bind_rs}"
        );
    }

    test.cargo_check().expect(
        "WDB-161: demoted CatalogResolver method receivers must cargo-check without .as_ref().",
    );
}

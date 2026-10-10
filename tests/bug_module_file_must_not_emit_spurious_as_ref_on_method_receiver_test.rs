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
//!   `resolver.as_ref().has_table(...)` → E0599 (~40 in full layers check).
//!
//! Cluster of the same `.wj` alone emits `resolver.has_table(...)` (GREEN).
//! Full multipass of the relational slice still invents `.as_ref()` (RED).
//!
//! Gate A: minimal multipass must not invent `.as_ref()` (regression).

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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("catalog.wj", CATALOG);
    test.add_file("bind.wj", BIND);
    test
}

#[test]
fn module_file_clone_method_receiver_must_not_emit_as_ref() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let bind_rs = map.get("bind.rs").expect("bind.rs");

    eprintln!("minimal bind.rs:\n{bind_rs}");

    assert!(
        !bind_rs.contains(".as_ref()"),
        "minimal multipass must not invent .as_ref() on CatalogResolver receivers."
    );
    // Also require cargo-check: demoted `&str` + `.clone()` into owned `String` formals
    // must auto-own (`.to_string()`), not leave E0308.
    test.cargo_check().expect(
        "minimal CatalogResolver receivers must cargo-check (no as_ref; demoted &str owned).",
    );
}


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

//! WDB-161: method calls on owned/locals must not emit spurious `.as_ref()`.
//!
//! WindjammerDB CQ-C5 cold tip gen (`relational_sql_binder_port.rs`):
//!   `resolver.as_ref().has_table(...)` → E0599 AsRef not satisfied (~40 residuals).
//!
//! Idiomatic Windjammer never writes `.as_ref()`; tip must not invent it for method
//! receivers that are already the correct type (owned or `&T`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod catalog
pub mod bind
"#;

const CATALOG: &str = r#"
pub struct Resolver {
    pub name: string,
}

pub fn has_table(resolver: Resolver, table: string) -> bool {
    resolver.name == table
}

pub fn make_resolver() -> Resolver {
    Resolver { name: "t" }
}
"#;

const BIND: &str = r#"
use crate::catalog::Resolver
use crate::catalog::has_table
use crate::catalog::make_resolver

pub fn check(table: string) -> bool {
    let resolver = make_resolver()
    has_table(resolver, table)
}

pub fn check_method(table: string) -> bool {
    let resolver = make_resolver()
    // Prefer free-fn path above; if tip emits method style, must not insert .as_ref()
    has_table(resolver, table)
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
fn wdb161_module_file_must_not_emit_spurious_as_ref_on_method_receiver() {
    let test = wdb161_fixture();
    let map = test
        .compile()
        .expect("WDB-161 multipass compile should succeed (codegen may still be wrong)");
    let bind_rs = map.get("bind.rs").expect("bind.rs");
    let catalog_rs = map.get("catalog.rs").expect("catalog.rs");

    eprintln!("WDB-161 catalog.rs:\n{catalog_rs}\nbind.rs:\n{bind_rs}");

    let spurious = bind_rs.contains(".as_ref()") || catalog_rs.contains(".as_ref()");
    if spurious {
        panic!(
            "WDB-161 RED: tip must not emit .as_ref() on resolver/method receivers. \
             Product: resolver.as_ref().has_table → E0599 (~40)."
        );
    }

    test.cargo_check()
        .expect("WDB-161: resolver call sites must cargo-check without .as_ref().");
}

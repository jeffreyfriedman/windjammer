#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO / product-shape gate — tip must not double-import the same type (E0252).
//!
//! Product cold gen (LedgerKit adapters): tip emits prelude
//!   `use crate::domain::…::InventoryItemView;`
//! and also the source brace import containing the same name → E0252.
//!
//! Product interim (P3.265): omit View/Line types from brace imports (tip auto-imports).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn explicit_type_import_must_not_duplicate_prelude_use() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod ports
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod item\n");
    project.add_file(
        "domain/item.wj",
        include_str!("fixtures/library_multipass/explicit_type_import_must_not_duplicate_prelude.wj"),
    );
    project.add_file("ports/mod.wj", "pub mod item_port\n");
    project.add_file(
        "ports/item_port.wj",
        r#"
use crate::domain::item::ItemView

pub trait ItemPort {
    fn create(self, name: string) -> ItemView
}
"#,
    );
    project.add_file("adapters/mod.wj", "pub mod repo\n");
    project.add_file(
        "adapters/repo.wj",
        r#"
use crate::domain::item::{ItemView, make_view}
use crate::ports::item_port::ItemPort

pub struct SeedRepo {}

impl ItemPort for SeedRepo {
    fn create(self, name: string) -> ItemView {
        make_view(name)
    }
}
"#,
    );

    let map = project
        .compile()
        .expect("duplicate-import fixture compile should succeed");
    let repo_key = map
        .keys()
        .find(|k| k.ends_with("repo.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing repo.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let rs = map.get(&repo_key).expect("repo.rs");

    let item_view_use_lines: Vec<&str> = rs
        .lines()
        .filter(|l| l.contains("ItemView") && l.trim_start().starts_with("use "))
        .collect();

    let duplicate = item_view_use_lines.len() >= 2
        || (rs.contains("use crate::domain::item::ItemView;")
            && rs.contains("use crate::domain::item::{")
            && rs.lines().any(|l| {
                l.contains("use crate::domain::item::{") && l.contains("ItemView")
            }));

    if duplicate {
        eprintln!("RED P3.265 duplicate ItemView imports:\n{rs}");
    }

    assert!(
        !duplicate,
        "RED P3.265: tip must not emit ItemView more than once in use lines. Uses: {item_view_use_lines:?}\n{rs}"
    );

    project.cargo_check().unwrap_or_else(|e| {
        panic!(
            "RED P3.265: hexagonal ItemView import must cargo-check without E0252.\n{e}\nGenerated repo.rs:\n{rs}"
        );
    });
}

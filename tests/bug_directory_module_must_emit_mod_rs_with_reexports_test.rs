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

//! P3.720: Directory modules (`src/inventory/mod.wj` + siblings) must emit
//! `inventory/mod.rs` with `pub mod` + `pub use` re-exports under `--output gen`.
//!
//! Breach Protocol tip transpile writes `gen/inventory/*.rs` but previously omitted (now tip GREEN; host guardrail remains)
//! `gen/inventory/mod.rs`, so `lib.rs` `pub mod inventory` fails with E0583
//! (wj-game restores from stale `build/` or synthesizes thin decls).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn directory_module_must_emit_mod_rs_with_reexports() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod inventory
"#,
    );
    test.add_file(
        "inventory/mod.wj",
        r#"
pub mod item_id
pub mod item
pub use item_id::ItemId
pub use item::Item
"#,
    );
    test.add_file(
        "inventory/item_id.wj",
        r#"
pub struct ItemId {
    pub value: u32,
}
impl ItemId {
    pub fn new(value: u32) -> ItemId {
        ItemId { value: value }
    }
}
"#,
    );
    test.add_file(
        "inventory/item.wj",
        r#"
use crate::inventory::ItemId
pub struct Item {
    pub id: ItemId,
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.720: directory inventory package must transpile");
    let keys: Vec<_> = map.keys().cloned().collect();
    let mod_key = keys
        .iter()
        .find(|k| {
            k.ends_with("inventory/mod.rs")
                || *k == "inventory/mod.rs"
                || k.ends_with("inventory\\mod.rs")
        })
        .cloned();
    assert!(
        mod_key.is_some(),
        "must emit inventory/mod.rs under output; keys={keys:?}"
    );
    let body = map.get(mod_key.as_ref().unwrap()).unwrap();
    assert!(
        body.contains("pub mod item_id") && body.contains("pub mod item"),
        "inventory/mod.rs must declare child modules; got:\n{body}"
    );
    assert!(
        body.contains("pub use") && body.contains("ItemId"),
        "inventory/mod.rs must re-export ItemId; got:\n{body}"
    );
}

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

//! P3.701: Breach Protocol inventory `remove_item` shape.
//!
//! Untyped `let mut removed = 0` plus `Vec<Option<Stack>>` + `u32.min(u32)` must
//! stay U32 through numeric inference — not widen peers to U64 and fail analysis.
//!
//! Dogfood: `breach-protocol/src/inventory/inventory.wj` `remove_item`.
//! Typed `let mut removed: u32 = 0u32` is GREEN (not a product workaround target —
//! tip must accept the untyped zero under u32 return context).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod inv
"#;

const INV: &str = r#"
pub struct Stack {
    quantity: u32,
}

impl Stack {
    pub fn quantity(self) -> u32 {
        self.quantity
    }

    pub fn remove(self, amount: u32) {
        self.quantity = self.quantity - amount
    }

    pub fn is_empty(self) -> bool {
        self.quantity == 0
    }
}

pub struct Inv {
    slots: Vec<Option<Stack>>,
}

impl Inv {
    pub fn remove_item(self, quantity: u32) -> u32 {
        let mut removed = 0
        let mut remaining = quantity
        for i in 0..self.slots.len() {
            if remaining == 0 {
                break
            }
            if let Some(stack) = self.slots[i] {
                let to_remove = remaining.min(stack.quantity())
                let mut new_stack = stack
                new_stack.remove(to_remove)
                removed = removed + to_remove
                remaining = remaining - to_remove
                if new_stack.is_empty() {
                    self.slots[i] = None
                } else {
                    self.slots[i] = Some(new_stack)
                }
            }
        }
        removed
    }
}
"#;

#[test]
fn u32_option_slot_min_untyped_zero_must_not_widen_u64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("inv.wj", INV);
    let map = test
        .compile()
        .expect("P3.701: untyped 0 under -> u32 with Option slots + min must analyze (no U32/U64 conflict)");
    let rs = map.get("inv.rs").expect("inv.rs");
    assert!(
        !rs.contains("_u64"),
        "must not emit u64 peers for u32 inventory remove. Got:\n{rs}"
    );
}

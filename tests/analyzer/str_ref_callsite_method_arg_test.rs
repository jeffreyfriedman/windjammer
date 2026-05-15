#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! Regression: Phase-2 &str parameters and String fields passed to methods that take &str must not
//! get spurious `.to_string()` or miss a leading `&` on owned String fields (E0308).
//!
//! Codegen-registered [`windjammer::codegen::rust::generator::MethodSignature::param_types`]
//! must match analyzer `inferred_param_types` (Phase-2 `Reference(str)`), or call-site coercion
//! used by user-defined methods (`has_item`, etc.) can miss `&` or insert wrong conversions.

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Passthrough call: optimized &str argument forwarded to callee that also uses &str.
#[test]
fn str_ref_parameter_not_to_string_when_callee_accepts_str() {
    let code = r#"
pub struct Inv {}

impl Inv {
    pub fn verify(self, item_id: string, qty: i32) -> bool {
        qty > 0
    }
}

pub struct Shop {
    pub inv: Inv,
}

impl Shop {
    pub fn has_stock(self, item_id: string, qty: i32) -> bool {
        self.inv.verify(item_id, qty)
    }
}

pub fn broker_buy(shop: Shop, item_id: string, qty: i32) -> bool {
    shop.has_stock(item_id, qty)
}

fn main() {}
"#;

    let generated = test_utils::compile_single_result(code).expect("compile");
    assert!(
        generated.contains("fn broker_buy") && generated.contains("item_id: &str"),
        "expected str-ref param in broker_buy;\n{}",
        generated
    );
    assert!(
        !generated.contains("item_id.to_string()"),
        "must not allocate when passing &str to &str;\n{}",
        generated
    );
}

/// Same as above with a negated condition — must not regress to `.to_string()` on the callee arg.
#[test]
fn str_ref_parameter_under_negated_if_no_to_string() {
    let code = r#"
pub struct Inv {}
impl Inv {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        qty > 0
    }
}
pub struct Shop {
    pub inv: Inv,
}
impl Shop {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        self.inv.has_item(item_id, qty)
    }
}
pub fn check(shop: Shop, item_id: string, qty: i32) -> bool {
    if !shop.has_item(item_id, qty) {
        return false
    }
    true
}
fn main() {}
"#;

    let generated = test_utils::compile_single_result(code).expect("compile");
    assert!(
        generated.contains("fn check") && generated.contains("item_id: &str"),
        "expected str-ref parameter;\n{}",
        generated
    );
    assert!(
        !generated.contains("item_id.to_string()"),
        "negated condition must still pass &str through without allocating;\n{}",
        generated
    );
}

/// Owned String field on a loop variable passed to callee that uses &str for that parameter.
#[test]
fn string_field_access_gets_ampersand_for_str_callee_in_method_call() {
    let code = r#"
pub struct Item {
    pub id: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub struct Inv {}

impl Inv {
    pub fn has_item(self, item_id: string, quantity: i32) -> bool {
        quantity >= 1
    }
}

pub struct Session {
    pub offer: Vec<ItemStack>,
}

impl Session {
    pub fn check(self, inv: Inv) -> bool {
        for stack in self.offer {
            if !inv.has_item(stack.item.id, stack.quantity) {
                return false
            }
        }
        true
    }
}

fn main() {}
"#;

    let generated = test_utils::compile_single_result(code).expect("compile");
    assert!(
        generated.contains("has_item(&stack.item.id"),
        "owned String field must borrow as &str for callee;\n{}",
        generated
    );
}

/// Same field shape as trading session: verify every stack loop gets `&` on `stack.item.id`
/// regardless of receiver type (Inventory vs forwarding Merchant).
#[test]
fn duplicate_stack_loops_both_ampersand_on_owned_id_field() {
    let code = r#"
pub struct Item {
    pub id: string,
}
pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}
pub struct Inv {}
impl Inv {
    pub fn has_item(self, item_id: string, quantity: i32) -> bool {
        quantity >= 1
    }
}
pub struct Seller {
    pub inv: Inv,
}
impl Seller {
    pub fn has_item(self, item_id: string, quantity: i32) -> bool {
        self.inv.has_item(item_id, quantity)
    }
}
pub struct Cart {
    pub a: Vec<ItemStack>,
    pub b: Vec<ItemStack>,
}
impl Cart {
    pub fn check(self, buyer: Inv, seller: Seller) -> bool {
        for stack in self.a {
            if !buyer.has_item(stack.item.id, stack.quantity) {
                return false
            }
        }
        for stack in self.b {
            if !seller.has_item(stack.item.id, stack.quantity) {
                return false
            }
        }
        true
    }
}
fn main() {}
"#;

    let generated = test_utils::compile_single_result(code).expect("compile");
    let cnt = generated.matches("has_item(&stack.item.id").count();
    assert_eq!(
        cnt, 2,
        "both loops must borrow owned id field; got {} occurrences in:\n{}",
        cnt, generated
    );
}

/// Mirror breach-protocol merchant → inventory forwarding + dual has_item loops (buy_item + execute_trade edge cases).
#[test]
fn str_ref_forwarding_merchant_inventory_and_duplicate_field_calls() {
    let code = r#"
pub struct Item {
    pub id: string,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub struct Inventory {}

impl Inventory {
    pub fn has_item(self, item_id: string, quantity: i32) -> bool {
        if quantity < 1 {
            return false
        }
        true
    }
}

pub struct Merchant {
    pub inv: Inventory,
}

impl Merchant {
    pub fn has_item(self, item_id: string, quantity: i32) -> bool {
        self.inv.has_item(item_id, quantity)
    }
}

pub fn buy_item(merchant: Merchant, item_id: string, qty: i32) -> bool {
    merchant.has_item(item_id, qty)
}

pub fn exec_trade(
    player: Inventory,
    merchant: Merchant,
    player_offer: Vec<ItemStack>,
    merchant_offer: Vec<ItemStack>
) -> bool {
    for stack in player_offer {
        if !player.has_item(stack.item.id, stack.quantity) {
            return false
        }
    }
    for stack in merchant_offer {
        if !merchant.has_item(stack.item.id, stack.quantity) {
            return false
        }
    }
    true
}

fn main() {}
"#;

    let generated = test_utils::compile_single_result(code).expect("compile");
    assert!(
        generated.contains("fn buy_item") && generated.contains("item_id: &str"),
        "expected buy_item parameter lowered to &str;\n{}",
        generated
    );
    assert!(
        !generated.contains("item_id.to_string()"),
        "must not add .to_string() when forwarding &str to &str;\n{}",
        generated
    );
    assert!(
        generated.contains("merchant.has_item(&stack.item.id")
            || generated.contains("has_item(&stack.item.id"),
        "merchant branch must borrow owned id field;\n{}",
        generated
    );
}

/// When another type defines `has_item` with incompatible string semantics, passthrough inference
/// must still resolve through the correct receiver (`Inventory::has_item`), not ambiguous `has_item`.
#[test]
fn passthrough_uses_receiver_qualified_has_item_under_name_collision() {
    let code = r#"
pub struct Inventory {}

impl Inventory {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        qty >= 1
    }
}

pub struct Decoy {}

impl Decoy {
    /// Different shape / usage — registering another `has_item` must not confuse passthrough.
    pub fn has_item(self, label: string, min: i32) -> bool {
        min >= 99
    }
}

pub struct Shelf {
    pub stock: Inventory,
}

impl Shelf {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        self.stock.has_item(item_id, qty)
    }
}

pub fn check_item(shelf: Shelf, item_id: string, qty: i32) -> bool {
    shelf.has_item(item_id, qty)
}

fn main() {}
"#;

    let generated = test_utils::compile_single_result(code).expect("compile");
    assert!(
        generated.contains("fn check_item") && generated.contains("item_id: &str"),
        "expected optimized string param;\n{}",
        generated
    );
    assert!(
        !generated.contains("item_id.to_string()"),
        "must not emit .to_string() when callee also takes borrowed text;\n{}",
        generated
    );
}

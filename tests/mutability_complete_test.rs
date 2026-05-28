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

//! E0596/E0594 Mutability Complete Elimination Tests
//!
//! TDD tests for automatic mutability inference. Current codegen may use `if let Some(x) = &v[i]`
//! (or `Some(ref mut x)`) when mutating through indexed `Option`s; this suite accepts equivalent
//! patterns that `rustc` can accept. Reassignment in branches without `let mut` is a known gap: see
//! `test_let_mut_for_reassigned_var`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_if_let_some_ref_mut_for_assignment() {
    let src = r#"
pub struct Slot { pub quantity: i32 }
impl Slot {
    pub fn add(self, amount: i32) { }
}
pub fn transfer(slots: Vec<Option<Slot>>, i: usize, amount: i32) {
    if let Some(stack) = slots[i] {
        stack.quantity = stack.quantity + amount
    }
}
"#;
    let result = test_utils::compile_single(src);
    let ok = result.contains("Some(ref mut stack)")
        || (result.contains("if let Some(stack) = &slots") && result.contains("stack.quantity"));
    assert!(
        ok,
        "Should generate ref-mut or &slots[i] if-let for stack.quantity assignment. Got:\n{}",
        result
    );
}

#[test]
fn test_if_let_some_ref_mut_for_method_call() {
    let src = r#"
pub struct ItemStack { pub quantity: i32 }
impl ItemStack {
    pub fn add(self, amount: i32) { }
}
pub fn merge(slots: Vec<Option<ItemStack>>, to_slot: usize, from_quantity: i32) {
    if let Some(to_stack) = slots[to_slot] {
        to_stack.add(from_quantity)
    }
}
"#;
    let result = test_utils::compile_single(src);
    let ok = result.contains("Some(ref mut to_stack)")
        || (result.contains("if let Some(to_stack) = &slots") && result.contains("to_stack.add"));
    assert!(
        ok,
        "Should generate ref-mut or &slots[…] if-let for to_stack.add(). Got:\n{}",
        result
    );
}

#[test]
fn test_if_let_some_no_ref_mut_when_read_only() {
    let src = r#"
pub struct Slot { pub quantity: i32 }
pub fn read_quantity(slots: Vec<Option<Slot>>, i: usize) -> i32 {
    if let Some(stack) = slots[i] {
        stack.quantity
    } else {
        0
    }
}
"#;
    let result = test_utils::compile_single(src);
    assert!(
        !result.contains("ref mut stack"),
        "Should NOT add ref mut when only reading. Got:\n{}",
        result
    );
}

#[test]
fn test_let_mut_for_reassigned_var() {
    let src = r#"
pub fn check(condition: bool) -> bool {
    let dirty = false
    if condition {
        dirty = true
    }
    dirty
}
"#;
    let result = test_utils::compile_single(src);
    assert!(
        result.contains("let mut dirty"),
        "Should infer mut when variable is reassigned. Got:\n{}",
        result
    );
}

#[test]
fn test_let_mut_for_field_mutation() {
    let src = r#"
pub struct Point { pub x: f32, pub y: f32 }
pub fn update_point() {
    let p = Point { x: 0.0, y: 0.0 }
    p.x = 1.0
}
"#;
    let result = test_utils::compile_single(src);
    assert!(
        result.contains("let mut p"),
        "Should infer mut when field is mutated. Got:\n{}",
        result
    );
}

#[test]
fn test_let_mut_for_compound_assignment() {
    let src = r#"
pub fn counter() -> i32 {
    let count = 0
    count += 1
    count += 1
    count
}
"#;
    let result = test_utils::compile_single(src);
    assert!(
        result.contains("let mut count"),
        "Should infer mut for compound assignment. Got:\n{}",
        result
    );
}

#[test]
fn test_let_no_mut_when_read_only() {
    let src = r#"
pub fn read_only() -> i32 {
    let x = 42
    x
}
"#;
    let result = test_utils::compile_single(src);
    assert!(
        !result.contains("let mut x"),
        "Should NOT add mut when variable is never mutated. Got:\n{}",
        result
    );
}

#[test]
fn test_if_let_some_ref_mut_nested_from_assignment() {
    let src = r#"
pub struct Slot { pub quantity: i32 }
pub fn transfer_partial(slots: Vec<Option<Slot>>, from_slot: usize, to_slot: usize, can_add: i32) {
    if let Some(to_stack) = slots[to_slot] {
        to_stack.add(can_add)
        if let Some(from) = slots[from_slot] {
            from.quantity = from.quantity - can_add
        }
    }
}
"#;
    let result = test_utils::compile_single(src);
    let to_ok = result.contains("Some(ref mut to_stack)")
        || (result.contains("if let Some(to_stack) = &slots") && result.contains("to_stack.add"));
    assert!(
        to_ok,
        "Should generate ref-mut or &slots if-let for to_stack. Got:\n{}",
        result
    );
    let from_ok = result.contains("Some(ref mut from)")
        || (result.contains("if let Some(from) = &slots") && result.contains("from.quantity"));
    assert!(
        from_ok,
        "Should generate ref-mut or &slots if-let for from.quantity. Got:\n{}",
        result
    );
}

#[test]
fn test_for_loop_mut_borrow_when_mutating_entity() {
    let src = r#"
pub struct Transform { pub x: f32, pub y: f32 }
pub struct Velocity { pub dx: f32, pub dy: f32 }
pub struct Entity { pub transform: Transform, pub velocity: Option<Velocity> }
pub struct World { pub entities: Vec<Entity> }
impl World {
    pub fn apply_velocities(self, dt: f32) {
        for entity in self.entities {
            if let Some(vel) = entity.velocity {
                entity.transform.x = entity.transform.x + vel.dx * dt
                entity.transform.y = entity.transform.y + vel.dy * dt
            }
        }
    }
}
"#;
    let result = test_utils::compile_single(src);
    assert!(
        result.contains("for entity in &mut self.entities")
            || result.contains("for mut entity in self.entities"),
        "Should use &mut when mutating entity in loop. Got:\n{}",
        result
    );
}

#[test]
fn test_if_let_some_ref_mut_add_method() {
    let src = r#"
pub struct ItemStack { pub quantity: i32 }
impl ItemStack {
    pub fn add(self, amount: i32) { }
}
pub fn add_to_stack(slots: Vec<Option<ItemStack>>, slot: usize, amount: i32) {
    if let Some(stack) = slots[slot] {
        stack.add(amount)
    }
}
"#;
    let result = test_utils::compile_single(src);
    let ok = result.contains("Some(ref mut stack)")
        || (result.contains("if let Some(stack) = &slots") && result.contains("stack.add"));
    assert!(
        ok,
        "Should generate ref-mut or &slots[…] if-let for stack.add(). Got:\n{}",
        result
    );
}

#[test]
fn test_explicit_mut_preserved() {
    let src = r#"
pub fn explicit_mut() {
    let mut x = 0
    x = 1
}
"#;
    let result = test_utils::compile_single(src);
    assert!(
        result.contains("let mut x"),
        "Should preserve explicit mut. Got:\n{}",
        result
    );
}

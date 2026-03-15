//! E0596/E0594 Mutability Complete Elimination Tests
//!
//! TDD tests for automatic mutability inference:
//! - if let Some(ref mut x) when body mutates through x
//! - let mut x when variable is reassigned
//! - for entity in &mut self.entities when body mutates entity

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile(src: &str) -> String {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, src).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
    let wj_binary = if wj_binary.exists() {
        wj_binary
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/wj")
    };

    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        panic!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generated_file = test_dir.join("build/test.rs");
    std::fs::read_to_string(&generated_file).expect("Failed to read generated file")
}

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
    let result = compile(src);
    assert!(
        result.contains("Some(ref mut stack)"),
        "Should generate ref mut for stack.quantity assignment. Got:\n{}",
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
    let result = compile(src);
    assert!(
        result.contains("Some(ref mut to_stack)"),
        "Should generate ref mut for to_stack.add() call. Got:\n{}",
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
    let result = compile(src);
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
    let result = compile(src);
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
    let result = compile(src);
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
    let result = compile(src);
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
    let result = compile(src);
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
    let result = compile(src);
    assert!(
        result.contains("Some(ref mut to_stack)"),
        "Should generate ref mut for to_stack. Got:\n{}",
        result
    );
    assert!(
        result.contains("Some(ref mut from)"),
        "Should generate ref mut for from.quantity. Got:\n{}",
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
    let result = compile(src);
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
    let result = compile(src);
    assert!(
        result.contains("Some(ref mut stack)"),
        "Should generate ref mut for stack.add(). Got:\n{}",
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
    let result = compile(src);
    assert!(
        result.contains("let mut x"),
        "Should preserve explicit mut. Got:\n{}",
        result
    );
}

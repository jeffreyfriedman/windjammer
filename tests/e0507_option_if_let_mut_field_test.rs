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

//! E0507 fix: Option if-let with Vec index + &mut self must NOT generate &mut & (double ref).
//!
//! Pattern: if let Some(stack) = self.slots[i] { stack.quantity -= 1 }
//! When self is &mut self, expression gen produces &self.slots[i] (Index auto-borrow).
//! We then add &mut prefix → &mut &self.slots[i] (WRONG - cannot move out of &mut &Option).
//!
//! Correct: &mut self.slots[i] (strip leading & before adding &mut).

#[path = "common/test_utils.rs"]
mod test_utils;

use std::process::Command;

fn rust_compiles(rust_code: &str) -> bool {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let rs_path = temp_dir.path().join("test.rs");
    std::fs::write(&rs_path, rust_code).expect("write");
    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .arg(&rs_path)
        .output()
        .expect("rustc");
    output.status.success()
}

#[test]
fn test_option_if_let_vec_index_mut_self_no_double_ref() {
    // E0507 fix: if let Some(stack) = self.slots[i] { stack.add(1) }
    // Expression gen produces &self.slots[i] (Index auto-borrow for non-Copy).
    // Must NOT generate &mut &self.slots[i] - strip leading & before adding &mut.
    let source = r#"
pub struct Item { pub id: string }
pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}
impl ItemStack {
    pub fn add(self, n: i32) {
        self.quantity = self.quantity + n
    }
}
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub capacity: i32,
}
impl Inventory {
    pub fn add_item(self, item: Item, quantity: i32) -> bool {
        let mut i = 0
        while i < self.capacity {
            if let Some(stack) = self.slots[i as usize] {
                if stack.item.id == item.id && stack.quantity + quantity <= 100 {
                    stack.add(quantity)
                    return true
                }
            }
            i = i + 1
        }
        false
    }
}
fn main() {}
"#;
    let rust = test_utils::compile_single_result(source).expect("compile");
    // Must NOT contain double ref (E0507: cannot move out of mutable reference)
    assert!(
        !rust.contains("&mut &self"),
        "Option if-let on Vec index must NOT generate &mut &: {}",
        rust
    );
    // Should generate &mut self.slots[...] (single mutable borrow)
    assert!(
        rust.contains("&mut self.slots[") || rust.contains("&mut self.slots ["),
        "Should generate &mut self.slots[i]: {}",
        rust
    );
    assert!(
        rust_compiles(&rust),
        "Generated Rust must compile: {}",
        rust
    );
}

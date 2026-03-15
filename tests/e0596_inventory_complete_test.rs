//! TDD: E0596 Phase 11 Regression - if let Some(x) = self.slots[i] { x.mutate() }
//!
//! Root cause: infer_match_bound_types always returned is_mut_ref=false for Index expressions,
//! causing type ascription `let stack: &ItemStack = stack` to overwrite `ref mut stack` (which
//! correctly gives &mut ItemStack) with immutable &ItemStack → E0596.
//!
//! Fix: For Index on &mut self.field, check base (self) in inferred_mut_borrowed_params.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_e0596_inv_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

/// E0596 fix: if let Some(stack) = self.slots[i] { stack.add(q) } → ref mut + &mut type ascription
#[test]
fn test_inventory_option_if_let_mut_field() {
    let source = r#"
pub struct Item {
    pub id: string,
    pub stackable: bool,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn can_add(self, q: i32) -> bool {
        self.quantity + q <= 100
    }
    pub fn add(self, q: i32) {
        self.quantity = self.quantity + q
    }
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub capacity: i32,
}

impl Inventory {
    pub fn new(capacity: i32) -> Inventory {
        let mut slots = Vec::new()
        let mut i = 0
        while i < capacity {
            slots.push(None)
            i = i + 1
        }
        Inventory { slots, capacity }
    }
    pub fn add_item(self, item: Item, quantity: i32) -> bool {
        if item.stackable {
            let mut i = 0
            while i < self.capacity {
                if let Some(stack) = self.slots[i as usize] {
                    if stack.item.id == item.id && stack.can_add(quantity) {
                        stack.add(quantity)
                        return true
                    }
                }
                i = i + 1
            }
        }
        false
    }
}

pub fn main() {}
"#;

    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile without E0596. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("Some(ref mut stack)"),
        "Should generate ref mut for mutated Option binding. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("&mut ") && (rs.contains("stack: &mut ") || rs.contains("let stack: &mut ")),
        "Type ascription must be &mut ItemStack, not &ItemStack. Generated:\n{}",
        rs
    );
}

/// E0596 fix: stack.quantity -= x pattern (field mutation)
#[test]
fn test_option_match_mut_field_quantity() {
    let source = r#"
pub struct Item { pub id: string }
pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub capacity: i32,
}

impl Inventory {
    pub fn new(cap: i32) -> Inventory {
        let mut slots = Vec::new()
        let mut i = 0
        while i < cap {
            slots.push(None)
            i = i + 1
        }
        Inventory { slots, capacity: cap }
    }
    pub fn remove_by_id(self, item_id: string, quantity: i32) -> bool {
        let mut remaining = quantity
        let mut i = 0
        while i < self.capacity && remaining > 0 {
            if let Some(stack) = self.slots[i as usize] {
                if stack.item.id == item_id {
                    if stack.quantity <= remaining {
                        remaining = remaining - stack.quantity
                        self.slots[i as usize] = None
                    } else {
                        stack.quantity = stack.quantity - remaining
                        remaining = 0
                    }
                }
            }
            i = i + 1
        }
        remaining == 0
    }
}

pub fn main() {}
"#;

    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile without E0596. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("Some(ref mut stack)"),
        "Should generate ref mut. Generated:\n{}",
        rs
    );
}

/// to_stack.add() - move_item pattern
#[test]
fn test_move_item_to_stack_add() {
    let source = r#"
pub struct Item { pub id: string, pub stackable: bool, pub max_stack: i32 }
pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

impl ItemStack {
    pub fn add(self, q: i32) {
        self.quantity = self.quantity + q
    }
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub capacity: i32,
}

impl Inventory {
    pub fn new(cap: i32) -> Inventory {
        let mut slots = Vec::new()
        let mut i = 0
        while i < cap {
            slots.push(None)
            i = i + 1
        }
        Inventory { slots, capacity: cap }
    }
    pub fn move_item(self, from_slot: i32, to_slot: i32) -> bool {
        if from_slot < 0 || to_slot < 0 || from_slot >= self.capacity || to_slot >= self.capacity {
            return false
        }
        if from_slot == to_slot {
            return false
        }
        let from_data = match &self.slots[from_slot as usize] {
            Some(from_stack) => Some((from_stack.item.id.clone(), from_stack.quantity)),
            _ => None,
        }
        if let Some((from_id, from_q)) = from_data {
            if let Some(to_stack) = self.slots[to_slot as usize] {
                if from_id == to_stack.item.id && to_stack.item.stackable {
                    let can_add = to_stack.item.max_stack - to_stack.quantity
                    if can_add >= from_q {
                        to_stack.add(from_q)
                        self.slots[from_slot as usize] = None
                        return true
                    }
                }
            }
        }
        false
    }
}

pub fn main() {}
"#;

    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile without E0596 (to_stack.add). Generated:\n{}",
        rs
    );
}

/// Read-only: if let Some(stack) = &self.slots[i] - should use ref not ref mut
#[test]
fn test_inventory_read_only_uses_ref() {
    let source = r#"
pub struct Item { pub id: string }
pub struct ItemStack { pub item: Item, pub quantity: i32 }

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub capacity: i32,
}

impl Inventory {
    pub fn new(cap: i32) -> Inventory {
        let mut slots = Vec::new()
        let mut i = 0
        while i < cap {
            slots.push(None)
            i = i + 1
        }
        Inventory { slots, capacity: cap }
    }
    pub fn has_item(self, item_id: string, quantity: i32) -> bool {
        let mut total = 0
        let mut i = 0
        while i < self.capacity {
            if let Some(stack) = self.slots[i as usize] {
                if stack.item.id == item_id {
                    total = total + stack.quantity
                }
            }
            i = i + 1
        }
        total >= quantity
    }
}

pub fn main() {}
"#;

    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("Some(ref stack)") && !rs.contains("Some(ref mut stack)"),
        "Read-only should use ref not ref mut. Generated:\n{}",
        rs
    );
}

/// Equipment slot - if let Some(stack) = &self.head (read-only)
#[test]
fn test_equipment_read_only_ref() {
    let source = r#"
pub struct Item { pub health: i32 }
pub struct ItemStack { pub item: Item }

pub struct Equipment {
    pub head: Option<ItemStack>,
}

impl Equipment {
    pub fn total_health(self) -> i32 {
        let mut total = 0
        if let Some(stack) = self.head {
            total = total + stack.item.health
        }
        total
    }
}

pub fn main() {}
"#;

    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(
        compiles,
        "Should compile. Generated:\n{}",
        rs
    );
}

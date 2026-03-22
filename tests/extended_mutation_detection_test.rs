/// TDD Test: Extended mutation detection for .take(), .push(), .insert(), etc.
///
/// Bug: E0596 - cannot borrow as mutable (17 errors in windjammer-game)
/// Root Cause: is_mutating_method() doesn't detect Option::take(), Vec::push(), etc.
/// Fix: Add pattern-based detection for common stdlib mutating methods.
///
/// Philosophy: "Compiler does hard work" - automatic &mut self inference
use std::fs;
use tempfile::TempDir;

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    let output_dir = test_dir.join("build");
    fs::write(&input_file, code).expect("Failed to write source file");

    windjammer::build_project(
        &input_file,
        &output_dir,
        windjammer::CompilationTarget::Rust,
        true,
    )
    .map_err(|e| format!("Windjammer compilation failed: {}", e))?;

    let generated_file = output_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");
    Ok(generated)
}

#[test]
fn test_take_method_infers_mut_self() {
    let code = r#"
pub struct Container {
    pub value: Option<i32>,
}

impl Container {
    pub fn extract_value(self) -> Option<i32> {
        // .take() mutates self.value - should infer &mut self
        self.value.take()
    }
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    // Should have &mut self (not &self)
    assert!(
        generated.contains("pub fn extract_value(&mut self)"),
        "extract_value() should infer &mut self because self.value.take() mutates.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_push_method_infers_mut_self() {
    let code = r#"
pub struct Buffer {
    pub items: Vec<i32>,
}

impl Buffer {
    pub fn add_item(self, item: i32) {
        self.items.push(item)
    }
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("pub fn add_item(&mut self, item: i32)"),
        "add_item() should infer &mut self because self.items.push() mutates.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_insert_method_infers_mut_self() {
    let code = r#"
use std::collections::HashMap

pub struct Cache {
    pub data: HashMap<string, i32>,
}

impl Cache {
    pub fn store(self, key: string, value: i32) {
        self.data.insert(key, value)
    }
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("pub fn store(&mut self, key: "),
        "store() should infer &mut self because self.data.insert() mutates.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_clear_method_infers_mut_self() {
    let code = r#"
pub struct List {
    pub items: Vec<i32>,
}

impl List {
    pub fn remove_all(self) {
        self.items.clear()
    }
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("pub fn remove_all(&mut self)"),
        "remove_all() should infer &mut self because self.items.clear() mutates.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_pop_method_infers_mut_self() {
    let code = r#"
pub struct Stack {
    pub items: Vec<i32>,
}

impl Stack {
    pub fn pop_item(self) -> Option<i32> {
        self.items.pop()
    }
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("pub fn pop_item(&mut self)"),
        "pop_item() should infer &mut self because self.items.pop() mutates.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_indexed_field_take_infers_mut_self() {
    // Real-world pattern from inventory.wj: self.slots[i].take()
    let code = r#"
pub struct SlotContainer {
    pub slots: Vec<Option<i32>>,
}

impl SlotContainer {
    pub fn remove_at(self, index: usize) -> Option<i32> {
        self.slots[index].take()
    }
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("pub fn remove_at(&mut self, index: usize)"),
        "remove_at() should infer &mut self because self.slots[index].take() mutates.\nGenerated:\n{}",
        generated
    );
}

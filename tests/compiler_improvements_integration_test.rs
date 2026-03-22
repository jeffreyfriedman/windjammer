//! TDD Integration Test: All 3 compiler improvements work together
//!
//! Validates that generic type propagation, trait ownership inference,
//! and extended mutation detection work correctly in combination.

use std::fs;
use tempfile::TempDir;

fn compile_windjammer(source: &str) -> Result<String, String> {
    let temp = TempDir::new().map_err(|e| e.to_string())?;
    let source_file = temp.path().join("test.wj");
    let output_dir = temp.path().join("build");
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    fs::write(&source_file, source).map_err(|e| e.to_string())?;

    windjammer::build_project(
        &source_file,
        &output_dir,
        windjammer::CompilationTarget::Rust,
        true,
    )
    .map_err(|e| format!("Compilation failed: {}", e))?;

    let rust_file = output_dir.join("test.rs");
    fs::read_to_string(&rust_file).map_err(|e| format!("Failed to read: {}", e))
}

#[test]
fn test_all_compiler_improvements_work_together() {
    // Code using all 3 improvements:
    // 1. Generic type parameter (identity<T>)
    // 2. Trait impl ownership (Container trait)
    // 3. Mutation detection (.push(), .take())
    let source = r#"
pub fn identity<T>(value: T) -> T {
    value
}

pub trait Container {
    fn add_item(self, item: int)
    fn get_item(self) -> Option<int>
}

pub struct ItemList {
    pub items: Vec<int>,
    pub last: Option<int>,
}

impl Container for ItemList {
    fn add_item(self, item: int) {
        self.items.push(item)
    }

    fn get_item(self) -> Option<int> {
        self.last.take()
    }
}

pub fn test_all() {
    let x = identity(42)
    let mut list = ItemList { items: Vec::new(), last: None }
    list.add_item(1)
    list.add_item(2)
    list.last = Some(42)
    let item = list.get_item()
}
"#;

    let generated = compile_windjammer(source).expect("Compilation should succeed");

    // Check improvement 1: Generic preserved
    assert!(
        generated.contains("fn identity<T>") || generated.contains("pub fn identity<T>"),
        "Generic function should preserve <T>. Generated:\n{}",
        generated
    );

    // Check improvement 2 + 3: Trait impl has &mut self
    assert!(
        generated.contains("fn add_item(&mut self, item: "),
        "add_item() should infer &mut self (push mutates). Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn get_item(&mut self) -> Option<"),
        "get_item() should infer &mut self (take mutates). Generated:\n{}",
        generated
    );
}

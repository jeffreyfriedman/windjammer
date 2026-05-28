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

//! TDD Integration Test: All 3 compiler improvements work together
//!
//! Validates that generic type propagation, trait ownership inference,
//! and extended mutation detection work correctly in combination.

#[path = "common/test_utils.rs"]
mod test_utils;

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

    let generated = test_utils::compile_single_result(source).expect("Compilation should succeed");

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

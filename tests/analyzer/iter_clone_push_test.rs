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

//! TDD Test: Auto-clone iterator variable when pushing to Vec that expects owned
//!
//! When iterating over borrowed collection and pushing to a new Vec that will
//! be assigned to a field, we need to clone the iterator variable.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_filter_push_clone() {
    // When filtering a Vec and pushing to new Vec, need to clone
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    id: i32,
    name: string,
}

pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn remove_item(&mut self, target_id: i32) {
        let mut new_items = Vec::new()
        for item in self.items {
            if item.id != target_id {
                new_items.push(item)
            }
        }
        self.items = new_items
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // The item should be cloned when pushed
    assert!(
        generated.contains("item.clone()") || generated.contains(".clone()"),
        "Should clone iterator variable. Generated:\n{}",
        generated
    );
    assert!(success, "Generated code should compile. Error: {}", err);
}

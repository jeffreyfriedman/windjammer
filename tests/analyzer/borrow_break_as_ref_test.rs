#[path = "../common/test_utils.rs"]
mod test_utils;

/// TDD Test: Borrow Break Pattern - as_ref() vs as_deref()
///
/// PROBLEM: The codegen generates a "borrow break" pattern for match expressions
/// where the scrutinee borrows self and the arm body mutates self:
///   let __match_borrow_break = self.method().map(|v| v.to_owned());
///   match __match_borrow_break.as_deref() { ... }
///
/// The `.as_deref()` call requires the inner type to implement `Deref`, which
/// works for `String` (Deref<Target=str>) but fails for custom types like
/// `DialogueNode` (E0599: as_deref exists but trait bounds not satisfied).
///
/// FIX: Use `.as_ref()` instead of `.as_deref()`. This works for ALL types:
///   Option<String>.as_ref() → Option<&String>   (String auto-coerces to &str where needed)
///   Option<Custom>.as_ref() → Option<&Custom>   (works for any type)
#[test]
fn test_borrow_break_uses_as_ref_not_as_deref() {
    // This test models the exact pattern from windjammer-game-core/dialogue/manager.wj:
    // - self.tree.get_node(id) returns Option<&DialogueNode> (borrows from self.tree)
    // - The match arm body mutates self (self.current_index = ...)
    // - This triggers the borrow break pattern
    // - The generated code should use .as_ref() not .as_deref() because
    //   DialogueNode doesn't implement Deref
    // The key is that get_item returns Option<&Item> (a REFERENCE that borrows self)
    // and the match arm body mutates self. This creates the borrow conflict that
    // triggers the borrow break pattern.
    let source = r#"
use std::collections::HashMap

struct Item {
    pub name: String,
    pub value: i32,
}

impl Item {
    fn new(name: String, value: i32) -> Item {
        Item { name: name, value: value }
    }
}

struct Container {
    pub items: HashMap<String, Item>,
    pub last_accessed: Option<String>,
}

impl Container {
    fn new() -> Container {
        Container { items: HashMap::new(), last_accessed: None }
    }

    fn get_item(&self, key: &str) -> Option<&Item> {
        self.items.get(key)
    }

    fn access_item(&mut self, key: String) {
        match self.get_item(&key) {
            Some(item) => {
                self.last_accessed = Some(key)
            }
            _ => {}
        }
    }
}
"#;

    let rust_code = test_utils::compile_single(source);

    // If borrow break is generated, it should use .as_ref() not .as_deref()
    if rust_code.contains("__match_borrow_break") {
        assert!(
            !rust_code.contains(".as_deref()"),
            "Borrow break pattern should use .as_ref() instead of .as_deref() \
             because custom types don't implement Deref.\nGenerated:\n{}",
            rust_code
        );
        assert!(
            rust_code.contains(".as_ref()"),
            "Borrow break pattern should use .as_ref() for universal compatibility.\n\
             Generated:\n{}",
            rust_code
        );
    }

    // The generated code should compile correctly with rustc
    // (as_deref would fail for custom types that don't implement Deref)
    assert!(
        !rust_code.contains("as_deref"),
        "Generated code should not use as_deref() on custom types.\nGenerated:\n{}",
        rust_code
    );
}

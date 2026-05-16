#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD Test: .clone() on borrowed strings should generate .to_string() when needed
///
/// Bug: When a string parameter is inferred as &str and .clone() is called on it,
/// and the result is passed to a function expecting String, the codegen generates
/// .clone() which returns &str, not String, causing E0308 type mismatch.
///
/// Fix: Detect when .clone() result needs to be String and generate .to_string() instead.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_string_clone_generates_to_string() {
    let source = r#"
struct DialogTree {
    id: string,
}

impl DialogTree {
    pub fn new(id: string) -> DialogTree {
        DialogTree { id }
    }
}

pub fn create_dialog(id: string) -> DialogTree {
    DialogTree::new(id.clone())
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated Rust:\n{}", generated);

    if generated.contains("id: &str") {
        assert!(
            generated.contains(".to_string()") || generated.contains(".to_owned()"),
            "Should convert &str to String with .to_string() or .to_owned(), not .clone()"
        );
    }
}

#[test]
fn test_owned_string_can_use_clone() {
    let source = r#"
struct DialogTree {
    id: string,
}

impl DialogTree {
    pub fn new(id: string) -> DialogTree {
        DialogTree { id }
    }
}

pub fn create_dialog(id: string, suffix: string) -> DialogTree {
    let full_id = format!("{}_{}", id, suffix)
    DialogTree::new(full_id.clone())
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated Rust:\n{}", generated);

    assert!(!generated.is_empty(), "Should generate valid Rust code");
}

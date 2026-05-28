#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

// Test: HashMap/HashSet in comments should NOT trigger auto-import
//
// Bug: The compiler uses `format!("{:?}", program)` to scan for HashMap/HashSet usage,
// which matches text in comments (e.g., "HashMap-like"). This causes
// `use std::collections::HashMap;` to be emitted even when no HashMap is used in code.
//
// Root cause: Debug representation includes comment text, so string matching on it
// produces false positives.
//
// Fix: Walk the AST properly to detect actual type usage, not debug text.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_in_comment_does_not_trigger_import() {
    let source = r#"
/// ComponentArray - Stores components of a single type
/// Uses sparse set architecture (HashMap-like lookup)
struct Counter {
    count: i64,
}

fn main() {
    let c = Counter { count: 0 }
    println!("{}", c.count)
}
"#;

    let rust_code = test_utils::compile_single(source);

    assert!(
        !rust_code.contains("use std::collections::HashMap"),
        "HashMap import should NOT be generated when HashMap only appears in comments!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashset_in_comment_does_not_trigger_import() {
    let source = r#"
// Uses HashSet internally for deduplication
struct UniqueList {
    items: Vec<i64>,
}

fn main() {
    let list = UniqueList { items: Vec::new() }
    println!("{}", list.items.len())
}
"#;

    let rust_code = test_utils::compile_single(source);

    assert!(
        !rust_code.contains("use std::collections::HashSet"),
        "HashSet import should NOT be generated when HashSet only appears in comments!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_in_actual_code_does_trigger_import() {
    let source = r#"
struct Registry {
    items: HashMap<String, i64>,
}

fn main() {
    let reg = Registry { items: HashMap::new() }
    println!("registry created")
}
"#;

    let rust_code = test_utils::compile_single(source);

    assert!(
        rust_code.contains("use std::collections::HashMap"),
        "HashMap import SHOULD be generated when HashMap is used as a type!\nGenerated:\n{}",
        rust_code
    );
}

/// Test: HashMap/HashSet in comments should NOT trigger auto-import
///
/// Bug: The compiler uses `format!("{:?}", program)` to scan for HashMap/HashSet usage,
/// which matches text in comments (e.g., "HashMap-like"). This causes
/// `use std::collections::HashMap;` to be emitted even when no HashMap is used in code.
///
/// Root cause: Debug representation includes comment text, so string matching on it
/// produces false positives.
///
/// Fix: Walk the AST properly to detect actual type usage, not debug text.
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_wj_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj")
}

fn compile_wj_source(test_name: &str, source: &str) -> String {
    let wj_binary = get_wj_binary();
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("test_{}", test_name));

    fs::create_dir_all(&test_dir).unwrap();

    let test_file = test_dir.join(format!("{}.wj", test_name));
    fs::write(&test_file, source).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join(format!("{}.rs", test_name));
    let rust_code = fs::read_to_string(&rust_file)
        .unwrap_or_else(|e| panic!("Failed to read generated Rust file {:?}: {}", rust_file, e));
    println!("Generated Rust:\n{}", rust_code);
    rust_code
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_in_comment_does_not_trigger_import() {
    // This source mentions "HashMap" only in a comment, never in actual code
    let source = r#"
/// ComponentArray - Stores components of a single type
/// Uses sparse set architecture (HashMap-like lookup)
struct Counter {
    count: int
}

fn main() {
    let c = Counter { count: 0 };
    println!("{}", c.count);
}
"#;

    let rust_code = compile_wj_source("hashmap_comment_false_positive", source);

    // Should NOT contain HashMap import since HashMap is only in a comment
    assert!(
        !rust_code.contains("use std::collections::HashMap"),
        "HashMap import should NOT be generated when HashMap only appears in comments!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashset_in_comment_does_not_trigger_import() {
    // This source mentions "HashSet" only in a comment
    let source = r#"
// Uses HashSet internally for deduplication
struct UniqueList {
    items: Vec<int>
}

fn main() {
    let list = UniqueList { items: Vec::new() };
    println!("{}", list.items.len());
}
"#;

    let rust_code = compile_wj_source("hashset_comment_false_positive", source);

    // Should NOT contain HashSet import
    assert!(
        !rust_code.contains("use std::collections::HashSet"),
        "HashSet import should NOT be generated when HashSet only appears in comments!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_in_actual_code_does_trigger_import() {
    // This source actually uses HashMap as a type
    let source = r#"
struct Registry {
    items: HashMap<string, int>
}

fn main() {
    let reg = Registry { items: HashMap::new() };
    println!("registry created");
}
"#;

    let rust_code = compile_wj_source("hashmap_actual_usage", source);

    // SHOULD contain HashMap import since HashMap is used as a real type
    assert!(
        rust_code.contains("use std::collections::HashMap"),
        "HashMap import SHOULD be generated when HashMap is used as a type!\nGenerated:\n{}",
        rust_code
    );
}

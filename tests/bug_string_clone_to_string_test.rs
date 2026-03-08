/// TDD Test: .clone() on borrowed strings should generate .to_string() when needed
/// 
/// Bug: When a string parameter is inferred as &str and .clone() is called on it,
/// and the result is passed to a function expecting String, the codegen generates
/// .clone() which returns &str, not String, causing E0308 type mismatch.
///
/// Fix: Detect when .clone() result needs to be String and generate .to_string() instead.

use std::process::Command;
use std::fs;

#[test]
fn test_string_clone_generates_to_string() {
    // Windjammer code where id is inferred as &str, but DialogTree::new expects String
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

    // Write temporary file
    let temp_file = "/tmp/test_string_clone.wj";
    fs::write(temp_file, source).unwrap();

    // Compile with Windjammer
    let output = Command::new("wj")
        .args(&["build", "--output", "/tmp", "--target", "rust", temp_file])
        .output()
        .expect("Failed to run wj");

    let generated = fs::read_to_string("/tmp/test_string_clone.rs").unwrap();
    
    // The generated code should use .to_string() or .to_owned(), not .clone()
    // when passing &str to a function expecting String
    println!("Generated Rust:\n{}", generated);
    
    // Check that Rust compilation succeeds
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error[E0308]"),
        "Should not have type mismatch error. Stderr:\n{}",
        stderr
    );
    
    // Verify the generated code has proper string conversion
    // When id is &str and needs to be String, should use .to_string() not .clone()
    if generated.contains("id: &str") {
        assert!(
            generated.contains(".to_string()") || generated.contains(".to_owned()"),
            "Should convert &str to String with .to_string() or .to_owned(), not .clone()"
        );
    }
}

#[test]
fn test_owned_string_can_use_clone() {
    // When parameter is owned String, .clone() is correct
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

    let temp_file = "/tmp/test_owned_string.wj";
    fs::write(temp_file, source).unwrap();

    let output = Command::new("wj")
        .args(&["build", "--output", "/tmp", "--target", "rust", temp_file])
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error[E0308]"),
        "Should not have type mismatch error when cloning owned String. Stderr:\n{}",
        stderr
    );
}

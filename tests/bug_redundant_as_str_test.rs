/// TDD Test: Redundant .as_str() on inferred &str parameters
///
/// Bug: When a string parameter is inferred as &str, calling .as_str() on it
/// generates code that tries to call the unstable String::as_str() method,
/// even though it's already &str and doesn't need conversion.
///
/// Fix: Detect when .as_str() is called on an already-&str type and either:
/// 1. Remove the .as_str() call entirely (preferred)
/// 2. Or recognize it's a no-op and generate appropriate code

use std::process::Command;
use std::fs;

#[test]
fn test_no_as_str_on_borrowed_string() {
    // When parameter is inferred as &str, .as_str() should be removed or recognized as no-op
    let source = r#"
enum AssetType {
    Texture,
    Audio,
    Unknown,
}

impl AssetType {
    pub fn from_extension(ext: string) -> AssetType {
        match ext.as_str() {
            "png" => AssetType::Texture,
            "wav" => AssetType::Audio,
            _ => AssetType::Unknown,
        }
    }
}
"#;

    let temp_file = "/tmp/test_as_str.wj";
    fs::write(temp_file, source).unwrap();

    let output = Command::new("wj")
        .args(&["build", "--output", "/tmp", "--target", "rust", temp_file])
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should not have E0658 unstable feature error
    assert!(
        !stderr.contains("error[E0658]"),
        "Should not generate unstable .as_str() call on &str. Stderr:\n{}",
        stderr
    );
    
    // Should not have any compilation errors
    assert!(
        !stderr.contains("error[E"),
        "Should compile without errors. Stderr:\n{}",
        stderr
    );
    
    let generated = fs::read_to_string("/tmp/test_as_str.rs").unwrap();
    println!("Generated Rust:\n{}", generated);
    
    // When ext is &str, should either:
    // 1. Not have .as_str() at all (preferred)
    // 2. Or match directly on ext without .as_str()
    if generated.contains("ext: &str") {
        // If we're matching on ext.as_str(), that's the bug
        // It should just be "match ext {" or the .as_str() should be removed
        let has_redundant_as_str = generated.contains("match ext.as_str()");
        assert!(
            !has_redundant_as_str,
            "Should not call .as_str() on &str parameter. Generated:\n{}",
            generated
        );
    }
}

#[test]
fn test_match_on_string_directly() {
    // Matching directly on string should work
    let source = r#"
enum FileType {
    Text,
    Binary,
}

impl FileType {
    pub fn classify(ext: string) -> FileType {
        match ext {
            "txt" => FileType::Text,
            "bin" => FileType::Binary,
            _ => FileType::Binary,
        }
    }
}
"#;

    let temp_file = "/tmp/test_match_string.wj";
    fs::write(temp_file, source).unwrap();

    let output = Command::new("wj")
        .args(&["build", "--output", "/tmp", "--target", "rust", temp_file])
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("error[E"),
        "Should compile without errors. Stderr:\n{}",
        stderr
    );
}

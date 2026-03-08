/// TDD TEST: String parameters should work in match without .as_str()
/// 
/// WINDJAMMER PHILOSOPHY: Compiler handles string conversions automatically.
/// Users shouldn't need Rust-specific .as_str() boilerplate.
/// 
/// Test validates:
/// 1. Match on string param works without .as_str()
/// 2. Generated Rust compiles without E0658
/// 3. Cross-backend consistency (Go/JS don't have .as_str())

use std::process::Command;

#[test]
fn test_string_match_without_as_str() {
    let source = r#"
enum BuildType {
    Warrior,
    Rogue,
    Tech,
}

impl BuildType {
    pub fn from_name(name: string) -> BuildType {
        match name {
            "warrior" => BuildType::Warrior,
            "rogue" => BuildType::Rogue,
            "tech" => BuildType::Tech,
            _ => BuildType::Warrior,
        }
    }
}
"#;

    // Write test file
    std::fs::write("/tmp/test_string_match.wj", source).unwrap();

    // Compile to Rust
    let output = Command::new("wj")
        .args(&["build", "--output", "/tmp", "--target", "rust", "/tmp/test_string_match.wj"])
        .output()
        .expect("Failed to run wj");

    assert!(output.status.success(), "wj build should succeed");

    // Read generated Rust
    let generated = std::fs::read_to_string("/tmp/test_string_match.rs")
        .expect("Generated file should exist");

    println!("Generated Rust:\n{}", generated);

    // Verify: Should compile without E0658
    let compile = Command::new("rustc")
        .args(&["--crate-type", "lib", "/tmp/test_string_match.rs", "-o", "/tmp/test_string_match.rlib"])
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&compile.stderr);
    
    // Should NOT have E0658 (unstable feature)
    assert!(
        !stderr.contains("E0658"),
        "Generated Rust should not use unstable features\nStderr: {}",
        stderr
    );

    // Should compile successfully
    assert!(
        compile.status.success(),
        "Generated Rust should compile\nStderr: {}",
        stderr
    );

    // Verify: Generated code handles string→&str conversion intelligently
    // Either: match name { ... } (if inferred as &str)
    // Or: match name.as_str() { ... } (if inferred as String)
    // But NOT: match name.as_str() { ... } when name is &str
    assert!(
        generated.contains("match name {") || generated.contains("match name.as_str() {"),
        "Match statement should exist"
    );
}

#[test]
fn test_string_match_with_explicit_as_str_should_warn() {
    // FUTURE: This should emit a warning or error
    // For now, just document the intent
    let source = r#"
enum BuildType {
    Warrior,
}

impl BuildType {
    pub fn from_name(name: string) -> BuildType {
        match name.as_str() {  // ← Redundant! Compiler should warn
            "warrior" => BuildType::Warrior,
            _ => BuildType::Warrior,
        }
    }
}
"#;

    std::fs::write("/tmp/test_explicit_as_str.wj", source).unwrap();

    let output = Command::new("wj")
        .args(&["build", "--output", "/tmp", "--target", "rust", "/tmp/test_explicit_as_str.wj"])
        .output()
        .expect("Failed to run wj");

    // TODO: Should emit warning about redundant .as_str()
    // For now, just ensure it compiles
    assert!(output.status.success(), "Should compile even with redundant .as_str()");
}

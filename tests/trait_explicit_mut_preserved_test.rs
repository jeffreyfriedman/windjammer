/// TDD Test: Trait methods with explicit &mut self should preserve it
///
/// BUG: When a trait method explicitly declares `fn init(&mut self)`,
/// the compiler infers `&self` if the body doesn't mutate fields.
///
/// EXPECTED: Explicit `&mut self` in trait definitions should ALWAYS be preserved,
/// regardless of what the body does.
use std::process::Command;
use tempfile;

#[test]
fn test_trait_explicit_mut_self_preserved() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let src_wj = temp_dir.path().join("src_wj");
    std::fs::create_dir_all(&src_wj).unwrap();

    // Trait with explicit &mut self that doesn't mutate
    let wj_code = r#"
pub trait GameLoop {
    // Explicit &mut self should be preserved even though body doesn't mutate
    fn init(&mut self) {
        // Empty default implementation
    }
    
    // Also test &self is preserved
    fn render(&self) {
        // Empty default implementation
    }
}
"#;

    let input_file = src_wj.join("mod.wj");
    std::fs::write(&input_file, wj_code).unwrap();

    let output_dir = temp_dir.path().join("out");
    let compile_result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute compiler");

    if !compile_result.status.success() {
        eprintln!(
            "STDOUT:\n{}",
            String::from_utf8_lossy(&compile_result.stdout)
        );
        eprintln!(
            "STDERR:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Compiler failed");
    }

    let generated_rust = std::fs::read_to_string(output_dir.join("mod.rs"))
        .or_else(|_| std::fs::read_to_string(output_dir.join("lib.rs")))
        .expect("Failed to read generated Rust");

    println!(
        "=== Generated Rust ===\n{}\n=====================",
        generated_rust
    );

    // CRITICAL: Explicit &mut self should be preserved
    assert!(
        generated_rust.contains("fn init(&mut self)"),
        "Trait definition should preserve explicit &mut self!\nGenerated:\n{}",
        generated_rust
    );

    // CRITICAL: Explicit &self should also be preserved
    assert!(
        generated_rust.contains("fn render(&self)"),
        "Trait definition should preserve explicit &self!\nGenerated:\n{}",
        generated_rust
    );
}

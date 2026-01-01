/// TDD Test: Trait methods with explicit &mut self should preserve it
///
/// BUG: When a trait method explicitly declares `fn init(&mut self)`,
/// the compiler infers `&self` if the body doesn't mutate fields.
///
/// EXPECTED: Explicit `&mut self` in trait definitions should ALWAYS be preserved,
/// regardless of what the body does.
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
#[cfg_attr(target_os = "windows", ignore = "Hangs on Windows CI - investigating")]
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

    eprintln!("🔧 trait_explicit_mut_preserved: Running compiler");
    eprintln!("   Binary: {}", env!("CARGO_BIN_EXE_wj"));
    eprintln!("   Input: {}", input_file.display());
    eprintln!("   Output: {}", output_dir.display());

    let compile_result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&input_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute compiler");

    // Add small delay for file I/O to complete (especially on Windows)
    std::thread::sleep(std::time::Duration::from_millis(100));

    eprintln!("   Exit code: {:?}", compile_result.status.code());
    eprintln!("   STDOUT length: {} bytes", compile_result.stdout.len());
    eprintln!("   STDERR length: {} bytes", compile_result.stderr.len());

    if !compile_result.status.success() {
        eprintln!(
            "STDOUT:\n{}",
            String::from_utf8_lossy(&compile_result.stdout)
        );
        eprintln!(
            "STDERR:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        eprintln!("   ⚠️ Checking if output files exist:");
        if let Ok(entries) = std::fs::read_dir(&output_dir) {
            for entry in entries.flatten() {
                eprintln!("     - {}", entry.path().display());
            }
        } else {
            eprintln!("     Output dir doesn't exist!");
        }
        panic!(
            "Compiler failed with exit code {:?}",
            compile_result.status.code()
        );
    }

    // Retry logic for file reading to handle I/O race conditions
    let mut generated_rust = String::new();
    let mut retries = 3;
    while retries > 0 {
        if let Ok(content) = std::fs::read_to_string(output_dir.join("mod.rs"))
            .or_else(|_| std::fs::read_to_string(output_dir.join("lib.rs")))
        {
            if !content.is_empty() {
                generated_rust = content;
                break;
            }
            eprintln!("   ⚠️ File empty, waiting 100ms before retry...");
        } else {
            eprintln!("   ⚠️ File not found, waiting 100ms before retry...");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        retries -= 1;
    }

    if generated_rust.is_empty() {
        panic!("Failed to read generated Rust after retries");
    }

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

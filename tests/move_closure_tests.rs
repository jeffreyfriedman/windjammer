//! Tests for auto-move closures
//!
//! Windjammer Philosophy: The compiler does the work, not the developer.
//! All closures automatically emit `move` in generated Rust - no explicit
//! keyword needed from the user!

use std::path::PathBuf;
use std::process::Command;

/// Helper to compile a test fixture and return the generated Rust code
fn compile_fixture(fixture_name: &str) -> Result<String, String> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}.wj", fixture_name));

    // Use unique output dir per fixture to avoid race conditions in parallel tests
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_output")
        .join(fixture_name);
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

    eprintln!("🔧 move_closures: Compiling {}", fixture_name);
    eprintln!("   Fixture: {}", fixture_path.display());
    eprintln!("   Output: {}", output_dir.display());
    eprintln!("   Binary: {}", env!("CARGO_BIN_EXE_wj"));

    let compiler_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            fixture_path.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;

    eprintln!("   Exit code: {:?}", compiler_output.status.code());
    eprintln!("   STDOUT: {} bytes", compiler_output.stdout.len());
    eprintln!("   STDERR: {} bytes", compiler_output.stderr.len());

    if !compiler_output.status.success() {
        eprintln!("   Compiler FAILED!");
        eprintln!(
            "STDERR:\n{}",
            String::from_utf8_lossy(&compiler_output.stderr)
        );
        return Err(format!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&compiler_output.stderr)
        ));
    }

    let rust_file = output_dir.join(format!("{}.rs", fixture_name));
    eprintln!("   Reading: {}", rust_file.display());
    eprintln!("   Exists: {}", rust_file.exists());

    // Retry logic to handle file I/O race conditions
    let mut retries = 3;
    let mut last_error = String::new();

    while retries > 0 {
        if rust_file.exists() {
            if let Ok(metadata) = std::fs::metadata(&rust_file) {
                eprintln!("   Size: {} bytes", metadata.len());

                // If file exists but is empty, wait and retry
                if metadata.len() == 0 {
                    eprintln!("   ⚠️ File is empty, waiting 100ms before retry...");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    retries -= 1;
                    continue;
                }
            }
        } else {
            eprintln!("   ⚠️ FILE DOES NOT EXIST!");
            if let Ok(entries) = std::fs::read_dir(&output_dir) {
                eprintln!("   Files in output dir:");
                for entry in entries.flatten() {
                    eprintln!("     - {}", entry.path().display());
                }
            }
        }

        match std::fs::read_to_string(&rust_file) {
            Ok(content) if !content.is_empty() => return Ok(content),
            Ok(_) => {
                eprintln!("   ⚠️ File read but empty, waiting 100ms before retry...");
                std::thread::sleep(std::time::Duration::from_millis(100));
                retries -= 1;
            }
            Err(e) => {
                last_error = format!("Failed to read generated code: {}", e);
                eprintln!("   ⚠️ Read error: {}, waiting 100ms before retry...", e);
                std::thread::sleep(std::time::Duration::from_millis(100));
                retries -= 1;
            }
        }
    }

    Err(format!("File I/O race condition: {}", last_error))
}

#[test]
fn test_closures_auto_generate_move() {
    let generated = compile_fixture("move_closures").expect("Compilation failed");

    // ALL closures should generate `move` automatically
    // This is the Windjammer philosophy - the compiler infers what the developer shouldn't need to write

    // Check that closures generate `move` without user needing to write it
    assert!(
        generated.contains("move ||") || generated.contains("move |"),
        "Closures should auto-generate 'move' keyword. Generated:\n{}",
        generated
    );

    // Verify thread blocks also use move (they already did)
    assert!(
        generated.contains("std::thread::spawn(move ||"),
        "Thread blocks should use 'move'. Generated:\n{}",
        generated
    );

    println!("✓ Windjammer auto-moves closures - no explicit 'move' keyword needed!");
}

#[test]
fn test_no_explicit_move_keyword_needed() {
    // This test verifies the Windjammer philosophy:
    // The developer writes: |x| x + 1
    // We generate: move |x| x + 1
    //
    // The developer writes: thread { ... }
    // We generate: std::thread::spawn(move || { ... })
    //
    // NO explicit 'move' keyword ever needed!

    let generated = compile_fixture("move_closures").expect("Compilation failed");

    // Count how many `move` keywords appear - should be multiple (auto-generated)
    let move_count = generated.matches("move").count();
    assert!(
        move_count >= 2,
        "Expected multiple auto-generated 'move' keywords, found {}. Generated:\n{}",
        move_count,
        generated
    );

    println!("✓ Found {} auto-generated 'move' keywords", move_count);
}

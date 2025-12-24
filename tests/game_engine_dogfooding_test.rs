/// Test: Game Engine Dogfooding
///
/// This test compiles the game-host engine (written in Windjammer)
/// to ensure the compiler can handle real-world, complex codebases.
///
/// This is a critical dogfooding test that validates:
/// - Multi-file compilation
/// - Module system
/// - Trait implementations
/// - Complex type inference
/// - Real-world code patterns
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_game_engine_compiles() {
    let game_engine_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("game-host/game-host-core");

    if !game_engine_path.exists() {
        eprintln!("⚠️  Game engine not found at {:?}", game_engine_path);
        eprintln!("   Skipping test (this is expected in CI)");
        return;
    }

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    if !wj_binary.exists() {
        panic!(
            "❌ Windjammer compiler not found at {:?}. Run `cargo build --release` first.",
            wj_binary
        );
    }

    // Step 1: Compile Windjammer source to Rust
    println!("🔨 Compiling game engine from Windjammer to Rust...");
    let output = Command::new(&wj_binary)
        .arg("build")
        .arg("src_wj") // Build the directory, not a single file
        .arg("--no-cargo") // Build to default build/ directory
        .current_dir(&game_engine_path)
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        eprintln!("❌ Windjammer compilation failed!");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("Windjammer compilation failed");
    }

    println!("✅ Windjammer compilation succeeded");

    // Step 2: Compile generated Rust code
    println!("🔨 Compiling generated Rust code...");
    let build_dir = game_engine_path.join("build");
    let cargo_output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&build_dir) // Build in the build/ directory
        .output()
        .expect("Failed to execute cargo");

    if !cargo_output.status.success() {
        eprintln!("❌ Cargo build failed!");
        eprintln!("stdout: {}", String::from_utf8_lossy(&cargo_output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&cargo_output.stderr));

        // Print first 50 errors for debugging
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        let errors: Vec<&str> = stderr
            .lines()
            .filter(|line| line.contains("error[E"))
            .take(50)
            .collect();

        eprintln!("\n📋 First {} errors:", errors.len());
        for error in errors {
            eprintln!("  {}", error);
        }

        panic!("Cargo build failed - see errors above");
    }

    println!("✅ Cargo build succeeded!");
    println!("🎉 Game engine compilation complete!");
}

#[test]
#[ignore] // Run with: cargo test --release --test game_engine_dogfooding_test -- --ignored
fn test_game_engine_compiles_and_runs() {
    // TODO: Once the engine compiles, add a test that runs it
    // This would validate that the generated code not only compiles but executes correctly
}

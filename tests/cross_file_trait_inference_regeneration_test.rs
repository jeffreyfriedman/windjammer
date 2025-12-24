/// TDD Test: Cross-File Trait Inference with Regeneration
///
/// Tests that when a trait has default implementations and an impl in another file
/// mutates self, the regenerated trait file correctly updates the signature to &mut self.
///
/// This is THE WINDJAMMER WAY: The compiler infers the most permissive trait signature
/// from ALL implementations across the project.
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_trait_signature_updates_on_regeneration() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    fs::create_dir_all(&src_dir).unwrap();

    // Create game_loop.wj - trait with default implementation
    let game_loop_wj = r#"
pub trait GameLoop {
    fn update(self, delta: f32) {
        // Default: do nothing
    }
}
"#;
    fs::write(src_dir.join("game_loop.wj"), game_loop_wj).unwrap();

    // Create my_game.wj - impl that mutates self
    let my_game_wj = r#"
use crate::game_loop::GameLoop

pub struct MyGame {
    pub frame_count: int,
}

impl GameLoop for MyGame {
    fn update(self, delta: f32) {
        self.frame_count = self.frame_count + 1
    }
}
"#;
    fs::write(src_dir.join("my_game.wj"), my_game_wj).unwrap();

    // First compilation
    let output_dir = temp_dir.path().join("build");
    let result =
        windjammer::build_project(&src_dir, &output_dir, windjammer::CompilationTarget::Rust);

    assert!(
        result.is_ok(),
        "First compilation failed: {:?}",
        result.err()
    );

    // Check the generated game_loop.rs
    let game_loop_rs = fs::read_to_string(output_dir.join("game_loop.rs")).unwrap();
    println!("Generated game_loop.rs:\n{}", game_loop_rs);

    // THE WINDJAMMER WAY: The trait should have been updated to &mut self
    // because the MyGame impl mutates self.frame_count
    assert!(
        game_loop_rs.contains("fn update(&mut self, delta: f32)")
            || game_loop_rs.contains("fn update(&mut self,delta:f32)"), // Allow no space
        "Trait signature should be updated to &mut self. Got:\n{}",
        game_loop_rs
    );

    // Check my_game.rs has the correct impl signature
    let my_game_rs = fs::read_to_string(output_dir.join("my_game.rs")).unwrap();
    assert!(
        my_game_rs.contains("fn update(&mut self, delta: f32)"),
        "Impl should have &mut self"
    );

    // Verify it compiles
    let cargo_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&output_dir)
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);
        println!("Cargo build failed:\n{}", stderr);

        // Check for E0053 errors specifically
        if stderr.contains("error[E0053]") {
            panic!("E0053: Trait signature mismatch - cross-file inference not working");
        }

        panic!("Generated Rust code should compile");
    }
}

#[test]
fn test_trait_param_inference_across_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src_wj");
    fs::create_dir_all(&src_dir).unwrap();

    // Create input.wj
    let input_wj = r#"
pub struct Input {
    pub mouse_x: f32,
}
"#;
    fs::write(src_dir.join("input.wj"), input_wj).unwrap();

    // Create game_loop.wj - trait that takes Input by value
    let game_loop_wj = r#"
use crate::input::Input

pub trait GameLoop {
    fn process_input(self, input: Input) {
        // Default: do nothing
    }
}
"#;
    fs::write(src_dir.join("game_loop.wj"), game_loop_wj).unwrap();

    // Create my_game.wj - impl that only reads from input (doesn't need ownership)
    let my_game_wj = r#"
use crate::game_loop::GameLoop
use crate::input::Input

pub struct MyGame {
    pub last_mouse_x: f32,
}

impl GameLoop for MyGame {
    fn process_input(self, input: Input) {
        self.last_mouse_x = input.mouse_x
    }
}
"#;
    fs::write(src_dir.join("my_game.wj"), my_game_wj).unwrap();

    // Compile
    let output_dir = temp_dir.path().join("build");
    let result =
        windjammer::build_project(&src_dir, &output_dir, windjammer::CompilationTarget::Rust);
    assert!(result.is_ok(), "Compilation failed: {:?}", result.err());

    // THE WINDJAMMER WAY: The trait should infer &Input since the impl only reads
    let game_loop_rs = fs::read_to_string(output_dir.join("game_loop.rs")).unwrap();
    println!("Generated game_loop.rs:\n{}", game_loop_rs);

    // For Copy types, Windjammer may keep it as owned (Input), but for non-Copy,
    // it should infer the most permissive (which is & if only reading)
    // Since we're reading input.mouse_x, the inference should determine if Input needs &

    // Verify it compiles without E0053 errors
    let cargo_output = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(&output_dir)
        .output()
        .expect("Failed to run cargo build");

    if !cargo_output.status.success() {
        let stderr = String::from_utf8_lossy(&cargo_output.stderr);

        if stderr.contains("error[E0053]") {
            println!("E0053 error found:\n{}", stderr);
            panic!("Trait parameter inference failed - signature mismatch");
        }

        println!("Other compilation error:\n{}", stderr);
    }

    assert!(
        cargo_output.status.success(),
        "Generated code should compile"
    );
}

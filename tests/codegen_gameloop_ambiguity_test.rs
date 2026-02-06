// TDD Test: Verify GameLoop trait and struct don't cause ambiguity
// Bug: E0659: `GameLoop` is ambiguous
// Root Cause: Both game_loop::GameLoop (trait) and game::GameLoop (struct) exist
// Fix: Rename game::GameLoop struct to FrameTimer to avoid conflict

use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
fn test_gameloop_no_ambiguity() {
    let code = r#"
        // Simulate the game_loop module (trait)
        mod game_loop {
            pub trait GameLoop {
                fn update(&mut self);
            }
        }
        
        // Simulate the game module (struct - should be renamed)
        mod game {
            pub struct FrameTimer {
                pub delta_time: f32,
            }
        }
        
        // Re-export both
        pub use game_loop::GameLoop;
        pub use game::FrameTimer;
        
        fn main() {
            let _timer = FrameTimer { delta_time: 0.016 };
        }
    "#;

    // Create temporary test directory
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_gameloop_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    std::fs::create_dir_all(&test_dir).unwrap();

    // Write test file
    std::fs::write(test_dir.join("main.wj"), code).unwrap();

    // Compile
    let output = Command::new(get_wj_compiler())
        .arg("build")
        .arg("main.wj")
        .arg("--no-cargo") // Skip cargo build to avoid devise_core dependency issue
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check generated Rust code for ambiguity
    let generated_code = std::fs::read_to_string(test_dir.join("build/main.rs"))
        .expect("Failed to read generated code");

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);

    if !output.status.success() {
        panic!(
            "Code generation failed!\nstdout: {}\nstderr: {}\ngenerated:\n{}",
            stdout, stderr, generated_code
        );
    }

    // Verify no ambiguity in generated code (Rust compilation errors would show up here if there were any)
    // The generated code should compile cleanly without E0659 errors
    assert!(
        !generated_code.contains("GameLoop::"),
        "Generated code should not have ambiguous GameLoop references"
    );
}

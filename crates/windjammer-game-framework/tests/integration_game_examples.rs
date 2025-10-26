//! Integration tests for game framework examples
//! Validates that all examples compile and run correctly

use std::process::Command;

/// Test that the window example compiles
#[test]
fn test_window_example_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--example",
            "window_test",
            "-p",
            "windjammer-game-framework",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Window example failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that the sprite example compiles
#[test]
fn test_sprite_example_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--example",
            "sprite_test",
            "-p",
            "windjammer-game-framework",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Sprite example failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that the physics example compiles and runs (headless)
#[test]
fn test_physics_example_runs() {
    // Physics example doesn't require a display, so we can run it
    let run_output = Command::new("cargo")
        .args([
            "run",
            "--example",
            "physics_test",
            "-p",
            "windjammer-game-framework",
        ])
        .output()
        .expect("Failed to execute cargo run");

    // Check that it ran successfully
    assert!(
        run_output.status.success(),
        "Physics example failed to run:\n{}",
        String::from_utf8_lossy(&run_output.stderr)
    );

    // Check for expected output
    let stdout = String::from_utf8_lossy(&run_output.stdout);
    assert!(
        stdout.contains("Physics world created"),
        "Missing physics initialization"
    );
    assert!(stdout.contains("Ball"), "Missing ball creation");
    assert!(stdout.contains("Ground created"), "Missing ground creation");
    assert!(
        stdout.contains("Simulation complete"),
        "Simulation didn't complete"
    );
}

/// Test that the game loop example compiles
#[test]
fn test_game_loop_example_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--example",
            "game_loop_test",
            "-p",
            "windjammer-game-framework",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Game loop example failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that rendering example compiles
#[test]
fn test_rendering_example_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--example",
            "rendering_test",
            "-p",
            "windjammer-game-framework",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Rendering example failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that audio example compiles (with audio feature)
#[test]
fn test_audio_example_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--example",
            "audio_test",
            "-p",
            "windjammer-game-framework",
            "--features",
            "audio",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Audio example failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that physics example compiles (without running)
#[test]
fn test_physics_example_compiles() {
    let output = Command::new("cargo")
        .args([
            "build",
            "--example",
            "physics_test",
            "-p",
            "windjammer-game-framework",
        ])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Physics example failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that all examples compile together
#[test]
fn test_all_examples_compile() {
    let output = Command::new("cargo")
        .args(["build", "--examples", "-p", "windjammer-game-framework"])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Not all examples compiled successfully:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that the framework compiles with default features (2D + audio)
#[test]
fn test_default_features_compile() {
    let output = Command::new("cargo")
        .args(["build", "-p", "windjammer-game-framework"])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "Framework with default features failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

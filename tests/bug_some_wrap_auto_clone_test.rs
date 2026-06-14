#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD Test: Auto-clone when wrapping parameter in Some() then using it later
///
/// Bug: `self.last_camera = Some(camera)` consumes `camera`, but later uses
/// like `camera_data_to_gpu_state(camera)` get `.clone()` appended to a
/// dead variable instead of cloning at the `Some(...)` call site.
///
/// Root Cause: When `Some(camera)` has no signature in the registry, the
/// call argument generation's auto-clone check is completely skipped.
/// The auto-clone analysis correctly identifies the clone site, but the
/// codegen never applies it.
///
/// Fix: In the "no signature found" path for call arguments, still check
/// the auto-clone analysis and apply `.clone()` when needed.
use std::process::Command;

#[test]
fn test_some_wrap_then_use_parameter() {
    let source = r#"
struct CameraData {
    label: string,
    x: f32,
    y: f32,
}

struct Renderer {
    last_camera: Option<CameraData>,
}

impl Renderer {
    fn set_camera(self, camera: CameraData) {
        self.last_camera = Some(camera)
        self.apply_camera(camera)
    }

    fn apply_camera(self, camera: CameraData) {
        // uses camera
    }
}
"#;

    let wj = env!("CARGO_BIN_EXE_windjammer");
    let dir = std::env::temp_dir().join("wj_some_wrap_auto_clone_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/test.wj"), source).unwrap();

    let output = Command::new(wj)
        .arg("build")
        .arg("--path")
        .arg(dir.join("src/test.wj"))
        .arg("--output")
        .arg(dir.join("out"))
        .output()
        .expect("failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "wj build failed: {}\n{}",
        stderr,
        stdout
    );

    let generated = std::fs::read_to_string(dir.join("out/test.rs")).unwrap();

    // The `Some(camera)` call should clone camera, not the later use.
    // Correct: `self.last_camera = Some(camera.clone());` then `self.apply_camera(camera...)`
    // Wrong: `self.last_camera = Some(camera);` then `self.apply_camera(camera.clone())`
    assert!(
        generated.contains("Some(camera.clone())")
            || generated.contains("Some( camera.clone()"),
        "Expected Some(camera.clone()) but got:\n{}",
        generated
    );

    // The generated code should compile with rustc (no E0382)
    // Write a minimal Cargo project to verify
    std::fs::write(
        dir.join("out/main.rs"),
        "#[allow(dead_code, unused)]\nmod test;\nfn main() {}",
    )
    .unwrap();

    let rustc_output = Command::new("rustc")
        .arg("--edition=2021")
        .arg(dir.join("out/main.rs"))
        .arg("--out-dir")
        .arg(dir.join("out"))
        .output()
        .expect("failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    assert!(
        !rustc_stderr.contains("E0382"),
        "Generated code has use-after-move error (E0382):\n{}",
        rustc_stderr
    );

    let _ = std::fs::remove_dir_all(&dir);
}

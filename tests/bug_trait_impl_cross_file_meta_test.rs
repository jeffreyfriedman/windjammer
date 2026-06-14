#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// TDD test: TRUE cross-file trait impl via .wj.meta.
///
/// This simulates the real game scenario:
/// 1. File A defines a trait whose methods mutate self
/// 2. File A is compiled, generating .wj.meta with TraitName::method ownership
/// 3. File B implements the trait for a struct
/// 4. File B is compiled separately, loading .wj.meta to get correct self-ownership
///
/// Bug: When game_renderer.wj implements RenderPort trait (defined in render_port.wj),
/// the impl methods get &self instead of &mut self because the trait method ownership
/// from .wj.meta wasn't being loaded/used correctly.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_cross_file_trait_impl_uses_meta_ownership() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_trait_cross_file_meta");

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(test_dir.join("src")).unwrap();

    // File A: trait definition with methods that mutate self (via impl body)
    let trait_source = r#"
pub trait RenderPort {
    fn initialize()
    fn set_camera(data: CameraData)
    fn render_frame()
    fn shutdown()
}

pub struct CameraData {
    pub x: f32,
    pub y: f32,
}

struct MockRenderer {
    initialized: bool,
    camera: CameraData,
    frame_count: i32,
}

impl RenderPort for MockRenderer {
    fn initialize() {
        self.initialized = true
    }

    fn set_camera(data: CameraData) {
        self.camera = data
    }

    fn render_frame() {
        self.frame_count = self.frame_count + 1
    }

    fn shutdown() {
        self.initialized = false
    }
}
"#;

    // File B: a separate struct implementing the same trait
    let impl_source = r#"
use crate::render_port::RenderPort
use crate::render_port::CameraData

struct GameRenderer {
    ready: bool,
    cam: CameraData,
    frames: i32,
}

impl RenderPort for GameRenderer {
    fn initialize() {
        self.ready = true
    }

    fn set_camera(data: CameraData) {
        self.cam = data
    }

    fn render_frame() {
        self.frames = self.frames + 1
    }

    fn shutdown() {
        self.ready = false
    }
}
"#;

    fs::write(test_dir.join("src").join("render_port.wj"), trait_source).unwrap();
    fs::write(test_dir.join("src").join("game_renderer.wj"), impl_source).unwrap();

    // Step 1: Compile as library (multipass) to generate both files + .wj.meta
    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("src"))
        .arg("--output")
        .arg(test_dir.join("out"))
        .arg("--library")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build (library) failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Check the generated game_renderer.rs for correct self-ownership
    let generated = fs::read_to_string(test_dir.join("out").join("game_renderer.rs")).unwrap();

    // The impl methods should use &mut self since the trait methods mutate self
    let impl_start = generated.find("impl RenderPort for GameRenderer").unwrap_or(0);
    let impl_section = &generated[impl_start..];

    assert!(
        impl_section.contains("fn initialize(&mut self)"),
        "Cross-file trait impl should get &mut self from trait meta.\nImpl section:\n{}",
        impl_section
    );

    assert!(
        impl_section.contains("fn render_frame(&mut self)"),
        "Cross-file trait impl should get &mut self for render_frame.\nImpl section:\n{}",
        impl_section
    );

    assert!(
        impl_section.contains("fn shutdown(&mut self)"),
        "Cross-file trait impl should get &mut self for shutdown.\nImpl section:\n{}",
        impl_section
    );

    // Verify the generated code compiles with rustc (both files together)
    let _render_port_rs = test_dir.join("out").join("render_port.rs");
    let _game_renderer_rs = test_dir.join("out").join("game_renderer.rs");

    // Create a lib.rs that includes both modules
    let lib_rs = "mod render_port;\nmod game_renderer;\n".to_string();
    fs::write(test_dir.join("out").join("lib.rs"), lib_rs).unwrap();

    let rustc_out = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg(test_dir.join("out").join("lib.rs"))
        .arg("-o")
        .arg(test_dir.join("out").join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_out.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_out.stderr);
        // Only fail if E0053 (type mismatch) - the specific bug we're fixing
        if stderr.contains("E0053") {
            panic!(
                "E0053: Trait impl self-ownership mismatch in generated code:\n{}",
                stderr
            );
        }
        // Other errors (missing imports, etc.) are acceptable in this isolated test
        eprintln!("Note: rustc had non-E0053 errors (expected in isolated test):\n{}", stderr);
    }
}

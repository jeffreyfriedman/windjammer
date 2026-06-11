#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

/// TDD Integration Test: Full build with FFI imports
///
/// This test reproduces the actual build process to find why
/// `use crate::ffi` is missing from generated runtime.rs
use std::fs;
use tempfile::TempDir;

#[test]
fn test_full_build_includes_ffi_imports() {
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let src = project_root.join("src");
    let build_dir = project_root.join("build");

    fs::create_dir_all(&src).unwrap();

    // Create runtime.wj that uses crate::ffi (exactly like the real file)
    let runtime_wj = r#"
use crate::GameLoop
use crate::GameLoopConfig
use crate::ffi

pub struct GameRuntime {
    config: GameLoopConfig,
}

impl GameRuntime {
    pub fn new(config: GameLoopConfig) -> GameRuntime {
        GameRuntime { config }
    }
    
    pub fn run<G: GameLoop>(self, mut game: G) {
        ffi::run_with_event_loop(game, self.config.window_title, self.config.window_width, self.config.window_height)
    }
}
"#;
    fs::write(src.join("runtime.wj"), runtime_wj).unwrap();

    // Create game_loop.wj to provide GameLoop trait
    let game_loop_wj = r#"
pub trait GameLoop {
    fn update(self, delta: f32)
    fn render(self)
}

pub struct GameLoopConfig {
    pub window_title: string,
    pub window_width: int,
    pub window_height: int,
}
"#;
    fs::write(src.join("game_loop.wj"), game_loop_wj).unwrap();

    // Create ffi.rs in project root (hand-written)
    let ffi_rs = r#"
pub fn run_with_event_loop<G>(mut _game: G, _title: string, _width: i64, _height: i64)
where
    G: crate::GameLoop,
{
    println!("FFI: run_with_event_loop");
}
"#;
    fs::write(project_root.join("ffi.rs"), ffi_rs).unwrap();

    // Run the actual build process
    let result =
        windjammer::build_project(&src, &build_dir, windjammer::CompilationTarget::Rust, true);

    assert!(result.is_ok(), "Build failed: {:?}", result.err());

    // THE WINDJAMMER WAY: Check that runtime.rs includes all necessary imports
    let runtime_rs_path = build_dir.join("runtime.rs");
    assert!(runtime_rs_path.exists(), "runtime.rs should exist");

    let runtime_rs = fs::read_to_string(&runtime_rs_path).unwrap();
    println!("Generated runtime.rs:\n{}\n", runtime_rs);

    // Critical assertion: use crate::ffi MUST be present
    assert!(
        runtime_rs.contains("use crate::ffi;"),
        "runtime.rs MUST contain 'use crate::ffi;'\n\nGenerated code:\n{}",
        runtime_rs
    );

    // Also check for the other imports
    assert!(
        runtime_rs.contains("use crate::GameLoop;"),
        "runtime.rs should contain 'use crate::GameLoop;'"
    );

    assert!(
        runtime_rs.contains("use crate::GameLoopConfig;"),
        "runtime.rs should contain 'use crate::GameLoopConfig;'"
    );

    // Verify the ffi call is still there
    assert!(
        runtime_rs.contains("ffi::run_with_event_loop"),
        "runtime.rs should contain ffi::run_with_event_loop call"
    );
}

#[test]
fn test_regeneration_preserves_ffi_imports() {
    // THE WINDJAMMER WAY: Two-pass compilation shouldn't drop imports
    let temp_dir = TempDir::new().unwrap();
    let project_root = temp_dir.path();
    let src = project_root.join("src");
    let build_dir = project_root.join("build");

    fs::create_dir_all(&src).unwrap();

    let simple_wj = r#"
use crate::ffi

pub fn test() {
    ffi::some_function()
}
"#;
    fs::write(src.join("simple.wj"), simple_wj).unwrap();

    let ffi_rs = "pub fn some_function() {}";
    fs::write(project_root.join("ffi.rs"), ffi_rs).unwrap();

    // Build
    let result =
        windjammer::build_project(&src, &build_dir, windjammer::CompilationTarget::Rust, true);
    assert!(result.is_ok());

    // Check generated file
    let simple_rs = fs::read_to_string(build_dir.join("simple.rs")).unwrap();
    println!("Generated simple.rs:\n{}", simple_rs);

    assert!(
        simple_rs.contains("use crate::ffi;"),
        "Regeneration should preserve 'use crate::ffi;'\n\nGenerated:\n{}",
        simple_rs
    );
}

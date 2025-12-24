/// TDD Test: Files with same name as their parent directory should compile correctly
///
/// BUG: When compiling a full project, files like `src_wj/game_loop/game_loop.wj`
/// end up with 0 bytes in the output `game_loop/game_loop.rs`.
///
/// This happens because during full project compilation, these files are being
/// compiled with the wrong source root, leading to incorrect output paths or empty writes.
///
/// THE WINDJAMMER WAY: All files should compile correctly regardless of naming!
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_same_name_module_compiles_correctly() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();

    // Create a project structure with same-name modules
    let src_wj = project_root.join("src_wj");
    std::fs::create_dir_all(&src_wj).unwrap();

    // Create root mod.wj
    std::fs::write(
        src_wj.join("mod.wj"),
        r#"
    pub mod game_loop;
    pub mod input;
    "#,
    )
    .unwrap();

    // Create game_loop directory with game_loop.wj (same name!)
    let game_loop_dir = src_wj.join("game_loop");
    std::fs::create_dir_all(&game_loop_dir).unwrap();

    std::fs::write(
        game_loop_dir.join("mod.wj"),
        r#"
    pub mod game_loop;
    pub use game_loop::GameLoop;
    "#,
    )
    .unwrap();

    std::fs::write(
        game_loop_dir.join("game_loop.wj"),
        r#"
    pub trait GameLoop {
        fn update(self);
        fn render(self);
    }
    "#,
    )
    .unwrap();

    // Create input directory with input.wj (same name!)
    let input_dir = src_wj.join("input");
    std::fs::create_dir_all(&input_dir).unwrap();

    std::fs::write(
        input_dir.join("mod.wj"),
        r#"
    pub mod input;
    pub use input::Input;
    "#,
    )
    .unwrap();

    std::fs::write(
        input_dir.join("input.wj"),
        r#"
    pub struct Input {
        pub frame: i64,
    }
    
    impl Input {
        pub fn new() -> Input {
            Input { frame: 0 }
        }
    }
    "#,
    )
    .unwrap();

    // Compile the FULL PROJECT (not individual files)
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output_dir = project_root.join("output");

    let compile_result = Command::new(&wj_binary)
        .args([
            "build",
            src_wj.join("mod.wj").to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    assert!(
        compile_result.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_result.stdout),
        String::from_utf8_lossy(&compile_result.stderr)
    );

    // CRITICAL ASSERTION 1: game_loop/game_loop.rs should exist and be NON-EMPTY
    let game_loop_rs = output_dir.join("game_loop/game_loop.rs");
    assert!(
        game_loop_rs.exists(),
        "game_loop/game_loop.rs should exist at: {}",
        game_loop_rs.display()
    );

    let game_loop_content =
        std::fs::read_to_string(&game_loop_rs).expect("Failed to read game_loop/game_loop.rs");

    assert!(
        !game_loop_content.is_empty(),
        "game_loop/game_loop.rs should NOT be empty (0 bytes)! BUG: Same-name modules end up empty."
    );

    assert!(
        game_loop_content.contains("pub trait GameLoop"),
        "game_loop/game_loop.rs should contain GameLoop trait, got:\n{}",
        game_loop_content
    );

    // CRITICAL ASSERTION 2: input/input.rs should exist and be NON-EMPTY
    let input_rs = output_dir.join("input/input.rs");
    assert!(
        input_rs.exists(),
        "input/input.rs should exist at: {}",
        input_rs.display()
    );

    let input_content = std::fs::read_to_string(&input_rs).expect("Failed to read input/input.rs");

    assert!(
        !input_content.is_empty(),
        "input/input.rs should NOT be empty (0 bytes)! BUG: Same-name modules end up empty."
    );

    assert!(
        input_content.contains("pub struct Input"),
        "input/input.rs should contain Input struct, got:\n{}",
        input_content
    );

    // Verify it compiles with rustc
    let rustc_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(output_dir.join("lib.rs"))
        .arg("--out-dir")
        .arg(&output_dir)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to run rustc");

    assert!(
        rustc_result.status.success(),
        "Generated code should compile with rustc!\nrustc stderr:\n{}",
        String::from_utf8_lossy(&rustc_result.stderr)
    );
}

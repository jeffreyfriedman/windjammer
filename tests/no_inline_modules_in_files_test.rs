/// TDD Test: Individual .rs files should NOT have inline module definitions
///
/// THE WINDJAMMER WAY: When compiling a multi-file library project,
/// the compiler should generate separate .rs files in their correct directories
/// and NOT inline unrelated modules into every file.
///
/// BUG: audio/sound.rs was incorrectly containing:
/// ```
/// pub mod rigidbody2d;
/// pub mod collision2d;
/// ...
/// pub mod input {
///     // entire input module inlined here
/// }
/// ```
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_no_inline_modules_in_individual_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let src_wj = temp_dir.path().join("src_wj");

    // Create a multi-file project structure
    std::fs::create_dir_all(src_wj.join("audio")).unwrap();
    std::fs::create_dir_all(src_wj.join("physics")).unwrap();

    // Create audio module
    std::fs::write(
        src_wj.join("audio/sound.wj"),
        r#"
    pub struct Sound {
        pub volume: f32,
    }
    
    impl Sound {
        pub fn new(volume: f32) -> Sound {
            Sound { volume }
        }
    }
    "#,
    )
    .unwrap();

    std::fs::write(src_wj.join("audio/mod.wj"), "pub use sound::Sound;").unwrap();

    // Create physics module (completely unrelated)
    std::fs::write(
        src_wj.join("physics/rigidbody.wj"),
        r#"
    pub struct RigidBody {
        pub mass: f32,
    }
    "#,
    )
    .unwrap();

    std::fs::write(
        src_wj.join("physics/mod.wj"),
        "pub use rigidbody::RigidBody;",
    )
    .unwrap();

    // Create root mod.wj
    std::fs::write(
        src_wj.join("mod.wj"),
        r#"
    pub mod audio;
    pub mod physics;
    "#,
    )
    .unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output_dir = temp_dir.path().join("output");

    // Compile with --library --module-file
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

    // Read the generated audio/mod.rs (not sound.rs - compiler generates mod.rs)
    let audio_mod_rs = std::fs::read_to_string(output_dir.join("audio/mod.rs"))
        .expect("Failed to read audio/mod.rs");

    // CRITICAL ASSERTION: audio/mod.rs should NOT contain inlined physics modules
    assert!(
        !audio_mod_rs.contains("pub mod rigidbody {"),
        "audio/mod.rs should NOT inline physics modules!\nFile contents:\n{}",
        audio_mod_rs
    );

    assert!(
        !audio_mod_rs.contains("pub struct RigidBody"),
        "audio/mod.rs should NOT contain RigidBody struct (that's in physics!)!\nFile contents:\n{}",
        audio_mod_rs
    );

    // audio/mod.rs should contain the sound module DECLARATION (not inline!)
    assert!(
        audio_mod_rs.contains("pub mod sound;"),
        "audio/mod.rs should contain the sound module declaration, got:\n{}",
        audio_mod_rs
    );

    // It should NOT contain inline struct definitions
    assert!(
        !audio_mod_rs.contains("pub struct Sound"),
        "audio/mod.rs should NOT contain inline Sound struct (should be in sound.rs)!\nFile contents:\n{}",
        audio_mod_rs
    );

    // Similarly, physics/mod.rs should NOT contain inlined audio modules
    let physics_mod_rs = std::fs::read_to_string(output_dir.join("physics/mod.rs"))
        .expect("Failed to read physics/mod.rs");

    assert!(
        !physics_mod_rs.contains("pub mod sound {"),
        "physics/mod.rs should NOT inline audio modules!\nFile contents:\n{}",
        physics_mod_rs
    );

    assert!(
        !physics_mod_rs.contains("pub struct Sound"),
        "physics/mod.rs should NOT contain Sound struct (that's in audio!)!\nFile contents:\n{}",
        physics_mod_rs
    );

    // physics/mod.rs should contain the rigidbody module DECLARATION (not inline!)
    assert!(
        physics_mod_rs.contains("pub mod rigidbody;"),
        "physics/mod.rs should contain the rigidbody module declaration, got:\n{}",
        physics_mod_rs
    );

    // It should NOT contain inline struct definitions
    assert!(
        !physics_mod_rs.contains("pub struct RigidBody"),
        "physics/mod.rs should NOT contain inline RigidBody struct (should be in rigidbody.rs)!\nFile contents:\n{}",
        physics_mod_rs
    );
}

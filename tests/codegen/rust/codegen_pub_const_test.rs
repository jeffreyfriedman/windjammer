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

/// Test: pub const declarations in Windjammer must emit `pub const` in generated Rust.
///
/// Bug: The Windjammer compiler parsed `pub const` but discarded the `pub` modifier,
/// generating `const` instead. This caused E0603 "private constant" errors when
/// other modules tried to use the constants.
///
/// Example Windjammer:
///   pub const ACTION_ATTACK: i32 = 0
///
/// Expected Rust:
///   pub const ACTION_ATTACK: i32 = 0;
///
/// Actual (broken):
///   const ACTION_ATTACK: i32 = 0;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pub_const_emits_pub_in_generated_rust() {
    let temp_dir = TempDir::new().unwrap();

    let source = r#"
pub const ACTION_ATTACK: i32 = 0
pub const ACTION_DEFEND: i32 = 1
const PRIVATE_CONST: i32 = 99
"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("tactical.wj"), source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to run wj");

    assert!(
        wj_output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&wj_output.stdout),
        String::from_utf8_lossy(&wj_output.stderr)
    );

    let generated_path = temp_dir.path().join("build").join("tactical.rs");
    let generated = fs::read_to_string(&generated_path).unwrap_or_else(|_| {
        panic!(
            "generated tactical.rs not found at {:?}.\nBuild output:\n{}",
            generated_path,
            String::from_utf8_lossy(&wj_output.stdout)
        )
    });

    assert!(
        generated.contains("pub const ACTION_ATTACK: i32 ="),
        "Expected 'pub const ACTION_ATTACK' but got:\n{}",
        generated
    );
    assert!(
        generated.contains("pub const ACTION_DEFEND: i32 ="),
        "Expected 'pub const ACTION_DEFEND' but got:\n{}",
        generated
    );

    assert!(
        !generated.contains("pub const PRIVATE_CONST"),
        "PRIVATE_CONST should NOT have pub but got:\n{}",
        generated
    );
    let has_private = generated
        .lines()
        .any(|l| l.contains("const PRIVATE_CONST") && !l.contains("pub const"));
    assert!(
        has_private,
        "Expected 'const PRIVATE_CONST' (without pub) but got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pub_const_accessible_from_other_module() {
    let temp_dir = TempDir::new().unwrap();

    let constants_source = r#"
pub const MAX_HEALTH: i32 = 100
pub const MIN_HEALTH: i32 = 0
"#;

    let game_source = r#"
use crate::constants

pub fn check_health(health: i32) -> bool {
    health >= constants::MIN_HEALTH
}
"#;

    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("constants.wj"), constants_source).unwrap();
    fs::write(src_dir.join("game.wj"), game_source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("src/")
        .current_dir(temp_dir.path())
        .output()
        .expect("failed to run wj");

    assert!(
        wj_output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&wj_output.stdout),
        String::from_utf8_lossy(&wj_output.stderr)
    );

    let constants_rs = temp_dir.path().join("build").join("constants.rs");
    let generated = fs::read_to_string(&constants_rs).unwrap_or_else(|_| {
        panic!(
            "generated constants.rs not found.\nBuild output:\n{}",
            String::from_utf8_lossy(&wj_output.stdout)
        )
    });

    assert!(
        generated.contains("pub const MAX_HEALTH: i32 ="),
        "Expected 'pub const MAX_HEALTH' in constants.rs:\n{}",
        generated
    );
    assert!(
        generated.contains("pub const MIN_HEALTH: i32 ="),
        "Expected 'pub const MIN_HEALTH' in constants.rs:\n{}",
        generated
    );
}

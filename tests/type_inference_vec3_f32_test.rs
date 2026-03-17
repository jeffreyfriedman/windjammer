/// TDD: Vec3 (f32) vs f64 literal consistency
///
/// BUG: Windjammer generates Vec3::new(1.0_f64, 2.0_f64, 3.0_f64) but Vec3 has f32 fields,
/// causing E0308 type mismatches (expected f32, found f64).
///
/// ROOT CAUSE: Type inference not propagating Vec3::new param types (f32) to literals.
///
/// SUCCESS: Vec3::new(1.0, 2.0, 3.0) should generate 1.0_f32, 2.0_f32, 3.0_f32.

use std::fs;
use std::process::Command;
use windjammer::{build_project, CompilationTarget};

fn run_wj_build(wj_source: &str, output_dir: &str) -> (bool, String) {
    fs::create_dir_all(output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let result = build_project(
        &std::path::Path::new(&format!("{}/test.wj", output_dir)),
        std::path::Path::new(output_dir),
        CompilationTarget::Rust,
        false,
    );

    match result {
        Ok(()) => (true, String::new()),
        Err(e) => (false, e.to_string()),
    }
}

#[test]
fn test_vec3_with_f32_literals() {
    // Vec3::new(x: f32, y: f32, z: f32) - literals should infer f32 from signature
    let wj_source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

pub fn make_vec() -> Vec3 {
    Vec3::new(1.0, 2.0, 3.0)
}
"#;

    let output_dir = "/tmp/wj_test_vec3_f32";
    let (success, err_msg) = run_wj_build(wj_source, output_dir);

    assert!(
        success,
        "Compilation should succeed: {}",
        err_msg
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    // Vec3::new params are f32 - literals MUST be f32
    assert!(
        rust_code.contains("1.0_f32") && rust_code.contains("2.0_f32") && rust_code.contains("3.0_f32"),
        "Vec3::new(1.0, 2.0, 3.0) should generate f32 literals, got:\n{}",
        rust_code
    );

    // Should NOT have f64 for those literals (any f64 causes E0308)
    assert!(
        !rust_code.contains("1.0_f64") && !rust_code.contains("2.0_f64") && !rust_code.contains("3.0_f64"),
        "Vec3::new literals should not be f64 (causes E0308), got:\n{}",
        rust_code
    );

    // Verify Rust compiles with rustc (no E0308, no Cargo.toml needed)
    let rust_build = Command::new("rustc")
        .args(&[
            &format!("{}/test.rs", output_dir),
            "--crate-type=lib",
            "-o",
            &format!("{}/test.lib", output_dir),
        ])
        .output()
        .expect("Failed to run rustc");

    assert!(
        rust_build.status.success(),
        "Generated Rust should compile (no f32/f64 mismatch), stderr: {}",
        String::from_utf8_lossy(&rust_build.stderr)
    );
}

#[test]
fn test_vec3_math_f32() {
    // Vec3 + Vec3 and Vec3 * f32 should stay f32
    let wj_source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

pub fn add_vecs() -> Vec3 {
    let a = Vec3::new(1.0, 0.0, 0.0)
    let b = Vec3::new(0.0, 1.0, 0.0)
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
"#;

    let output_dir = "/tmp/wj_test_vec3_math";
    let (success, err_msg) = run_wj_build(wj_source, output_dir);

    assert!(
        success,
        "Compilation should succeed: {}",
        err_msg
    );

    let rust_code = fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found");

    // All Vec3 literals should be f32
    let f64_in_vec3 = rust_code.contains("1.0_f64") || rust_code.contains("0.0_f64");
    assert!(
        !f64_in_vec3,
        "Vec3 math literals should be f32 (not f64), got:\n{}",
        rust_code
    );

    // Verify Rust compiles with rustc
    let rust_build = Command::new("rustc")
        .args(&[
            &format!("{}/test.rs", output_dir),
            "--crate-type=lib",
            "-o",
            &format!("{}/test.lib", output_dir),
        ])
        .output()
        .expect("Failed to run rustc");

    assert!(
        rust_build.status.success(),
        "Generated Rust should compile, stderr: {}",
        String::from_utf8_lossy(&rust_build.stderr)
    );
}

#[test]
#[ignore = "Requires wj binary (cargo run --bin wj). Run manually: cargo test test_vec3_cross_module_inference -- --ignored"]
fn test_vec3_cross_module_inference() {
    // TDD: Verify Vec3 type inference fix - cross-module metadata loading
    // Vec3 defined in math/vec3.wj, used in game.wj via use crate::math::vec3::Vec3
    // source_root must be correct so math/vec3.wj.meta is found
    let output_dir = std::env::temp_dir().join("wj_vec3_cross_module_test");
    let output_dir = output_dir.to_str().unwrap();
    fs::create_dir_all(output_dir).unwrap();
    fs::create_dir_all(format!("{}/math", output_dir)).unwrap();

    // math/vec3.wj - Vec3 with f32 fields
    let vec3_source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}
"#;
    fs::write(format!("{}/math/vec3.wj", output_dir), vec3_source).unwrap();

    // math/mod.wj - re-export vec3
    fs::write(format!("{}/math/mod.wj", output_dir), "pub mod vec3").unwrap();

    // game.wj - uses Vec3::new from another module (requires metadata load)
    // use statement triggers load_imported_metadata → math/vec3.wj.meta
    let game_source = r#"
mod math;
use crate::math::vec3::Vec3;

pub fn make_vec() -> Vec3 {
    Vec3::new(1.0, 2.0, 3.0)
}
"#;
    fs::write(format!("{}/game.wj", output_dir), game_source).unwrap();

    // Build via wj binary (full multi-file flow with metadata)
    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args(["build", "--target",
            "rust",
            "--no-cargo",
            &format!("{}/game.wj", output_dir),
            "--output",
            output_dir,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Compilation should succeed.\nstderr: {}\nstdout: {}",
        stderr,
        stdout
    );

    let rust_code = fs::read_to_string(format!("{}/game.rs", output_dir))
        .expect("Generated Rust file not found");

    // Literals must be f32 (from Vec3::new signature in metadata)
    assert!(
        rust_code.contains("1.0_f32") && rust_code.contains("2.0_f32") && rust_code.contains("3.0_f32"),
        "Cross-module Vec3::new should generate f32 literals, got:\n{}",
        rust_code
    );

    // Should NOT have f64 (would cause E0308)
    assert!(
        !rust_code.contains("1.0_f64") && !rust_code.contains("2.0_f64") && !rust_code.contains("3.0_f64"),
        "Vec3::new literals should not be f64, got:\n{}",
        rust_code
    );
}

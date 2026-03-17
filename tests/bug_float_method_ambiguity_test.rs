/// TDD Test: Float Method Call Type Ambiguity
///
/// **Windjammer Philosophy: Compiler Does the Hard Work**
///
/// Problem: When calling methods like `.max()` or `.min()` on float literals
/// or untyped floats, Rust can't infer the type (f32 vs f64).
///
/// Example (BROKEN):
/// ```windjammer
/// let tmin = 0.0
/// let tmax = 1.0
/// tmin = tmin.max(tx1.min(tx2))  // ERROR: ambiguous numeric type
/// ```
///
/// The compiler should infer f32 as the default float type (like Rust),
/// or add explicit type annotations when needed.
///
/// Root Cause: Windjammer doesn't track float type context during codegen.

use std::fs;
use std::process::Command;

#[test]
fn test_float_literal_method_calls_infer_f32() {
    let source = r#"
fn test_max_min() -> f64 {
    let tmin = 0.0
    let tmax = 1.0
    let tx1 = -0.5
    let tx2 = 0.5
    
    // These should compile - compiler should infer f64 from return type
    let result = tmin.max(tx1.min(tx2))
    result
}

fn main() {
    let result = test_max_min()
    println!("Result: {}", result)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_float_method_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rs_file)
        .expect("Failed to read generated Rust file");

    // Context-sensitive inference: Function returns f32, so literals should be f32
    // The compiler should generate: let tmin = 0.0_f32
    assert!(
        rust_code.contains("0.0_f64") || rust_code.contains("0.0_f32"),
        "Generated Rust should have explicit float suffix to avoid ambiguity, got:\n{}",
        rust_code
    );

    // Verify it compiles with rustc
    let rustc_output = Command::new("rustc")
        .arg(&rs_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Rustc compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_collision_aabb_ray_intersection() {
    // Real-world case from collision.wj
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn ray_aabb_intersection(ray_origin: Vec3, ray_dir: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> f32 {
    let mut tmin = 0.0
    let mut tmax = 1000.0
    
    // X axis
    let invdx = 1.0 / ray_dir.x
    let tx1 = (aabb_min.x - ray_origin.x) * invdx
    let tx2 = (aabb_max.x - ray_origin.x) * invdx
    tmin = tmin.max(tx1.min(tx2))
    tmax = tmax.min(tx1.max(tx2))
    
    // Y axis
    let invdy = 1.0 / ray_dir.y
    let ty1 = (aabb_min.y - ray_origin.y) * invdy
    let ty2 = (aabb_max.y - ray_origin.y) * invdy
    tmin = tmin.max(ty1.min(ty2))
    tmax = tmax.min(ty1.max(ty2))
    
    // Z axis
    let invdz = 1.0 / ray_dir.z
    let tz1 = (aabb_min.z - ray_origin.z) * invdz
    let tz2 = (aabb_max.z - ray_origin.z) * invdz
    tmin = tmin.max(tz1.min(tz2))
    tmax = tmax.min(tz1.max(tz2))
    
    if tmax >= tmin && tmax >= 0.0 {
        tmin
    } else {
        -1.0
    }
}

fn main() {
    let origin = Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    let dir = Vec3 { x: 1.0, y: 0.0, z: 0.0 }
    let aabb_min = Vec3 { x: 2.0, y: -1.0, z: -1.0 }
    let aabb_max = Vec3 { x: 4.0, y: 1.0, z: 1.0 }
    
    let hit_distance = ray_aabb_intersection(origin, dir, aabb_min, aabb_max)
    println!("Hit distance: {}", hit_distance)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_collision_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rs_file = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rs_file)
        .expect("Failed to read generated Rust file");

    // Verify it compiles with rustc (should not have ambiguous float errors)
    let rustc_output = Command::new("rustc")
        .arg(&rs_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        
        // Check if it's the specific ambiguous float error
        if stderr.contains("can't call method `max` on ambiguous numeric type") 
            || stderr.contains("can't call method `min` on ambiguous numeric type") {
            panic!(
                "AMBIGUOUS FLOAT TYPE ERROR (E0689):\n{}\n\nGenerated code:\n{}",
                stderr, rust_code
            );
        }
        
        panic!(
            "Rustc compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, rust_code
        );
    }

    let _ = fs::remove_dir_all(&test_dir);
}

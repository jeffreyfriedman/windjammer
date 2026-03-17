use std::process::Command;
use std::io::Write;

#[test]
fn test_binary_op_float_type_propagation() {
    // TDD: Reproduce binary operation type propagation bug
    // Given: f32 * f32_literal * float_literal
    // Expected: All literals should be f32
    // Bug: Last literal becomes f64
    
    let source = r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

pub fn compute() -> Vec3 {
    let radius: f32 = 10.0
    let i: i32 = 1
    Vec3::new(i as f32 * radius * 0.5, 0.0, 0.0)
}
"#;
    
    let temp_dir = std::env::temp_dir().join("wj_test_binary_op_prop");
    std::fs::create_dir_all(&temp_dir).unwrap();
    
    let wj_file = temp_dir.join("test.wj");
    let mut file = std::fs::File::create(&wj_file).unwrap();
    file.write_all(source.as_bytes()).unwrap();
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj")).args(["build", wj_file.to_str().unwrap(), "--output", temp_dir.to_str().unwrap(), "--no-cargo"])
        .current_dir(std::env::current_dir().unwrap())
        .output()
        .expect("Failed to run wj build");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    eprintln!("=== WJ BUILD STDOUT ===\n{}", stdout);
    eprintln!("=== WJ BUILD STDERR ===\n{}", stderr);
    
    assert!(output.status.success(), "wj build failed");
    
    let rs_file = temp_dir.join("test.rs");
    let generated = std::fs::read_to_string(&rs_file).unwrap();
    
    eprintln!("=== GENERATED RUST ===\n{}", generated);
    
    // Check what we actually generated
    let has_f64 = generated.contains("_f64");
    let has_correct_f32 = generated.contains("i as f32 * radius * 0.5_f32");
    
    eprintln!("=== INFERENCE CHECK ===");
    eprintln!("Has _f64: {}", has_f64);
    eprintln!("Has correct f32 chain: {}", has_correct_f32);
    
    if has_f64 {
        eprintln!("❌ BUG: Float literals still being inferred as f64!");
        // Find the specific line
        for (i, line) in generated.lines().enumerate() {
            if line.contains("_f64") {
                eprintln!("  Line {}: {}", i + 1, line.trim());
            }
        }
    }
    
    // All literals in the binary operation chain should be f32
    assert!(
        generated.contains("i as f32 * radius * 0.5_f32"),
        "Binary operation should infer 0.5 as f32, not f64"
    );
    
    // Arguments to Vec3::new should all be consistent type
    assert!(
        !generated.contains("0.5_f64") && !generated.contains("0.0_f64"),
        "Should not have any f64 literals when Vec3::new takes f32 params"
    );
    
    std::fs::remove_dir_all(&temp_dir).ok();
}

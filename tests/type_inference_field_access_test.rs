use std::process::Command;
use std::io::Write;

#[test]
fn test_field_access_constrains_binary_op() {
    // TDD: Reproduce field access + binary op type inference bug
    // Given: self.field (f32) * float_literal
    // Expected: Literal should be inferred as f32
    // Bug: Literal becomes f64
    
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

pub fn compute(radius: f32) -> Vec3 {
    let i: i32 = 1
    Vec3::new(i as f32 * radius * 0.5, 0.0, 0.0)
}
"#;
    
    let temp_dir = std::env::temp_dir().join("wj_test_field_access");
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
    
    // Check all float literals
    let f64_count = generated.matches("_f64").count();
    let f32_count = generated.matches("_f32").count();
    
    eprintln!("=== FLOAT LITERAL COUNT ===");
    eprintln!("f64 literals: {}", f64_count);
    eprintln!("f32 literals: {}", f32_count);
    
    if f64_count > 0 {
        eprintln!("❌ Found f64 literals:");
        for (i, line) in generated.lines().enumerate() {
            if line.contains("_f64") {
                eprintln!("  Line {}: {}", i + 1, line.trim());
            }
        }
        panic!("Should not have any f64 literals when working with f32 types");
    }
    
    // Field access (self.cell_size) is f32, so binary op operands should be f32
    assert!(
        generated.contains("radius * 0.5_f32") || generated.contains("0.5_f32"),
        "Expected 0.5 to be inferred as f32"
    );
    
    std::fs::remove_dir_all(&temp_dir).ok();
}

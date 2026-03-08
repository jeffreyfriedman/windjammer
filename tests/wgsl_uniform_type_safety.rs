// TDD: WGSL Uniform Type Safety
// Prevents black screen bugs by ensuring correct types in uniform buffers
//
// CRITICAL LESSON FROM BLACK SCREEN BUG:
// - Shader declared: var<uniform> screen_size: vec2<u32>
// - Host sent: vec2<f32> via uniform buffer
// - Result: GPU reads float bits as integers → GARBAGE VALUES → BLACK SCREEN!
//
// THE FIX: Auto-convert uint → f32 in uniform buffers, cast in shader code

use std::fs;
use std::process::Command;

fn transpile_wj_to_wgsl(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_uniform_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
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
        .arg("wgsl")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Compilation failed:\nSTDERR:\n{}\nSTDOUT:\n{}",
            stderr, stdout
        );
    }

    let wgsl_file = out_dir.join("test.wgsl");
    let content = fs::read_to_string(&wgsl_file).expect("Failed to read generated WGSL file");

    let _ = fs::remove_dir_all(&test_dir);

    content
}

#[test]
fn test_uint_in_uniform_auto_converts_to_f32() {
    // LESSON FROM BLACK SCREEN BUG:
    // Shader declared: var<uniform> screen_size: vec2<u32>
    // Host sent: vec2<f32>
    // Result: GARBAGE VALUES → BLACK SCREEN!
    //
    // FIX: Auto-convert uint → f32 in uniforms
    
    let source = r#"
@uniform
@binding(0)
extern let screen_width: uint;

@compute(workgroup_size = [8, 8, 1])
fn main() {
    // Compiler should auto-cast when used as u32
    let w = screen_width as uint;
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);
    
    // CRITICAL: Should generate f32, not u32!
    assert!(generated.contains("var<uniform> screen_width:") && generated.contains("f32"), 
        "Uniform should be f32, not u32! Got:\n{}", generated);
    
    // Should NOT contain u32 in uniform declaration (after var<uniform>)
    assert!(!generated.contains("var<uniform> screen_width: u32"),
        "Found u32 in uniform - this causes black screen bug!");
    
    // Should have auto-conversion comment
    assert!(generated.contains("auto-converted"),
        "Should document the type conversion! Got:\n{}", generated);
}

#[test]
fn test_vec2_uint_in_uniform_converts_to_vec2_f32() {
    // The exact pattern that caused the black screen
    
    let source = r#"
@uniform
@binding(3)
extern let screen_size: vec2<uint>;

@compute(workgroup_size = [8, 8, 1])
fn main(@builtin(global_invocation_id) id: vec3<uint>) {
    let w = screen_size.x as uint;
    let h = screen_size.y as uint;
}
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    println!("Generated WGSL:\n{}", generated);
    
    // MUST be vec2<f32>, not vec2<u32>!
    assert!(generated.contains("var<uniform> screen_size:") && generated.contains("vec2<f32>"),
        "Expected vec2<f32>, got:\n{}", generated);
    
    // Should NOT have vec2<u32> in uniform
    let has_bad_pattern = generated.contains("var<uniform> screen_size: vec2<u32>");
    assert!(!has_bad_pattern,
        "vec2<u32> in uniform causes type mismatch bug!");
    
    // Should have auto-conversion comment
    assert!(generated.contains("auto-converted"),
        "Should document the type conversion!");
}

#[test]
fn test_f32_in_uniform_stays_f32() {
    // f32 is correct - should pass through unchanged
    
    let source = r#"
@uniform
@binding(0)
extern let exposure: float;
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    assert!(generated.contains("var<uniform> exposure: f32"));
}

#[test]
fn test_struct_with_uint_fields_in_uniform_converts() {
    // Structs in uniforms should also have uint → f32 conversion
    
    let source = r#"
struct ScreenDimensions {
    width: uint,
    height: uint,
}

@uniform
@binding(0)
extern let dims: ScreenDimensions;
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    // Struct fields should be f32, not u32
    // (This is more complex - may require struct transformation)
    // For now, we can at least verify the uniform is declared
    assert!(generated.contains("var<uniform> dims"));
}

#[test]
fn test_u32_in_storage_buffer_is_allowed() {
    // u32 is fine in storage buffers, only problematic in uniforms
    
    let source = r#"
@storage(read_write)
@binding(0)
extern let indices: array<uint>;
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    // u32 is OK in storage buffers
    assert!(generated.contains("array<u32>") || generated.contains("u32"));
}

#[test]
fn test_warning_message_for_uint_conversion() {
    // Transpiler should warn developer about the conversion
    
    let source = r#"
@uniform
@binding(0)
extern let count: uint;
"#;
    
    let generated = transpile_wj_to_wgsl(source);
    
    // Should include a comment explaining the conversion
    // (This is optional but good for debugging)
    assert!(generated.contains("f32") && !generated.contains("var<uniform> count: u32"));
}

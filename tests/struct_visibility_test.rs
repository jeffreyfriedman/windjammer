/// Test that pub structs are generated with pub visibility in Rust
use std::process::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_pub_struct_generates_pub_in_rust() {
    let temp_dir = TempDir::new().unwrap();
    let src_file = temp_dir.path().join("test.wj");
    
    // Write Windjammer source with pub struct
    fs::write(&src_file, r#"
pub struct GpuCameraState {
    pub view_matrix: Mat4
    pub proj_matrix: Mat4
    pub position: Vec3
}

pub struct LightingConfig {
    pub sun_direction: Vec3
    pub sun_intensity: f32
    pub ambient_color: Vec3
}

struct PrivateHelper {
    pub data: i32
}
"#).unwrap();

    // Compile to Rust
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_file)
        .arg("--no-cargo")
        .arg("-o")
        .arg(temp_dir.path())
        .output()
        .unwrap();
    
    assert!(output.status.success(), "Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Read generated Rust
    let rust_file = temp_dir.path().join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    
    // Verify pub structs are marked pub
    assert!(rust_code.contains("pub struct GpuCameraState"), 
        "GpuCameraState should be pub in generated Rust");
    assert!(rust_code.contains("pub struct LightingConfig"), 
        "LightingConfig should be pub in generated Rust");
    
    // Verify private structs are not marked pub
    assert!(rust_code.contains("struct PrivateHelper") && !rust_code.contains("pub struct PrivateHelper"),
        "PrivateHelper should NOT be pub in generated Rust");
}

#[test]
fn test_pub_enum_generates_pub_in_rust() {
    let temp_dir = TempDir::new().unwrap();
    let src_file = temp_dir.path().join("test_enum.wj");
    
    fs::write(&src_file, r#"
pub enum ShaderFile {
    VoxelRaymarch
    HiZDownsample
    HiZCull
}

enum PrivateEnum {
    Variant1
    Variant2
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_file)
        .arg("--no-cargo")
        .arg("-o")
        .arg(temp_dir.path())
        .output()
        .unwrap();
    
    assert!(output.status.success());
    
    let rust_code = fs::read_to_string(temp_dir.path().join("test_enum.rs")).unwrap();
    
    assert!(rust_code.contains("pub enum ShaderFile"), "ShaderFile should be pub");
    assert!(rust_code.contains("enum PrivateEnum") && !rust_code.contains("pub enum PrivateEnum"),
        "PrivateEnum should NOT be pub");
}

#[test]
fn test_pub_fn_generates_pub_in_rust() {
    let temp_dir = TempDir::new().unwrap();
    let src_file = temp_dir.path().join("test_fn.wj");
    
    fs::write(&src_file, r#"
pub fn public_function(x: i32) -> i32 {
    x + 1
}

fn private_function(x: i32) -> i32 {
    x * 2
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&src_file)
        .arg("--no-cargo")
        .arg("-o")
        .arg(temp_dir.path())
        .output()
        .unwrap();
    
    assert!(output.status.success());
    
    let rust_code = fs::read_to_string(temp_dir.path().join("test_fn.rs")).unwrap();
    
    assert!(rust_code.contains("pub fn public_function"), "public_function should be pub");
    assert!(rust_code.contains("fn private_function") && !rust_code.contains("pub fn private_function"),
        "private_function should NOT be pub");
}

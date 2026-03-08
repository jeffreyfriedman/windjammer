// TDD: Test ownership inference for method parameters that mutate their arguments

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_source(name: &str, source: &str) -> Result<String, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_dir = manifest_dir.join("test_output").join(name);
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    
    let wj_file = temp_dir.join(format!("{}.wj", name));
    fs::write(&wj_file, source).map_err(|e| e.to_string())?;
    
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            temp_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;
    
    if !output.status.success() {
        return Err(format!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    let rs_file = temp_dir.join(format!("{}.rs", name));
    fs::read_to_string(rs_file).map_err(|e| format!("Failed to read generated code: {}", e))
}

#[test]
fn test_method_param_inferred_as_mut_ref_when_field_mutated() {
    let source = r#"
struct Grid {
    data: i32,
}

impl Grid {
    pub fn set(&mut self, value: i32) {
        self.data = value
    }
}

fn modify_grid(self, grid: Grid) {
    grid.set(42)  // Should infer `&mut grid` parameter
}
    "#;

    let rust_code = compile_source("mut_param_test", source)
        .expect("Compilation should succeed");
    
    // Should generate `&mut grid` parameter
    assert!(rust_code.contains("grid: &mut Grid"), 
        "Should infer &mut for parameter when method mutates it. Got:\n{}", rust_code);
}


#[test]
fn test_string_param_comparison_no_deref() {
    let source = r#"
pub fn check_topic(topic: string) -> bool {
    if topic == "test" {
        return true
    }
    false
}
    "#;

    let rust_code = compile_source("string_cmp_test", source)
        .expect("Compilation should succeed");
    
    // Should NOT generate *topic (topic is already &str)
    assert!(!rust_code.contains("*topic =="), 
        "Should not dereference &str parameter. Got:\n{}", rust_code);
    assert!(rust_code.contains("topic ==") || rust_code.contains("topic.as_str()"),
        "Should generate valid string comparison. Got:\n{}", rust_code);
}

#[test]
fn test_string_param_comparison_in_method_no_deref() {
    let source = r#"
struct Companion {
    name: string,
}

impl Companion {
    pub fn get_dialogue_response(&self, topic: string) -> string {
        if topic == "pragmatic" {
            return "Test response".to_string()
        }
        "Default response".to_string()
    }
}
    "#;

    let rust_code = compile_source("string_method_cmp_test", source)
        .expect("Compilation should succeed");
    
    // Should NOT generate *topic (topic is already &str, even in method context)
    assert!(!rust_code.contains("*topic =="), 
        "Should not dereference &str parameter in method. Got:\n{}", rust_code);
    assert!(rust_code.contains("topic ==") || rust_code.contains("topic.as_str()"),
        "Should generate valid string comparison in method. Got:\n{}", rust_code);
}

#[test]
fn test_param_inferred_as_mut_ref_when_method_called() {
    let source = r#"
struct VoxelGrid {
    data: Vec<i32>,
}

impl VoxelGrid {
    pub fn set(&mut self, x: i32, y: i32, z: i32, value: i32) {
        // Mutates grid
    }
}

struct Environment {
    name: string,
}

impl Environment {
    fn generate_ground(self, grid: VoxelGrid) {
        grid.set(0, 0, 0, 1)  // Should infer `&mut grid` parameter
    }
    
    fn generate_buildings(self, grid: VoxelGrid) {
        let mut i = 0
        while i < 5 {
            grid.set(i, 0, 0, 2)  // Should infer `&mut grid` parameter
            i = i + 1
        }
    }
}
    "#;

    let rust_code = compile_source("voxel_mut_test", source)
        .expect("Compilation should succeed");
    
    // Should infer &mut for grid parameters (used in mutating method calls)
    assert!(rust_code.contains("grid: &mut VoxelGrid"), 
        "Should infer &mut VoxelGrid for parameter when method mutates it. Got:\n{}", rust_code);
}

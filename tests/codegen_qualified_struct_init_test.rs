// TDD Test: Verify qualified struct initialization (Module::Struct { ... }) works
// Bug: E0423 + E0425: `ffi::GpuVertex { position: [...] }` becomes 
//      `ffi::GpuVertex; HashMap::from([(position, [...])])`
// Root Cause: Parser doesn't recognize qualified names as struct literals
// Fix: Update parser lookahead to handle Module::Type { ... } syntax

use std::process::Command;

#[test]
fn test_qualified_struct_init_simple() {
    let code = r#"
        mod ffi {
            pub struct GpuVertex {
                pub position: [f32; 3],
                pub color: [f32; 4],
            }
        }
        
        fn main() {
            let vertex = ffi::GpuVertex {
                position: [1.0, 2.0, 3.0],
                color: [1.0, 0.0, 0.0, 1.0],
            };
        }
    "#;
    
    // Create temporary test directory
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_qualified_struct_{}_{}", 
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        std::process::id()
    ));
    std::fs::create_dir_all(&test_dir).unwrap();
    
    // Write test file
    std::fs::write(test_dir.join("main.wj"), code).unwrap();
    
    // Compile
    let wj_binary = std::env::var("CARGO_BIN_EXE_wj")
        .unwrap_or_else(|_| {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            format!("{}/target/release/wj", manifest_dir)
        });
    
    let output = Command::new(&wj_binary)
        .arg("build")
        .arg("main.wj")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Check generated code
    let generated_code = std::fs::read_to_string(test_dir.join("build/main.rs"))
        .expect("Failed to read generated code");
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);
    
    if !output.status.success() {
        panic!("Compilation failed!\nstdout: {}\nstderr: {}\ngenerated:\n{}", stdout, stderr, generated_code);
    }
    
    // Verify the struct initialization is correct
    assert!(generated_code.contains("ffi::GpuVertex {") || generated_code.contains("ffi::GpuVertex{"), 
        "Generated code should contain 'ffi::GpuVertex {{' for qualified struct initialization\nGenerated:\n{}", generated_code);
    assert!(generated_code.contains("position: ["), 
        "Generated code should contain 'position: [' for struct field\nGenerated:\n{}", generated_code);
    assert!(!generated_code.contains("HashMap::from"), 
        "Generated code should NOT convert struct init to HashMap\nGenerated:\n{}", generated_code);
    assert!(!generated_code.contains("ffi::GpuVertex;"), 
        "Generated code should not have incomplete struct initialization\nGenerated:\n{}", generated_code);
}

#[test]
fn test_qualified_struct_init_in_loop() {
    let code = r#"
        mod types {
            pub struct Vertex {
                pub x: f32,
                pub y: f32,
            }
        }
        
        fn convert(count: i32) -> Vec<types::Vertex> {
            let mut result = Vec::new();
            
            for i in 0..count {
                let v = types::Vertex {
                    x: i as f32,
                    y: i as f32 * 2.0,
                };
                result.push(v);
            }
            
            result
        }
        
        fn main() {
            let vertices = convert(10);
        }
    "#;
    
    // Create temporary test directory
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_qualified_loop_{}_{}", 
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        std::process::id()
    ));
    std::fs::create_dir_all(&test_dir).unwrap();
    
    // Write test file
    std::fs::write(test_dir.join("main.wj"), code).unwrap();
    
    // Compile
    let wj_binary = std::env::var("CARGO_BIN_EXE_wj")
        .unwrap_or_else(|_| {
            let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
            format!("{}/target/release/wj", manifest_dir)
        });
    
    let output = Command::new(&wj_binary)
        .arg("build")
        .arg("main.wj")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Check generated code
    let generated_code = std::fs::read_to_string(test_dir.join("build/main.rs"))
        .expect("Failed to read generated code");
    
    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);
    
    if !output.status.success() {
        panic!("Compilation failed!\nstdout: {}\nstderr: {}\ngenerated:\n{}", stdout, stderr, generated_code);
    }
    
    // Verify the struct initialization is correct IN THE LOOP
    assert!(generated_code.contains("types::Vertex {") || generated_code.contains("types::Vertex{"), 
        "Generated code should contain 'types::Vertex {{' for qualified struct initialization in loop\nGenerated:\n{}", generated_code);
    assert!(!generated_code.contains("HashMap::from"), 
        "Generated code should NOT convert struct init to HashMap in loop\nGenerated:\n{}", generated_code);
}


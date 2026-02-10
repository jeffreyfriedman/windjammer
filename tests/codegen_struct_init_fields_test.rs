// TDD Test: Verify struct initialization fields are not dropped
// Bug: E0423: expected value, found struct `ffi::GpuVertex`
// Root Cause: Struct initialization fields are being dropped in code generation
// Fix: Ensure struct initialization with fields is properly generated

use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_init_with_array_fields() {
    let code = r#"
        struct GpuVertex {
            position: [f32; 3],
            color: [f32; 4],
        }
        
        fn main() {
            let x = 1.0;
            let y = 2.0;
            let z = 3.0;
            
            let vertex = GpuVertex {
                position: [x, y, z],
                color: [1.0, 0.0, 0.0, 1.0],
            };
        }
    "#;

    // Create temporary test directory
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_struct_init_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    std::fs::create_dir_all(&test_dir).unwrap();

    // Write test file
    std::fs::write(test_dir.join("main.wj"), code).unwrap();

    // Compile
    let wj_binary = std::env::var("CARGO_BIN_EXE_wj").unwrap_or_else(|_| {
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
        panic!(
            "Compilation failed!\nstdout: {}\nstderr: {}\ngenerated:\n{}",
            stdout, stderr, generated_code
        );
    }

    // Verify the struct initialization is complete
    assert!(
        generated_code.contains("position: ["),
        "Generated code should contain 'position: [' for struct field initialization"
    );
    assert!(
        generated_code.contains("color: ["),
        "Generated code should contain 'color: [' for struct field initialization"
    );
    assert!(
        !generated_code.contains("GpuVertex;"),
        "Generated code should not have incomplete struct initialization (GpuVertex;)"
    );
}

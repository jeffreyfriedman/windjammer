// TDD Test: Verify struct initialization in loops is not converted to HashMap
// Bug: E0423 + E0425: Struct init `ffi::GpuVertex { position: [...], normal: [...] }`
//      becomes `ffi::GpuVertex; HashMap::from([(position, [...]), (normal, [...])])`
// Root Cause: Struct field initialization in loops is incorrectly parsed as HashMap
// Fix: Ensure struct initialization is properly distinguished from HashMap creation

use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_init_in_for_loop() {
    let code = r#"
        struct GpuVertex {
            position: [f32; 3],
            normal: [f32; 3],
            color: [f32; 4],
        }
        
        struct Vertex {
            position: Vec3,
            normal: Vec3,
            color: Color,
        }
        
        struct Vec3 {
            x: f32,
            y: f32,
            z: f32,
        }
        
        struct Color {
            r: f32,
            g: f32,
            b: f32,
            a: f32,
        }
        
        fn convert_vertices(vertices: &Vec<Vertex>) -> Vec<GpuVertex> {
            let mut gpu_vertices = Vec::new();
            
            for vertex in vertices {
                let gpu_vertex = GpuVertex {
                    position: [vertex.position.x, vertex.position.y, vertex.position.z],
                    normal: [vertex.normal.x, vertex.normal.y, vertex.normal.z],
                    color: [vertex.color.r, vertex.color.g, vertex.color.b, vertex.color.a],
                };
                gpu_vertices.push(gpu_vertex);
            }
            
            gpu_vertices
        }
        
        fn main() {
            let vertices = Vec::new();
            let gpu = convert_vertices(&vertices);
        }
    "#;

    // Create temporary test directory
    let test_dir = std::env::temp_dir().join(format!(
        "wj_test_struct_loop_{}_{}",
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

    // Verify the struct initialization is NOT converted to HashMap
    assert!(
        generated_code.contains("GpuVertex {"),
        "Generated code should contain 'GpuVertex {{' for struct initialization"
    );
    assert!(
        generated_code.contains("position: ["),
        "Generated code should contain 'position: [' for struct field initialization"
    );
    assert!(
        !generated_code.contains("HashMap::from"),
        "Generated code should NOT convert struct init to HashMap"
    );
    assert!(
        !generated_code.contains("GpuVertex;\n"),
        "Generated code should not have incomplete struct initialization (GpuVertex;)"
    );
}

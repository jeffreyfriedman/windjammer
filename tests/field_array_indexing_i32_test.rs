// TDD Test: Field array indexing with i32 should auto-cast to usize
//
// Bug: Compiler auto-casts for `items[i]` but NOT for `agent.field[i]`
//
// Working case:
//   let items = [1, 2, 3]
//   let i = 0  // i32
//   let x = items[i]  // ✅ Generates: items[i as usize]
//
// Broken case:
//   struct Agent { neighbors: Vec<u64> }
//   let agent = Agent { neighbors: vec![1, 2, 3] }
//   let i = 0  // i32
//   let id = agent.neighbors[i]  // ❌ Generates: agent.neighbors[i] (no cast!)

use std::fs;
use std::process::Command;

#[test]
fn test_field_array_indexing_with_i32() {
    let test_wj = r#"
struct Agent {
    neighbors: Vec<u64>
}

fn test_indexing() {
    let agent = Agent { neighbors: vec![1, 2, 3] }
    let i = 0
    let id = agent.neighbors[i]
    println!("ID: {}", id)
}
"#;
    
    let test_file = "/tmp/test_field_index.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    // Transpile
    let output = Command::new("./target/release/wj")
        .args(&["build", "--no-cargo", test_file])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    // Read generated Rust
    let rs_file = "./build/test_field_index.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Verify auto-cast is generated for field access + indexing
    assert!(
        rust_code.contains("agent.neighbors[i as usize]") ||
        rust_code.contains("agent.neighbors[(i as usize)]"),
        "Should auto-cast field array indexing: agent.neighbors[i as usize]\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Field array indexing test PASSED");
}

#[test]
fn test_vec_indexing_with_loop() {
    // Real-world pattern from steering.wj
    let test_wj = r#"
struct SteeringAgent {
    neighbors: Vec<u64>
}

fn process_neighbors(agent: SteeringAgent) {
    let mut i = 0
    while i < agent.neighbors.len() {
        let neighbor_id = agent.neighbors[i]
        println!("{}", neighbor_id)
        i = i + 1
    }
}
"#;
    
    let test_file = "/tmp/test_vec_loop.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", "--no-cargo", test_file])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_vec_loop.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Verify auto-cast in loop body
    assert!(
        rust_code.contains("agent.neighbors[i as usize]"),
        "Should auto-cast in loop: agent.neighbors[i as usize]\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Vec loop indexing test PASSED");
}

#[test]
fn test_struct_field_compound_assignment() {
    // Real-world pattern from object_pool.wj
    let test_wj = r#"
struct Counter {
    value: usize
}

impl Counter {
    fn increment(self) {
        self.value = self.value + 1
    }
}
"#;
    
    let test_file = "/tmp/test_struct_field.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", "--no-cargo", test_file])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_struct_field.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Verify struct field compound assignment uses correct type
    assert!(
        rust_code.contains("self.value += 1_usize"),
        "Should generate: self.value += 1_usize (matching field type)\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Struct field compound assignment test PASSED");
}

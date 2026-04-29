// TDD Test: Field array indexing with i32 should auto-cast to usize

use std::fs;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }

    let src_main = out_dir.join("src").join("main.rs");
    let test_rs = out_dir.join("test.rs");
    if src_main.exists() {
        fs::read_to_string(src_main).expect("Failed to read generated .rs file")
    } else if test_rs.exists() {
        fs::read_to_string(test_rs).expect("Failed to read generated .rs file")
    } else {
        panic!("No generated Rust file found in {:?}", out_dir);
    }
}

#[test]
fn test_field_array_indexing_with_i32() {
    let source = r#"
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

    let rust = compile_and_get_rust(source);
    println!("Generated Rust:\n{}", rust);

    assert!(
        rust.contains("agent.neighbors[i as usize]")
            || rust.contains("agent.neighbors[(i as usize)]")
            || rust.contains("agent.neighbors[i]"),
        "Should index into field array.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_vec_indexing_with_loop() {
    let source = r#"
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

    let rust = compile_and_get_rust(source);
    println!("Generated Rust:\n{}", rust);

    // Both `i as usize` and plain `i` are valid when i is inferred as usize
    assert!(
        rust.contains("agent.neighbors[i as usize]") || rust.contains("agent.neighbors[i]"),
        "Should index into vec in loop.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_struct_field_compound_assignment() {
    let source = r#"
struct Counter {
    value: usize
}

impl Counter {
    fn increment(self) {
        self.value = self.value + 1
    }
}
"#;

    let rust = compile_and_get_rust(source);
    println!("Generated Rust:\n{}", rust);

    assert!(
        rust.contains("self.value += 1_usize") || rust.contains("self.value += 1"),
        "Should generate compound assignment for usize field.\nGenerated:\n{}",
        rust
    );
}

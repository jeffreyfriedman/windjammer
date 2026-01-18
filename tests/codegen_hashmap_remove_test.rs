use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_hashmap_remove_auto_borrow() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_hashmap_remove");
    
    fs::create_dir_all(&test_dir).unwrap();

    // Test that HashMap.remove() auto-borrows for Copy type keys
    // This is different from Vec.remove(index: usize) which takes by value
    let test_content = r#"
use std::collections::HashMap;

fn remove_entity(entities: &mut HashMap<i64, String>, entity_id: i64) {
    entities.remove(entity_id);
}

fn main() {
    let mut entities = HashMap::new();
    entities.insert(1, "Enemy".to_string());
    remove_entity(&mut entities, 1);
}
"#;

    let test_file = test_dir.join("hashmap_remove.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join("hashmap_remove.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should auto-borrow: entities.remove(&entity_id)
    // NOT entities.remove(entity_id) which would fail for HashMap
    assert!(
        rust_code.contains("entities.remove(&entity_id)"),
        "Expected auto-borrow for HashMap::remove.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("hashmap_remove.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile.\nRustc errors:\n{}",
        compile_stderr
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_vec_remove_no_borrow() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_vec_remove");
    
    fs::create_dir_all(&test_dir).unwrap();

    // Test that Vec.remove() does NOT auto-borrow for usize index
    let test_content = r#"
fn remove_at_index(items: &mut Vec<String>, index: usize) -> String {
    items.remove(index)
}

fn main() {
    let mut items = vec!["a".to_string(), "b".to_string()];
    let removed = remove_at_index(&mut items, 0);
}
"#;

    let test_file = test_dir.join("vec_remove.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join("vec_remove.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should NOT auto-borrow: items.remove(index)
    // NOT items.remove(&index) which would fail for Vec
    assert!(
        rust_code.contains("items.remove(index)"),
        "Expected NO auto-borrow for Vec::remove (index is usize).\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("vec_remove.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile.\nRustc errors:\n{}",
        compile_stderr
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}



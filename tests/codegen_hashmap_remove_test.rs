#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_remove_auto_borrow() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_hashmap_remove");

    fs::create_dir_all(&test_dir).unwrap();

    // Focused on remove key borrow; keep main minimal so rustc validates that path.
    let test_content = r#"
use std::collections::HashMap

fn remove_entity(mut entities: HashMap<i64, string>, entity_id: i64) {
    entities.remove(entity_id)
}

fn main() {
    let mut entities = HashMap::new()
    let id = 1
    remove_entity(entities, id)
}
"#;

    let test_file = test_dir.join("hashmap_remove.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg("hashmap_remove.wj")
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
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_no_borrow() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_vec_remove");

    fs::create_dir_all(&test_dir).unwrap();

    // Vec.remove takes usize by value — must not emit `&index`.
    // Idiomatic Windjammer: no explicit `&` / `.to_string()` in source.
    let test_content = r#"
fn remove_at_index(mut items: Vec<string>, index: usize) -> string {
    items.remove(index)
}

fn main() {
    let mut items = Vec::new()
    items.push("a")
    items.push("b")
    let _removed = remove_at_index(items, 0)
}
"#;

    let test_file = test_dir.join("vec_remove.wj");
    fs::write(&test_file, test_content).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg("vec_remove.wj")
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

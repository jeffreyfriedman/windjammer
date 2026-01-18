use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_hashmap_contains_key_auto_borrow() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_hashmap_auto_borrow");
    
    fs::create_dir_all(&test_dir).unwrap();

    // Test that HashMap methods auto-borrow Copy type arguments
    let test_content = r#"
use std::collections::HashMap;

fn check_user(users: &HashMap<i64, String>, user_id: i64) -> bool {
    users.contains_key(user_id)
}

fn main() {
    let mut users = HashMap::new();
    users.insert(1, "Alice".to_string());
    let exists = check_user(&users, 1);
}
"#;

    let test_file = test_dir.join("hashmap_test.wj");
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

    // Check the generated Rust code
    let rust_file = test_dir.join("build").join("hashmap_test.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should auto-borrow: users.contains_key(&user_id)
    assert!(
        rust_code.contains("users.contains_key(&user_id)"),
        "Expected auto-borrow for HashMap::contains_key.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles with rustc
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("hashmap_test.rs")
        .output()
        .expect("Failed to run rustc");

    let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
    assert!(
        compile_output.status.success(),
        "Expected generated code to compile with rustc.\nRustc errors:\n{}",
        compile_stderr
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_hashmap_get_auto_borrow() {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_hashmap_get");
    
    fs::create_dir_all(&test_dir).unwrap();

    let test_content = r#"
use std::collections::HashMap;

fn get_user_name(users: &HashMap<i64, String>, user_id: i64) -> String {
    if let Some(name) = users.get(user_id) {
        name.clone()
    } else {
        "Unknown".to_string()
    }
}

fn main() {
    let mut users = HashMap::new();
    users.insert(1, "Alice".to_string());
    let name = get_user_name(&users, 1);
}
"#;

    let test_file = test_dir.join("hashmap_get.wj");
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

    let rust_file = test_dir.join("build").join("hashmap_get.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap();
    println!("Generated Rust:\n{}", rust_code);

    // The generated code should auto-borrow: users.get(&user_id)
    assert!(
        rust_code.contains("users.get(&user_id)"),
        "Expected auto-borrow for HashMap::get.\nGenerated code:\n{}",
        rust_code
    );

    // Verify it compiles
    let compile_output = Command::new("rustc")
        .current_dir(test_dir.join("build"))
        .arg("--crate-type")
        .arg("bin")
        .arg("hashmap_get.rs")
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


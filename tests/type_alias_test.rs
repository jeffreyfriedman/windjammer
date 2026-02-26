// TDD Test: Type Alias Support
// Test that Windjammer can parse and generate type aliases

use std::fs;
use std::path::PathBuf;
use std::env;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_simple_type_alias() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_type_alias_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let source = r#"
pub type UserId = u32

pub fn get_user() -> UserId {
    42
}

fn main() {
    let id: UserId = get_user()
    println!("{}", id)
}
"#;

    fs::write(test_dir.join("main.wj"), source).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("main.wj"))
        .arg("--output")
        .arg(test_dir.join("output"))
        .arg("--target")
        .arg("rust")
        .output()
        .expect("Failed to run wj command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed:\n{}", stderr);
    }
    
    let result = fs::read_to_string(test_dir.join("output/main.rs"))
        .expect("Failed to read generated Rust file");
    
    // Should generate "pub type UserId = u32;"
    assert!(result.contains("type UserId"), 
        "Expected 'type UserId' in generated code, got: {}", result);
    assert!(result.contains("u32"), 
        "Expected 'u32' in type alias, got: {}", result);
    
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_type_alias() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_type_alias_string_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let source = r#"
pub type QuestId = String

pub fn create_quest(id: QuestId) -> QuestId {
    id
}

fn main() {
    let quest: QuestId = "rescue_silas".to_string()
    println!("{}", create_quest(quest))
}
"#;

    fs::write(test_dir.join("main.wj"), source).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("main.wj"))
        .arg("--output")
        .arg(test_dir.join("output"))
        .arg("--target")
        .arg("rust")
        .output()
        .expect("Failed to run wj command");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed:\n{}", stderr);
    }
    
    let result = fs::read_to_string(test_dir.join("output/main.rs"))
        .expect("Failed to read generated Rust file");
    
    // Should generate "pub type QuestId = String;"
    assert!(result.contains("type QuestId"), 
        "Expected 'type QuestId' in generated code, got: {}", result);
    assert!(result.contains("String"), 
        "Expected 'String' in type alias, got: {}", result);
    
    fs::remove_dir_all(&test_dir).ok();
}

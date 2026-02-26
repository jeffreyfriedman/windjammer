// Test return statement generation
// Dogfooding bug: return statements in else blocks

use std::fs;
use std::path::PathBuf;
use std::env;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_else_block() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_return_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let source = r#"
pub fn check_slot(found: bool) -> bool {
    if found {
        let x = 42
    } else {
        return false
    }
    true
}

fn main() {
    println!("{}", check_slot(true))
}
"#;

    fs::write(test_dir.join("main.wj"), source).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let _output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("main.wj"))
        .arg("--output")
        .arg(test_dir.join("output"))
        .arg("--target")
        .arg("rust")
        .output()
        .expect("Failed to run wj command");
    
    // Read generated Rust code (transpilation succeeds even if cargo build fails)
    let result = fs::read_to_string(test_dir.join("output/main.rs"))
        .expect("Failed to read generated Rust file");
    
    // Should generate "return false" not just "false"
    assert!(result.contains("return false"), 
        "return statement should be preserved in else block, got: {}", result);
    
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_if_block() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_return_if_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let source = r#"
pub fn early_exit(x: i32) -> bool {
    if x > 10 {
        return true
    }
    false
}

fn main() {
    println!("{}", early_exit(15))
}
"#;

    fs::write(test_dir.join("main.wj"), source).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let _output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("main.wj"))
        .arg("--output")
        .arg(test_dir.join("output"))
        .arg("--target")
        .arg("rust")
        .output()
        .expect("Failed to run wj command");
    
    let result = fs::read_to_string(test_dir.join("output/main.rs"))
        .expect("Failed to read generated Rust file");
    
    // Should generate "return true"
    assert!(result.contains("return true"), 
        "return statement should be preserved in if block");
    
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_in_nested_block() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_return_nested_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let source = r#"
pub fn nested_return(a: bool, b: bool) -> bool {
    if a {
        if b {
            return true
        } else {
            return false
        }
    }
    false
}

fn main() {
    println!("{}", nested_return(true, false))
}
"#;

    fs::write(test_dir.join("main.wj"), source).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let _output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(test_dir.join("main.wj"))
        .arg("--output")
        .arg(test_dir.join("output"))
        .arg("--target")
        .arg("rust")
        .output()
        .expect("Failed to run wj command");
    
    let result = fs::read_to_string(test_dir.join("output/main.rs"))
        .expect("Failed to read generated Rust file");
    
    // Both returns should be preserved
    assert!(result.contains("return true"), 
        "return true should be in generated code");
    assert!(result.contains("return false"), 
        "return false should be in generated code");
    
    fs::remove_dir_all(&test_dir).ok();
}

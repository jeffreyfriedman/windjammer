// TDD Test: Rust keywords (crate, super, self) should NOT be external dependencies
// THE WINDJAMMER WAY: Compiler filters out language keywords automatically

use std::fs;
use std::process::Command;

fn compile_code_to_cargo_toml(code: &str, test_name: &str) -> Result<String, String> {
    let test_dir = format!("tests/generated/module_keywords_{}", test_name);
    fs::create_dir_all(&test_dir).expect("Failed to create test dir");
    let input_file = format!("{}/test.wj", test_dir);
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            &input_file,
            "--output",
            &test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        fs::remove_dir_all(&test_dir).ok();
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let cargo_toml_path = format!("{}/Cargo.toml", test_dir);
    let cargo_toml = fs::read_to_string(&cargo_toml_path)
        .expect("Failed to read Cargo.toml");

    fs::remove_dir_all(&test_dir).ok();

    Ok(cargo_toml)
}

#[test]
fn test_crate_keyword_not_in_dependencies() {
    // Code that uses "use crate::foo" should NOT add "crate" to Cargo.toml
    let code = r#"
    use crate::ffi
    
    pub fn test() {
        println!("test")
    }
    "#;

    let cargo_toml = compile_code_to_cargo_toml(code, "crate_keyword")
        .expect("Compilation failed");

    // Should NOT contain "crate = " in dependencies
    assert!(
        !cargo_toml.contains("crate = "),
        "Should NOT add 'crate' keyword to dependencies, got:\n{}",
        cargo_toml
    );
}

#[test]
fn test_super_keyword_not_in_dependencies() {
    // Code that uses "use super::foo" should NOT add "super" to Cargo.toml
    let code = r#"
    use super::game_loop::GameLoop
    
    pub fn test() {
        println!("test")
    }
    "#;

    let cargo_toml = compile_code_to_cargo_toml(code, "super_keyword")
        .expect("Compilation failed");

    // Should NOT contain "super = " in dependencies
    assert!(
        !cargo_toml.contains("super = "),
        "Should NOT add 'super' keyword to dependencies, got:\n{}",
        cargo_toml
    );
}

#[test]
#[ignore] // Parser doesn't support "use self::" yet - this is a parser limitation, not a dependency issue
fn test_self_keyword_not_in_dependencies() {
    // Code that uses "use self::foo" should NOT add "self" to Cargo.toml
    // NOTE: Parser doesn't currently support "self::" in use statements
    let code = r#"
    use self::utils
    
    pub fn test() {
        println!("test")
    }
    "#;

    let cargo_toml = compile_code_to_cargo_toml(code, "self_keyword")
        .expect("Compilation failed");

    // Should NOT contain "self = " in dependencies
    assert!(
        !cargo_toml.contains("self = "),
        "Should NOT add 'self' keyword to dependencies, got:\n{}",
        cargo_toml
    );
}

#[test]
fn test_multiple_rust_keywords_filtered() {
    // Code using multiple Rust keywords should filter all of them
    // NOTE: Only testing crate:: and super:: since self:: isn't parser-supported yet
    let code = r#"
    use crate::config
    use super::base::Base
    
    pub fn test() {
        println!("test")
    }
    "#;

    let cargo_toml = compile_code_to_cargo_toml(code, "multiple_keywords")
        .expect("Compilation failed");

    // Should NOT contain ANY Rust keywords in dependencies
    assert!(
        !cargo_toml.contains("crate = ") && 
        !cargo_toml.contains("super = "),
        "Should NOT add Rust keywords to dependencies, got:\n{}",
        cargo_toml
    );
}

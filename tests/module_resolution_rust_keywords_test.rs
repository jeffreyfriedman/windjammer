#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

// TDD Test: Rust keywords (crate, super, self) should NOT be external dependencies
// THE WINDJAMMER WAY: Compiler filters out language keywords automatically

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_crate_keyword_not_in_dependencies() {
    // Code that uses "use crate::foo" should NOT add "crate" to Cargo.toml
    let code = r#"
    use crate::ffi
    
    pub fn test() {
        println!("test")
    }
    "#;

    let cargo_toml = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT contain "crate = " in dependencies
    assert!(
        !cargo_toml.contains("crate = "),
        "Should NOT add 'crate' keyword to dependencies, got:\n{}",
        cargo_toml
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_super_keyword_not_in_dependencies() {
    // Code that uses "use super::foo" should NOT add "super" to Cargo.toml
    let code = r#"
    use super::game_loop::GameLoop
    
    pub fn test() {
        println!("test")
    }
    "#;

    let cargo_toml = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT contain "super = " in dependencies
    assert!(
        !cargo_toml.contains("super = "),
        "Should NOT add 'super' keyword to dependencies, got:\n{}",
        cargo_toml
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_self_keyword_not_in_dependencies() {
    // Code that uses "use self::foo" should NOT add "self" to Cargo.toml
    let code = r#"
    use self::utils
    
    pub fn test() {
        println!("test")
    }
    "#;

    let cargo_toml = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT contain "self = " in dependencies
    assert!(
        !cargo_toml.contains("self = "),
        "Should NOT add 'self' keyword to dependencies, got:\n{}",
        cargo_toml
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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

    let cargo_toml = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT contain ANY Rust keywords in dependencies
    assert!(
        !cargo_toml.contains("crate = ") && !cargo_toml.contains("super = "),
        "Should NOT add Rust keywords to dependencies, got:\n{}",
        cargo_toml
    );
}

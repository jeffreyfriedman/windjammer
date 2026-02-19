//! TDD Test: Unit struct syntax
//! WINDJAMMER PHILOSOPHY: Support zero-field structs with semicolon syntax
//! Unit structs are useful for marker types, singleton patterns, and FFI

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = test_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");

    Ok(generated)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_struct_simple() {
    // TDD: Simple unit struct
    let code = r#"
    pub struct Marker;
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate Rust unit struct with semicolon
    assert!(
        generated.contains("pub struct Marker;"),
        "Unit struct should end with semicolon. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_struct_with_impl() {
    // TDD: Unit struct with implementation
    let code = r#"
    pub struct Logger;
    
    impl Logger {
        pub fn log(&self, message: &str) -> int {
            return message.len() as int
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("pub struct Logger;"),
        "Unit struct should be generated. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("impl Logger {"),
        "Impl block should be generated. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_struct_multiple() {
    // TDD: Multiple unit structs
    let code = r#"
    pub struct TypeA;
    pub struct TypeB;
    pub struct TypeC;
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("pub struct TypeA;"),
        "TypeA should be generated. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("pub struct TypeB;"),
        "TypeB should be generated. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("pub struct TypeC;"),
        "TypeC should be generated. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_struct_with_trait_impl() {
    // TDD: Unit struct implementing a trait
    let code = r#"
    pub trait Processor {
        fn process(&self) -> int;
    }
    
    pub struct SimpleProcessor;
    
    impl Processor for SimpleProcessor {
        fn process(&self) -> int {
            return 42
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("pub struct SimpleProcessor;"),
        "Unit struct should be generated. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("impl Processor for SimpleProcessor"),
        "Trait impl should be generated. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_struct_instantiation() {
    // TDD: Unit struct can be instantiated
    let code = r#"
    pub struct Token;
    
    pub fn create_token() -> Token {
        return Token
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("pub struct Token;"),
        "Unit struct should be generated. Generated:\n{}",
        generated
    );

    // Unit struct instantiation in Rust is just the name (implicit or explicit return)
    assert!(
        generated.contains("Token\n}") || generated.contains("return Token;"),
        "Unit struct instantiation should be simple name. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_struct_with_visibility() {
    // TDD: Unit structs with different visibility modifiers
    let code = r#"
    pub struct Public;
    struct Private;
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("pub struct Public;"),
        "Public unit struct should have pub. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("struct Private;") || generated.contains("pub(crate) struct Private;"),
        "Private unit struct should be generated. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_unit_struct_with_decorators() {
    // TDD: Unit struct with derive decorators
    let code = r#"
    @derive(Debug, Clone, Copy)
    pub struct Marker;
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("pub struct Marker;"),
        "Unit struct should be generated. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("#[derive(Debug, Clone, Copy)]"),
        "Derive decorators should be generated. Generated:\n{}",
        generated
    );
}

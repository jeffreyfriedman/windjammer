// TDD Test: Literals in struct fields should infer to field type (u32, u8, etc.)
//
// Bug: MyStruct { id: 1 } generates id: 1_i32 when field is u32
// Root cause: Int inference defaults to i32 without checking struct field types
// Rust: struct field expects u32, literal should be u32
//
// Fix: Constrain literals to match struct field types

use std::fs;
use std::process::Command;

#[test]
fn test_u32_struct_field_literal() {
    let test_wj = r#"
pub struct Entity {
    pub id: u32,
    pub health: u32
}

fn create_entity() -> Entity {
    Entity {
        id: 1,
        health: 100
    }
}
"#;
    
    let test_file = "/tmp/test_u32_field.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_u32_field.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should generate u32 literals, NOT i32
    assert!(
        rust_code.contains("id: 1_u32") || (rust_code.contains("id: 1") && !rust_code.contains("id: 1_i32")),
        "Should generate u32 literal for u32 field\nGenerated:\n{}", 
        rust_code
    );
    
    assert!(
        rust_code.contains("health: 100_u32") || (rust_code.contains("health: 100") && !rust_code.contains("health: 100_i32")),
        "Should generate u32 literal for u32 field\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ u32 struct field literal test PASSED");
}

#[test]
fn test_u8_struct_field_literal() {
    let test_wj = r#"
struct Color {
    r: u8,
    g: u8,
    b: u8
}

fn create_red() -> Color {
    Color { r: 255, g: 0, b: 0 }
}
"#;
    
    let test_file = "/tmp/test_u8_field.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_u8_field.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should NOT generate i32 literals
    assert!(
        !rust_code.contains("255_i32") && !rust_code.contains("0_i32"),
        "Should NOT generate i32 literals for u8 fields\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ u8 struct field literal test PASSED");
}

#[test]
fn test_nested_struct_u32_literal() {
    let test_wj = r#"
struct Inner {
    value: u32
}

struct Outer {
    inner: Inner,
    count: u32
}

fn create_nested() -> Outer {
    Outer {
        inner: Inner { value: 42 },
        count: 10
    }
}
"#;
    
    let test_file = "/tmp/test_nested_u32.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_nested_u32.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should NOT have i32 literals
    assert!(
        !rust_code.contains("42_i32") && !rust_code.contains("10_i32"),
        "Nested struct u32 fields should NOT generate i32 literals\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Nested struct u32 literal test PASSED");
}

#[test]
fn test_vec_of_structs_with_u32_literals() {
    let test_wj = r#"
pub struct Choice {
    pub id: u32,
    pub text: String
}

fn create_choices() -> Vec<Choice> {
    vec![
        Choice { id: 1, text: "First".to_string() },
        Choice { id: 2, text: "Second".to_string() },
        Choice { id: 3, text: "Third".to_string() }
    ]
}
"#;
    
    let test_file = "/tmp/test_vec_struct_u32.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_vec_struct_u32.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should NOT generate i32 literals in vec of structs
    assert!(
        !rust_code.contains("id: 1_i32") && 
        !rust_code.contains("id: 2_i32") && 
        !rust_code.contains("id: 3_i32"),
        "Vec of structs should generate u32 literals, not i32\nGenerated:\n{}", 
        rust_code
    );
    
    // Should generate u32 literals
    let has_u32 = rust_code.contains("id: 1_u32") || 
                  (rust_code.contains("id: 1") && !rust_code.contains("1_i32"));
    assert!(
        has_u32,
        "Should generate u32 literals for u32 fields\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Vec of structs with u32 literals test PASSED");
}

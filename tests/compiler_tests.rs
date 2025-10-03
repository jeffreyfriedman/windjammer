use std::path::PathBuf;
use std::process::Command;

/// Helper to compile a test fixture and return the generated Rust code
fn compile_fixture(fixture_name: &str) -> Result<String, String> {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(format!("{}.wj", fixture_name));
    
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_output");
    std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    
    // Run the compiler
    let compiler_output = Command::new(env!("CARGO_BIN_EXE_windjammer"))
        .args(&["build", "--path", fixture_path.to_str().unwrap(), "--output", output_dir.to_str().unwrap()])
        .output()
        .map_err(|e| format!("Failed to run compiler: {}", e))?;
    
    if !compiler_output.status.success() {
        return Err(format!(
            "Compiler failed: {}",
            String::from_utf8_lossy(&compiler_output.stderr)
        ));
    }
    
    // Read generated Rust code
    let rust_file = output_dir.join(format!("{}.rs", fixture_name));
    std::fs::read_to_string(rust_file).map_err(|e| format!("Failed to read generated code: {}", e))
}

#[test]
fn test_automatic_reference_insertion() {
    let generated = compile_fixture("auto_reference").expect("Compilation failed");
    
    // Check that function signatures have borrowed parameters
    assert!(generated.contains("fn double(x: &i64) -> i64"), 
        "Function should infer borrowed parameter");
    assert!(generated.contains("fn greet(name: &String)"),
        "Function should infer borrowed string parameter");
    
    // Check that call sites have automatic & insertion
    assert!(generated.contains("double(&x)"),
        "Call site should auto-insert &");
    assert!(generated.contains("greet(&name)"),
        "Call site should auto-insert & for string");
    
    println!("✓ Automatic reference insertion works");
}

#[test]
fn test_string_interpolation() {
    let generated = compile_fixture("string_interpolation").expect("Compilation failed");
    
    // Check that interpolated strings are converted to println! with format args
    assert!(generated.contains(r#"println!("Hello, {}! You are {} years old.", name, age)"#),
        "String interpolation should flatten into println!");
    
    // Check that expressions in interpolation work
    assert!(generated.contains(r#"println!("{} + {} = {}", x, y, x + y)"#),
        "String interpolation should handle expressions");
    
    println!("✓ String interpolation works");
}

#[test]
fn test_pipe_operator() {
    let generated = compile_fixture("pipe_operator").expect("Compilation failed");
    
    // Check that pipe operator is transformed to nested calls
    // 5 |> double |> add_ten becomes add_ten(&double(&5))
    assert!(generated.contains("add_ten(&double(&5))"),
        "Pipe operator should transform to nested calls with auto-reference");
    
    println!("✓ Pipe operator works");
}

#[test]
fn test_structs_and_impl() {
    let generated = compile_fixture("structs_and_impl").expect("Compilation failed");
    
    // Check struct definition
    assert!(generated.contains("struct Point"),
        "Struct should be generated");
    
    // Check impl block
    assert!(generated.contains("impl Point"),
        "Impl block should be generated");
    
    // Check associated function
    assert!(generated.contains("fn new("),
        "Associated function should be generated");
    
    // Check method with &self
    assert!(generated.contains("fn distance(&self)"),
        "Method with &self should be generated");
    
    println!("✓ Structs and impl blocks work");
}

#[test]
fn test_combined_features() {
    // Test that automatic reference insertion works with pipe operator
    let fixture = r#"
fn double(x: int) -> int { x * 2 }
fn main() {
    let result = 5 |> double
    println!("Result: ${result}")
}
"#;
    
    // Write temporary fixture
    let temp_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("temp_combined.wj");
    std::fs::write(&temp_path, fixture).expect("Failed to write temp fixture");
    
    let generated = compile_fixture("temp_combined").expect("Compilation failed");
    
    // Check that both features work together
    assert!(generated.contains("double(&5)"),
        "Pipe operator should work with auto-reference");
    assert!(generated.contains(r#"println!("Result: {}", result)"#),
        "String interpolation should work in combined test");
    
    // Clean up
    std::fs::remove_file(temp_path).ok();
    
    println!("✓ Combined features work");
}

#[test]
fn test_ownership_inference_borrowed() {
    // Test that parameters used read-only are inferred as borrowed
    let fixture = r#"
fn print_twice(x: int) {
    println!("{}", x)
    println!("{}", x)
}

fn main() {
    print_twice(42)
}
"#;
    
    let temp_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("temp_borrowed.wj");
    std::fs::write(&temp_path, fixture).expect("Failed to write temp fixture");
    
    let generated = compile_fixture("temp_borrowed").expect("Compilation failed");
    
    // Should infer borrowed since x is only read
    assert!(generated.contains("fn print_twice(x: &i64)"),
        "Read-only parameter should be inferred as borrowed");
    assert!(generated.contains("print_twice(&42)"),
        "Call site should auto-insert &");
    
    // Clean up
    std::fs::remove_file(temp_path).ok();
    
    println!("✓ Ownership inference (borrowed) works");
}

#[test]
fn test_ternary_operator() {
    let generated = compile_fixture("ternary_operator").expect("Compilation failed");
    
    // Check that ternary is converted to if-else expression
    assert!(generated.contains("if x > 0 { \"positive\" } else { \"non-positive\" }"),
        "Simple ternary should convert to if-else");
    
    // Check nested ternary (right-associative)
    assert!(generated.contains("if x >= 90"),
        "Nested ternary should be handled");
    
    // Check ternary with variables
    assert!(generated.contains("if x > y { x } else { y }"),
        "Ternary with variables should work");
    
    println!("✓ Ternary operator works");
}

#[test]
fn test_smart_auto_derive() {
    let generated = compile_fixture("smart_auto_derive").expect("Compilation failed");
    
    // Check Point: all fields are Copy (int, int)
    // Should derive: Debug, Clone, Copy, PartialEq, Eq, Hash, Default
    assert!(generated.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]") 
            && generated.contains("struct Point"),
        "Point with all Copy fields should derive Debug, Clone, Copy, PartialEq, Eq, Hash, Default");
    
    // Check User: name is String (not Copy), age is int (Copy)
    // Should derive: Debug, Clone, PartialEq, Eq, Hash, Default (NO Copy)
    // String is hashable and comparable, so we get those
    assert!(generated.contains("#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]\nstruct User"),
        "User with String field should derive Debug, Clone, PartialEq, Eq, Hash, Default but NOT Copy");
    
    // Check Container: Vec<int> is not Eq or Hash (only PartialEq)
    // Should derive: Debug, Clone, Default (NO Copy, NO PartialEq/Eq, NO Hash)
    // Note: Our conservative approach doesn't derive PartialEq for Vec (even though it has it)
    let has_container_derive = generated.contains("#[derive(Debug, Clone, Default)]\nstruct Container");
    assert!(has_container_derive,
        "Container with Vec should derive Debug, Clone, Default (no Eq, no Hash, no Copy)");
    
    // Check Config: explicit traits specified
    // Should derive exactly: Debug, Clone, Serialize, Deserialize
    assert!(generated.contains("#[derive(Debug, Clone, Serialize, Deserialize)]") 
            && generated.contains("struct Config"),
        "Config with explicit traits should derive only those traits");
    
    println!("✓ Smart @auto derive works");
}

#[test]
fn test_ownership_inference_mut_borrowed() {
    let generated = compile_fixture("mut_borrowed").expect("Compilation failed");
    
    // Should infer &mut since x is mutated
    assert!(generated.contains("fn increment(x: &mut i64)"),
        "Mutated parameter should be inferred as &mut");
    assert!(generated.contains("increment(&mut counter)"),
        "Call site should auto-insert &mut");
    
    println!("✓ Ownership inference (mut borrowed) works");
}


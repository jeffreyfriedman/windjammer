#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

use std::path::PathBuf;

/// Helper to compile a test fixture and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_automatic_reference_insertion() {
    let generated = test_utils::compile_fixture("auto_reference").expect("Compilation failed");

    // Check that Copy types are passed by value (no mut if not mutated)
    assert!(
        generated.contains("fn double(x: i64) -> i64"),
        "Copy types should be passed by value without mut if not mutated"
    );

    // THE WINDJAMMER WAY: Read-only string parameters infer to &str (idiomatic Rust!)
    assert!(
        generated.contains("fn greet(name: &str)"),
        "Read-only string parameter should infer to &str.\nGenerated:\n{}",
        generated
    );

    // Check that call sites pass Copy types by value (no &)
    assert!(
        generated.contains("double(x)"),
        "Copy types should be passed by value at call site"
    );

    // Check that owned String is borrowed when passed to &str parameter
    assert!(
        generated.contains("greet(&name)"),
        "Owned String should be borrowed when passed to &str parameter.\nGenerated:\n{}",
        generated
    );

    println!("✓ Ownership inference and auto-ref working correctly");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_interpolation() {
    let generated =
        test_utils::compile_fixture("string_interpolation").expect("Compilation failed");

    // Check that interpolated strings are converted to println! with format args
    assert!(
        generated.contains(r#"println!("Hello, {}! You are {} years old.", name, age)"#),
        "String interpolation should flatten into println!"
    );

    // Check that expressions in interpolation work
    assert!(
        generated.contains(r#"println!("{} + {} = {}", x, y, x + y)"#),
        "String interpolation should handle expressions"
    );

    println!("✓ String interpolation works");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_pipe_operator() {
    let generated = test_utils::compile_fixture("pipe_operator").expect("Compilation failed");

    // Check that pipe operator is transformed to nested calls
    // 5 |> double |> add_ten becomes add_ten(double(5_i64)) (int literals get i64 suffix in Rust)
    // No & needed because int/i64 is a Copy type
    assert!(
        generated.contains("add_ten(double(5_i64))"),
        "Pipe operator should transform to nested calls (no & for Copy types)"
    );

    println!("✓ Pipe operator works");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_structs_and_impl() {
    let generated = test_utils::compile_fixture("structs_and_impl").expect("Compilation failed");

    // Check struct definition
    assert!(
        generated.contains("struct Point"),
        "Struct should be generated"
    );

    // Check impl block
    assert!(
        generated.contains("impl Point"),
        "Impl block should be generated"
    );

    // Check associated function
    assert!(
        generated.contains("fn new("),
        "Associated function should be generated"
    );

    // Check method with &self or owned self (Copy structs use by-value receiver)
    assert!(
        generated.contains("fn distance(&self)")
            || generated.contains("fn distance(self)"),
        "Method receiver should be &self or self (Copy): {}",
        generated
    );

    println!("✓ Structs and impl blocks work");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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

    let generated = test_utils::compile_fixture("temp_combined").expect("Compilation failed");

    // Check that both features work together
    // int/i64 is a Copy type, so no & is inserted (literal emitted as 5_i64)
    assert!(
        generated.contains("double(5_i64)"),
        "Pipe operator should work correctly (no & for Copy types)"
    );
    assert!(
        generated.contains(r#"println!("Result: {}", result)"#),
        "String interpolation should work in combined test"
    );

    // Clean up
    std::fs::remove_file(temp_path).ok();

    println!("✓ Combined features work");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ownership_inference_borrowed() {
    // Test that parameters used read-only are inferred correctly
    let fixture = r#"
fn print_twice(x: int) {
    println("{}", x)
    println("{}", x)
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

    let generated = test_utils::compile_fixture("temp_borrowed").expect("Compilation failed");

    // Copy types are passed by value (no mut if not mutated)
    assert!(
        generated.contains("fn print_twice(x: i64)"),
        "Copy types should be passed by value without mut if read-only"
    );
    assert!(
        generated.contains("print_twice(42_i64)"),
        "Copy types should be passed by value at call site"
    );

    // Clean up
    std::fs::remove_file(temp_path).ok();

    println!("✓ Copy type handling works correctly");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ternary_operator() {
    let generated = test_utils::compile_fixture("ternary_operator").expect("Compilation failed");

    // Check that if/else expressions work correctly
    assert!(
        generated.contains("if x > 0")
            && generated.contains("\"positive\"")
            && generated.contains("\"non-positive\""),
        "Simple if/else expression should work"
    );

    // Check nested if/else (else if)
    assert!(
        generated.contains("if x >= 90") && generated.contains("if x >= 80"),
        "Nested if/else should be handled"
    );

    // Check if/else with variables
    assert!(
        generated.contains("if x > y"),
        "If/else with variables should work"
    );

    println!("✓ If/else expressions work correctly");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_smart_auto_derive() {
    let generated = test_utils::compile_fixture("smart_auto_derive").expect("Compilation failed");

    // Check Point: all fields are Copy (int, int)
    // Should derive: Debug, Clone, Copy, PartialEq, Eq, Hash, Default
    assert!(
        generated.contains("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]")
            && generated.contains("struct Point"),
        "Point with all Copy fields should derive Debug, Clone, Copy, PartialEq, Eq, Hash, Default"
    );

    // Check User: name is String (not Copy), age is int (Copy)
    // Should derive: Debug, Clone, PartialEq, Eq, Hash, Default (NO Copy)
    // String is hashable and comparable, so we get those
    assert!(
        generated.contains("#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]")
            && generated.contains("struct User"),
        "User with String field should derive Debug, Clone, PartialEq, Eq, Hash, Default but NOT Copy"
    );

    // Check Container: Vec<int> implements Clone, Debug, Default, PartialEq, and Eq
    // Should derive: Debug, Clone, PartialEq, Eq, Default (NO Copy, NO Hash)
    // Vec<T> is PartialEq and Eq if T is PartialEq/Eq, but NOT Hash (even if T: Hash)
    assert!(
        generated.contains("#[derive(Debug, Clone, PartialEq, Eq, Default)]")
            && generated.contains("struct Container"),
        "Container with Vec should derive Debug, Clone, PartialEq, Eq, Default (no Hash, no Copy)"
    );

    // Check Config: explicit @auto traits are merged with inferred standard traits
    // (see merge_standard_derive_traits — partial lists must not drop Debug/Clone/Eq/etc.)
    // Serde traits sort after the standard block.
    assert!(
        generated.contains(
            "#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]"
        ) && generated.contains("struct Config"),
        "Config should list explicit Serde derives plus inferred standard derives"
    );

    println!("✓ Smart @auto derive works");
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_ownership_inference_mut_borrowed() {
    let generated = test_utils::compile_fixture("mut_borrowed").expect("Compilation failed");

    // Should infer &mut since x is mutated and mutation needs to be visible to caller
    // Currently broken: generates `mut x: i64` instead of `x: &mut i64`
    assert!(
        generated.contains("fn increment(x: &mut i64)"),
        "Mutated parameter should be inferred as &mut"
    );
    assert!(
        generated.contains("increment(&mut counter)"),
        "Call site should auto-insert &mut"
    );

    println!("✓ Ownership inference (mut borrowed) works");
}

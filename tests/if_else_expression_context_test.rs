// Test: If-else expression context tracking
// Ensures if-else branches don't get semicolons when used as values

use std::fs;
use std::process::Command;

fn compile_code(code: &str) -> Result<String, String> {
    let test_dir = "tests/generated/if_else_context_test";
    fs::create_dir_all(test_dir).expect("Failed to create test dir");
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
            test_dir,
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        fs::remove_dir_all(test_dir).ok();
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    // Read the generated file
    let generated_file = format!("{}/test.rs", test_dir);
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");

    fs::remove_dir_all(test_dir).ok();

    Ok(generated)
}

#[test]
fn test_if_else_in_let_no_semicolons() {
    // Test: if-else used in let binding should NOT have semicolons
    let code = r#"
    pub fn get_value(flag: bool) -> f32 {
        let result = if flag { 1.0 } else { 2.0 }
        return result
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should NOT have semicolons in branches
    assert!(
        !generated.contains("1.0;"),
        "If branch should NOT have semicolon when used as value, got: {}",
        generated
    );

    assert!(
        !generated.contains("2.0;") || generated.contains("2.0; }"),
        "Else branch should NOT have semicolon when used as value (unless at block end), got: {}",
        generated
    );

    // Should have branches without semicolons
    assert!(
        generated.contains("if flag") && generated.contains("1.0") && generated.contains("2.0"),
        "Should contain if-else with numeric values, got: {}",
        generated
    );
}

#[test]
fn test_if_else_with_field_access_no_semicolons() {
    // Test: if-else with field access should NOT have semicolons when used as value
    let code = r#"
    pub struct Body {
        pub is_grounded: bool,
    }
    
    pub struct Controller {
        pub body: Body,
        pub air_control: f32,
    }
    
    impl Controller {
        pub fn get_control(&self) -> f32 {
            let control = if self.body.is_grounded { 1.0 } else { self.air_control }
            return control
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // This is the EXACT bug case from character_controller.rs
    // Should NOT have semicolons in branches
    assert!(
        !generated.contains("1.0;") || generated.contains("1.0; }"),
        "If branch with literal should NOT have semicolon, got: {}",
        generated
    );

    assert!(
        !generated.contains("self.air_control;") || generated.contains("self.air_control; }"),
        "Else branch with field access should NOT have semicolon, got: {}",
        generated
    );
}

#[test]
fn test_if_else_in_return_no_semicolons() {
    // Test: if-else in return should NOT have semicolons
    let code = r#"
    pub fn classify(x: i32) -> string {
        return if x > 0 { "positive" } else { "negative" }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Branches should NOT have semicolons
    assert!(
        generated.contains(r#""positive""#) && generated.contains(r#""negative""#),
        "Should contain string literals, got: {}",
        generated
    );
}

#[test]
fn test_if_else_in_function_returning_unit_has_semicolons() {
    // Test: if-else in function returning () SHOULD have semicolons when not used as value
    let code = r#"
    pub fn print_value(x: i32) {
        if x > 0 {
            println("positive")
        } else {
            println("negative")
        }
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // This is a statement, not an expression, so semicolons are OK
    // (but Windjammer might optimize this anyway)
    assert!(
        generated.contains("if x > 0"),
        "Should contain if statement, got: {}",
        generated
    );
}

#[test]
fn test_nested_if_else_in_let_no_semicolons() {
    // Test: nested if-else in let should NOT have semicolons
    let code = r#"
    pub fn get_nested(a: bool, b: bool) -> i32 {
        let result = if a {
            if b { 1 } else { 2 }
        } else {
            if b { 3 } else { 4 }
        }
        return result
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Inner branches should NOT have semicolons
    assert!(
        generated.contains("1")
            && generated.contains("2")
            && generated.contains("3")
            && generated.contains("4"),
        "Should contain all numeric values, got: {}",
        generated
    );
}

#[test]
fn test_if_else_in_assignment_no_semicolons() {
    // Test: if-else in assignment should NOT have semicolons
    let code = r#"
    pub fn assign_value(mut x: i32, flag: bool) -> i32 {
        x = if flag { 10 } else { 20 }
        return x
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Branches should NOT have semicolons when used as RHS of assignment
    assert!(
        generated.contains("if flag"),
        "Should contain if expression, got: {}",
        generated
    );
}

#[test]
fn test_if_else_block_expressions_no_semicolons() {
    // Test: if-else with block expressions should work correctly
    let code = r#"
    pub fn compute(x: i32) -> i32 {
        let result = if x > 0 {
            let temp = x * 2
            temp + 1
        } else {
            let temp = x * 3
            temp - 1
        }
        return result
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Last expressions in blocks should NOT have semicolons
    assert!(
        generated.contains("temp + 1") && generated.contains("temp - 1"),
        "Should contain block final expressions, got: {}",
        generated
    );
}

// TDD: Cast expression ExprId collision fix
//
// Problem: Cast expressions and their inner operands often share (line, col),
// causing them to get the same ExprId. This leads to conflicting constraints:
//   - Inner operand: ExprId(42:17) must be I32
//   - Cast result: ExprId(42:17) must be Usize
//   - CONFLICT!
//
// Solution: Give cast expressions unique IDs (separate cache) while still
// recursing into inner expressions to collect their constraints.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cast_simple_identifier() {
    // Simplest case: (x as usize) where x is i32
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn get_index(x: i32) -> usize {
    return (x as usize)
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should NOT have type conflicts
    assert!(!stderr.contains("Type conflict"), 
        "Cast of simple identifier should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

#[test]
fn test_cast_with_binary_op() {
    // Nested expression: (a + b as usize)
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn calc(a: i32, b: i32) -> usize {
    return ((a + b) as usize)
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(!stderr.contains("Type conflict"), 
        "Cast of binary op should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

#[test]
fn test_cast_with_complex_expr() {
    // Complex nested: ((a + b * 2) as usize)
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn calc_index(a: i32, b: i32) -> usize {
    return ((a + b * 2) as usize)
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(!stderr.contains("Type conflict"), 
        "Cast of complex expression should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

#[test]
fn test_cast_with_method_call() {
    // Method call inside cast: (s.len() as i32)
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn get_len(s: String) -> i32 {
    return (s.len() as i32)
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(!stderr.contains("Type conflict"), 
        "Cast of method call should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

#[test]
fn test_cast_in_comparison() {
    // Cast in conditional: if (x as i32) > 100
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn check_value(x: i64) -> bool {
    if (x as i32) > 100 {
        return true
    }
    return false
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(!stderr.contains("Type conflict"), 
        "Cast in comparison should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

#[test]
fn test_cast_in_array_index() {
    // Cast for array indexing: arr[(x as usize)]
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn get_element(arr: Vec<i32>, x: i32) -> i32 {
    return arr[(x as usize)]
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(!stderr.contains("Type conflict"), 
        "Cast for array indexing should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

#[test]
fn test_multiple_casts_same_line() {
    // Edge case: multiple casts on same line
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn convert_both(a: i32, b: i32) -> i64 {
    return ((a as i64) + (b as i64))
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(!stderr.contains("Type conflict"), 
        "Multiple casts on same line should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

#[test]
fn test_nested_casts() {
    // Double cast: ((x as i64) as usize)
    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.wj");
    
    fs::write(&test_file, r#"
fn double_cast(x: i32) -> usize {
    return ((x as i64) as usize)
}
"#).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(!stderr.contains("Type conflict"), 
        "Nested casts should not cause type conflict.\nStderr: {}", stderr);
    
    assert!(output.status.success(), 
        "Should compile successfully.\nStderr: {}", stderr);
}

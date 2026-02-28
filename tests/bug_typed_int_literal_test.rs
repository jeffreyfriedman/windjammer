// TDD Test for Bug: Typed integer literals generating stray type statements
// 
// Bug: When using typed integer literals like `0u64`, the compiler
// incorrectly generates a stray type statement:
//   Source:    let mut total = 0u64
//   Generated: let mut total = 0;
//              u64;  <-- STRAY STATEMENT!
//
// This causes E0423: expected value, found builtin type `u64`

use std::process::Command;
use std::fs;

fn compile_wj_test(source: &str) -> (bool, String, String) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("wj_test_{}", timestamp));
    fs::create_dir_all(&temp_dir).unwrap();
    
    let source_file = temp_dir.join("test.wj");
    fs::write(&source_file, source).unwrap();
    
    let output_dir = temp_dir.join("out");
    
    let output = Command::new("wj")
        .args(&["build", source_file.to_str().unwrap()])
        .args(&["--output", output_dir.to_str().unwrap()])
        .args(&["--target", "rust"])
        .args(&["--no-cargo"])
        .output()
        .expect("Failed to run wj");
    
    let _success = output.status.success();
    
    // Read generated Rust code
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_else(|_| String::from("(file not generated)"));
    
    // Try to compile with rustc to check for errors
    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rust_file)
        .arg("--out-dir")
        .arg(&temp_dir)
        .output()
        .expect("Failed to run rustc");
    
    let stderr = String::from_utf8_lossy(&rustc_output.stderr).to_string();
    
    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
    
    (rustc_output.status.success(), rust_code, stderr)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_u64_typed_literal_no_stray_statement() {
    let source = r#"
fn count_items() -> u64 {
    let mut total = 0u64
    for i in 0..10 {
        total = total + i
    }
    return total
}

fn main() {
    let result = count_items()
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Should not have E0423 error (stray type statement)
    assert!(!stderr.contains("E0423"), 
            "Should not have E0423 error:\n{}", stderr);
    assert!(!stderr.contains("expected value, found builtin type"), 
            "Should not have 'expected value' error:\n{}", stderr);
    
    // Check generated code does NOT have stray u64; statement
    assert!(!rust_code.contains("u64;") && !rust_code.contains("u64 ;"), 
            "Generated code should not contain stray 'u64;' statement:\n{}", rust_code);
    
    // TDD: Windjammer infers types, so `let mut total = 0;` is valid.
    // The return type `u64` propagates through type inference.
    // We just need to ensure NO stray type statements exist (checked above).
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_i32_typed_literal_no_stray_statement() {
    let source = r#"
fn calculate() -> i32 {
    let mut sum = 0i32
    sum = sum + 5
    return sum
}

fn main() {
    let _result = calculate()
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Check generated code does NOT have stray i32; statement
    assert!(!rust_code.contains("i32;") && !rust_code.contains("i32 ;"), 
            "Generated code should not contain stray 'i32;' statement:\n{}", rust_code);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_u32_typed_literal_no_stray_statement() {
    let source = r#"
fn get_count() -> u32 {
    let mut count = 0u32
    count = count + 1
    return count
}

fn main() {
    let _x = get_count()
}
"#;

    let (success, rust_code, stderr) = compile_wj_test(source);
    
    if !success {
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }
    
    // Check generated code does NOT have stray u32; statement
    assert!(!rust_code.contains("u32;") && !rust_code.contains("u32 ;"), 
            "Generated code should not contain stray 'u32;' statement:\n{}", rust_code);
}

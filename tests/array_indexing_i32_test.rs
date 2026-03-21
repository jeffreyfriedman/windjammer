// TDD Test: Array indexing with i32 should auto-cast to usize
//
// Bug: Windjammer allows `array[i32_index]` but generates invalid Rust
//
// Windjammer code:
//   let items = [1, 2, 3]
//   let i = 0  // i32 in Windjammer
//   let item = items[i]  // Should work!
//
// Generated Rust (BEFORE FIX):
//   let items = [1, 2, 3];
//   let i: i32 = 0;
//   let item = items[i];  // ERROR: can't index with i32
//
// Expected Rust (AFTER FIX):
//   let item = items[i as usize];  // Auto-cast i32 → usize

use std::fs;
use std::process::Command;

#[test]
fn test_array_indexing_with_i32() {
    let test_wj = r#"
fn test_array_index() {
    let items = [10, 20, 30, 40, 50]
    let i = 2  // i32 by default in Windjammer
    let value = items[i]  // Should auto-cast i32 → usize
    println!("Value: {}", value)
}
"#;
    
    let test_file = "/tmp/test_array_index.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    // Transpile
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    // Read generated Rust
    let rs_file = "./build/test_array_index.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Verify auto-cast is generated
    assert!(rust_code.contains("i as usize") || rust_code.contains("(i as usize)"),
        "Should generate auto-cast: items[i as usize]\nGenerated:\n{}", rust_code);
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Array indexing with i32 test PASSED");
}

#[test]
fn test_array_indexing_with_loop_variable() {
    // Common case: for i in range
    let test_wj = r#"
fn process_items() {
    let items = [1, 2, 3, 4, 5]
    for i in 0..items.len() {
        let value = items[i]  // i is i32, should auto-cast
        println!("{}", value)
    }
}
"#;
    
    let test_file = "/tmp/test_loop_index.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_loop_index.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // TDD FIX: After range fix, i is inferred as usize from 0_usize..len()
    // So items[i] doesn't need a cast! This is CORRECT and more efficient.
    // Verify i is usize in the range
    assert!(rust_code.contains("0_usize..items.len()") || rust_code.contains("for i in 0..items.len()"),
        "Loop should use usize range\nGenerated:\n{}", rust_code);
    
    // Verify array indexing works (no cast needed since i is already usize)
    assert!(rust_code.contains("items[i]"),
        "Should index array with i\nGenerated:\n{}", rust_code);
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ Loop variable indexing test PASSED");
}

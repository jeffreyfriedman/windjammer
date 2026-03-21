// TDD Test: Literal comparisons with .len() should use usize
//
// Bug: if vec.len() > 0 generates len() > 0_i32 → type error
// Rust: usize > i32 = type mismatch
//
// Fix: Infer literals as usize when compared with .len()

use std::fs;
use std::process::Command;

#[test]
fn test_len_comparison_with_zero() {
    let test_wj = r#"
fn has_items(items: Vec<i32>) -> bool {
    if items.len() > 0 {
        return true
    }
    false
}
"#;
    
    let test_file = "/tmp/test_len_zero.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_len_zero.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should generate 0_usize or just 0 (Rust infers from context)
    // Should NOT generate 0_i32
    assert!(
        !rust_code.contains("0_i32"),
        "Should NOT generate i32 literal in len() comparison\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ len() > 0 test PASSED");
}

#[test]
fn test_len_comparison_with_constant() {
    let test_wj = r#"
fn is_valid_team(team: Vec<String>) -> bool {
    team.len() >= 2
}
"#;
    
    let test_file = "/tmp/test_len_const.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_len_const.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Should NOT generate i32 literals
    assert!(
        !rust_code.contains("2_i32"),
        "Should NOT generate i32 literal in len() comparison\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ len() >= 2 test PASSED");
}

#[test]
fn test_len_assignment_to_usize() {
    let test_wj = r#"
struct Animation {
    current_frame_index: usize
}

impl Animation {
    fn reset(self) {
        self.current_frame_index = 0
    }
}
"#;
    
    let test_file = "/tmp/test_len_assign.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_len_assign.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // When assigning to usize field, literal should be usize
    assert!(
        rust_code.contains("0_usize") || !rust_code.contains("0_i32"),
        "Should generate usize literal for usize field\nGenerated:\n{}", 
        rust_code
    );
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    
    println!("✅ usize field assignment test PASSED");
}

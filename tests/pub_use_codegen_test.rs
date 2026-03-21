// TDD Test: Windjammer Compiler Bug - pub use relative paths incorrectly transpiled
//
// Bug: Relative `pub use` statements in mod.wj are being transpiled to incorrect absolute paths
//
// Source (.wj):     pub use achievement_id::AchievementId
// Generated (.rs):  pub use crate::achievement_id::AchievementId  // ❌ WRONG!
// Expected (.rs):   pub use achievement_id::AchievementId          // ✅ CORRECT
//
// This causes "unresolved import" errors because the submodules are in the same directory,
// not at crate root.

use std::fs;
use std::process::Command;

#[test]
fn test_pub_use_relative_paths() {
    // Create a test module with submodules
    let test_wj = r#"
// Test module with relative pub use statements
pub mod sub_module_a
pub mod sub_module_b

// These should remain RELATIVE paths in generated Rust
pub use sub_module_a::TypeA
pub use sub_module_b::TypeB
"#;
    
    let test_file = "/tmp/test_pub_use_module.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    // Transpile
    let output = Command::new("./target/release/wj")
        .args(&["build", "--no-cargo", test_file])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        panic!("Compilation failed");
    }
    
    // Read generated Rust
    let rs_file = "/tmp/build/test_pub_use_module.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // Verify pub use statements are RELATIVE (not crate::)
    assert!(rust_code.contains("pub use sub_module_a::TypeA"),
        "Should generate: pub use sub_module_a::TypeA (relative path)");
    
    assert!(rust_code.contains("pub use sub_module_b::TypeB"),
        "Should generate: pub use sub_module_b::TypeB (relative path)");
    
    // Should NOT have crate:: prefix for relative imports
    assert!(!rust_code.contains("pub use crate::sub_module_a"),
        "Should NOT generate: pub use crate::sub_module_a (wrong absolute path)");
    
    assert!(!rust_code.contains("pub use crate::sub_module_b"),
        "Should NOT generate: pub use crate::sub_module_b (wrong absolute path)");
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    let _ = fs::remove_dir_all("/tmp/build");
    
    println!("✅ pub use relative path test PASSED");
}

#[test]
fn test_pub_use_absolute_paths_unchanged() {
    // Verify that ABSOLUTE paths (starting with crate::) remain absolute
    let test_wj = r#"
// These are already absolute and should stay that way
pub use crate::some_module::TypeA
pub use crate::another::TypeB
"#;
    
    let test_file = "/tmp/test_pub_use_absolute.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", "--no-cargo", test_file])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        panic!("Compilation failed");
    }
    
    let rs_file = "/tmp/build/test_pub_use_absolute.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    // Absolute paths should remain unchanged
    assert!(rust_code.contains("pub use crate::some_module::TypeA"),
        "Absolute paths should remain: pub use crate::some_module::TypeA");
    
    assert!(rust_code.contains("pub use crate::another::TypeB"),
        "Absolute paths should remain: pub use crate::another::TypeB");
    
    // Cleanup
    let _ = fs::remove_file(test_file);
    let _ = fs::remove_dir_all("/tmp/build");
    
    println!("✅ pub use absolute path test PASSED");
}

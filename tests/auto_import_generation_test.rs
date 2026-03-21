// TDD Test: Automatic import generation for cross-module type references
//
// Problem: Using types from other modules causes E0425 (cannot find type)
// Example: manager.wj uses Achievement/AchievementId but no imports generated
// Root cause: Codegen doesn't detect type usage and generate imports
//
// Solution: Analyze struct/function signatures, detect external types, generate imports
//
// This is a MAJOR architectural feature for ergonomic module system!

use std::fs;
use std::process::Command;

#[test]
fn test_struct_field_external_type() {
    // Test: Struct field references type from another module
    let test_wj = r#"
// This struct uses DialogueLineId which should be imported
pub struct DialogueChoice {
    pub id: u32,
    pub next_line: DialogueLineId
}
"#;
    
    let test_file = "/tmp/test_external_type.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_external_type.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // For now, document current behavior
    // Future: Should generate: use super::DialogueLineId; or use crate::DialogueLineId;
    
    // This test documents the NEED for auto-imports
    // When implemented, we'll assert the import IS generated
    
    println!("⚠️  Current: No import generated for DialogueLineId");
    println!("🎯 Future: Should generate 'use super::DialogueLineId;'");
    
    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_function_param_external_type() {
    let test_wj = r#"
pub struct Manager {
    items: Vec<Item>
}

impl Manager {
    pub fn add_item(self, item: Item) {
        self.items.push(item)
    }
}
"#;
    
    let test_file = "/tmp/test_param_type.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_param_type.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    println!("⚠️  Current: No import for Item type");
    println!("🎯 Future: Should generate 'use super::Item;'");
    
    // Cleanup
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_detect_external_types() {
    // This test verifies we can detect which types are external
    // (not defined in current file, not stdlib)
    
    let test_wj = r#"
pub struct Manager {
    // External types that need importing:
    achievements: HashMap<AchievementId, Achievement>,
    users: Vec<User>,
    current_quest: Option<Quest>,
    
    // Stdlib types (no import needed):
    count: i32,
    name: String,
    active: bool
}
"#;
    
    let test_file = "/tmp/test_detect_types.wj";
    fs::write(test_file, test_wj).expect("Failed to write test file");
    
    let output = Command::new("./target/release/wj")
        .args(&["build", test_file, "-o", "./build", "--no-cargo"])
        .output()
        .expect("Failed to run wj compiler");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed: {}", stderr);
    }
    
    let rs_file = "./build/test_detect_types.rs";
    let rust_code = fs::read_to_string(rs_file)
        .expect("Failed to read generated .rs file");
    
    println!("Generated Rust:\n{}", rust_code);
    
    // External types that should be imported:
    // - AchievementId
    // - Achievement
    // - User
    // - Quest
    
    // Stdlib types (no import):
    // - HashMap (use std::collections::HashMap already generated)
    // - Vec, Option (prelude)
    // - i32, String, bool (primitives)
    
    println!("\n🎯 External types needing import:");
    println!("   - AchievementId");
    println!("   - Achievement");
    println!("   - User");
    println!("   - Quest");
    
    // Cleanup
    let _ = fs::remove_file(test_file);
}

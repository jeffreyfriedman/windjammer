/// TDD Test: Forward Reference Limitation
/// 
/// This test documents the current limitation of String → &str auto-conversion
/// when methods are defined after they are used.
/// 
/// **Current Behavior**: Fails when Inventory is defined after DialogCondition
/// **Expected Behavior** (after two-pass analysis): Should work regardless of order
/// 
/// **Status**: KNOWN LIMITATION - tracked for future enhancement

#[test]
#[ignore] // Ignore until two-pass analysis is implemented
fn test_forward_reference_limitation() {
    use std::fs;
    use std::process::Command;
    use std::path::Path;
    
    let test_code = r#"
// ❌ CURRENT: This fails because Inventory is defined AFTER DialogCondition
enum DialogCondition {
    HasItem(string, i32),
}

impl DialogCondition {
    pub fn evaluate(self, gs: GameState) -> bool {
        match self {
            DialogCondition::HasItem(item_id, qty) => {
                gs.inventory.has_item(item_id, qty)
            },
        }
    }
}

struct GameState {
    inventory: Inventory,
}

struct Inventory {
}

impl Inventory {
    pub fn has_item(self, item_id: string, min_qty: i32) -> bool {
        false
    }
}
"#;

    let temp_dir = std::env::temp_dir().join("wj_forward_ref_test");
    fs::create_dir_all(&temp_dir).unwrap();
    let test_file = temp_dir.join("forward_ref_test.wj");
    fs::write(&test_file, test_code).unwrap();

    // Run wj build
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(), "wj build should succeed");

    // Check generated code
    let generated_rs = temp_dir.join("build/forward_ref_test.rs");
    assert!(generated_rs.exists(), "Generated Rust file should exist");

    let generated_code = fs::read_to_string(&generated_rs).unwrap();

    // ✅ EXPECTED (after two-pass analysis): Should add & to item_id
    assert!(
        generated_code.contains("gs.inventory.has_item(&item_id, qty)"),
        "Should auto-add & to convert String → &str (requires two-pass analysis)"
    );

    // Clean up
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_forward_reference_workaround() {
    use std::fs;
    use std::process::Command;
    use std::path::Path;
    
    let test_code = r#"
// ✅ WORKAROUND: Define Inventory BEFORE DialogCondition
struct Inventory {
}

impl Inventory {
    pub fn has_item(self, item_id: string, min_qty: i32) -> bool {
        false
    }
}

struct GameState {
    inventory: Inventory,
}

enum DialogCondition {
    HasItem(string, i32),
}

impl DialogCondition {
    pub fn evaluate(self, gs: GameState) -> bool {
        match self {
            DialogCondition::HasItem(item_id, qty) => {
                gs.inventory.has_item(item_id, qty)
            },
        }
    }
}
"#;

    let temp_dir = std::env::temp_dir().join("wj_forward_ref_workaround_test");
    fs::create_dir_all(&temp_dir).unwrap();
    let test_file = temp_dir.join("workaround_test.wj");
    fs::write(&test_file, test_code).unwrap();

    // Run wj build
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(), "wj build should succeed");

    // Check generated code
    let generated_rs = temp_dir.join("build/workaround_test.rs");
    assert!(generated_rs.exists(), "Generated Rust file should exist");

    let generated_code = fs::read_to_string(&generated_rs).unwrap();

    // ✅ This SHOULD work with current implementation
    assert!(
        generated_code.contains("gs.inventory.has_item(&item_id, qty)"),
        "Should auto-add & when Inventory is defined first"
    );

    // Clean up
    fs::remove_dir_all(&temp_dir).ok();
}

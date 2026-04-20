/// TDD Test: Method Signature Lookup Integration
///
/// Tests the complete flow of looking up method signatures by receiver type
/// and using them to make proper parameter conversion decisions.
///
/// This validates that we can replace ALL hard-coded heuristics with type-based logic.
use std::fs;
use std::path::PathBuf;

fn compile_to_rust(wj_code: &str) -> String {
    let temp_dir = tempfile::tempdir().unwrap();
    let wj_file = temp_dir.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    let compiler_dir = std::env::current_dir().unwrap();
    let compiler = compiler_dir.join("target/release/wj");

    let output = std::process::Command::new(&compiler)
        .arg("build")
        .arg(&wj_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("wj build failed:\n{}", stderr);
    }

    let rs_file = temp_dir.path().join("build/test.rs");
    fs::read_to_string(rs_file).expect("Failed to read generated Rust")
}

fn compile_and_check_rust(wj_code: &str) -> Result<String, String> {
    let rust_code = compile_to_rust(wj_code);

    let temp_dir = tempfile::tempdir().unwrap();
    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, &rust_code).unwrap();

    let rustc_output = std::process::Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rs_file)
        .arg("--out-dir")
        .arg(temp_dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&rustc_output.stderr).to_string();

    if rustc_output.status.success() {
        Ok(rust_code)
    } else {
        Err(stderr)
    }
}

// =============================================================================
// STDLIB METHOD SIGNATURE TESTS
// =============================================================================

#[test]
fn test_vec_push_uses_signature_not_heuristic() {
    // This test validates that Vec::push correctly accepts owned String
    // based on its SIGNATURE (param_type: T, ownership: Owned)
    // NOT based on hard-coded "push" method name matching

    let code = r#"
pub fn add_items(items: Vec<string>, item: string) {
    items.push(item)  // Should NOT add & (signature says owned T)
}

pub fn main() {
    let items = Vec::new()
    add_items(items, "test")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "Vec::push should work with owned String:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    // Should NOT have &item (Vec::push wants owned T)
    assert!(
        rust_code.contains("items.push(item)") || rust_code.contains(".push(item)"),
        "Vec::push should not add & for owned parameter:\n{}",
        rust_code
    );
}

#[test]
fn test_vec_contains_uses_signature_not_heuristic() {
    // This test validates that Vec::contains correctly borrows the argument
    // based on its SIGNATURE (param_type: &T, ownership: Borrowed)
    // NOT based on hard-coded "contains" method name matching

    let code = r#"
pub fn check_item(items: Vec<string>, item: string) -> bool {
    items.contains(item)  // Should add & (signature says &T)
}

pub fn main() {
    let items = Vec::new()
    let has_it = check_item(items, "test")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "Vec::contains should work with borrowed String:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    // Should have &item (Vec::contains wants &T)
    assert!(
        rust_code.contains("&item") || rust_code.contains(".contains(&"),
        "Vec::contains should add & for borrowed parameter:\n{}",
        rust_code
    );
}

#[test]
fn test_string_contains_uses_signature() {
    let code = r#"
pub fn has_substring(text: string, pattern: string) -> bool {
    text.contains(pattern)  // Should add & (String::contains wants &str)
}

pub fn main() {
    let result = has_substring("hello", "ell")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "String::contains should work:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("&pattern") || rust_code.contains(".contains(&"),
        "String::contains should add & for &str parameter:\n{}",
        rust_code
    );
}

#[test]
fn test_hashmap_get_uses_signature() {
    let code = r#"
pub fn lookup(map: HashMap<string, i32>, key: string) -> Option<i32> {
    map.get(key)  // Should add & (HashMap::get wants &K)
}

pub fn main() {
    let map = HashMap::new()
    let val = lookup(map, "test")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "HashMap::get should work:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    assert!(
        rust_code.contains("&key") || rust_code.contains(".get(&"),
        "HashMap::get should add & for &K parameter:\n{}",
        rust_code
    );
}

#[test]
fn test_hashmap_insert_uses_signature() {
    let code = r#"
pub fn add_entry(map: HashMap<string, i32>, key: string, value: i32) {
    map.insert(key, value)  // Should NOT add & (HashMap::insert wants owned K, V)
}

pub fn main() {
    let map = HashMap::new()
    add_entry(map, "test", 42)
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "HashMap::insert should work:\n{:?}",
        result.err()
    );

    let rust_code = result.unwrap();
    // Should NOT have &key or &value (HashMap::insert wants owned)
    assert!(
        rust_code.contains("map.insert(key, value)") || rust_code.contains(".insert(key, value)"),
        "HashMap::insert should not add & for owned parameters:\n{}",
        rust_code
    );
}

// =============================================================================
// USER-DEFINED METHOD SIGNATURE TESTS
// =============================================================================

#[test]
fn test_user_method_signature_lookup() {
    // This test validates that user-defined methods are looked up by receiver type
    // Example: Inventory::has_item should use its ACTUAL signature, not guess based on method name

    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        // Method signature: has_item(&self, item_id: &str, qty: i32) -> bool
        // (after ownership inference: self → &self, item_id → &str)
        self.items.contains(item_id)
    }
}

pub fn check_inventory(inv: Inventory, item: string) -> bool {
    inv.has_item(item, 1)  // Should add & to item (signature says &str)
}

pub fn main() {
    let inv = Inventory { items: Vec::new() }
    let result = check_inventory(inv, "sword")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "User-defined method should work with signature lookup:\n{:?}",
        result.err()
    );

    // The key point: We should NOT need game-specific "has_item" heuristics
    // The signature lookup should handle this generically
}

#[test]
fn test_no_hardcoded_method_names_needed() {
    // This test uses a completely arbitrary method name that is NOT in any heuristic list
    // It should still work correctly based on the actual signature

    let code = r#"
pub struct Checker {
    pub value: string,
}

impl Checker {
    // Completely arbitrary method name - NOT "has_item", NOT "get_attribute", etc.
    pub fn arbitrary_check_method_xyz(self, input: string) -> bool {
        self.value == input
    }
}

pub fn test_checker(checker: Checker, text: string) -> bool {
    checker.arbitrary_check_method_xyz(text)  // Should add & (signature says &str)
}

pub fn main() {
    let c = Checker { value: "test" }
    let result = test_checker(c, "test")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "Arbitrary method name should work without hardcoded heuristics:\n{:?}",
        result.err()
    );
}

// =============================================================================
// FIELD-CHAIN METHOD SIGNATURE TESTS
// =============================================================================

#[test]
#[ignore] // Enable when Phase 3 (field-chain resolution) is implemented
fn test_field_chain_method_lookup() {
    // This test validates field-chain type resolution: game_state.inventory.has_item
    // Should resolve: game_state: GameState → inventory: Inventory → has_item signature

    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string) -> bool {
        self.items.contains(item_id)
    }
}

pub struct GameState {
    pub inventory: Inventory,
}

pub fn check_item(game_state: GameState, item: string) -> bool {
    game_state.inventory.has_item(item)  // Should resolve Inventory::has_item signature
}

pub fn main() {
    let state = GameState { inventory: Inventory { items: Vec::new() } }
    let result = check_item(state, "sword")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "Field-chain method lookup should work:\n{:?}",
        result.err()
    );
}

// =============================================================================
// MATCH ARM BINDING TESTS
// =============================================================================

#[test]
#[ignore] // Enable when match arm type tracking is complete
fn test_match_arm_binding_with_signature_lookup() {
    // This test validates that match arm bindings work with signature lookup
    // Example: DialogCondition::HasItem(item_id, qty) → item_id should be String
    // When passed to has_item, should add & based on Inventory::has_item signature

    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        self.items.contains(item_id)
    }
}

pub enum Condition {
    HasItem(string, i32),
}

impl Condition {
    pub fn evaluate(self, inv: Inventory) -> bool {
        match self {
            Condition::HasItem(item_id, qty) => {
                inv.has_item(item_id, qty)  // Should add & to item_id based on signature
            }
        }
    }
}

pub fn main() {
    let inv = Inventory { items: Vec::new() }
    let cond = Condition::HasItem("sword", 1)
    let result = cond.evaluate(inv)
}
"#;

    let result = compile_and_check_rust(code);
    assert!(
        result.is_ok(),
        "Match arm bindings should work with signature lookup:\n{:?}",
        result.err()
    );
}

/// TDD Test: Field-chain method signature resolution
///
/// Tests that game_state.inventory.has_item(item_id) correctly resolves:
/// 1. game_state → GameState type
/// 2. inventory field → Inventory type
/// 3. has_item method → Inventory::has_item signature
/// 4. item_id parameter → adds & for String → &str conversion
///
/// This validates Phase 3: Field-chain type resolution works end-to-end

use std::fs;

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

#[test]
fn test_field_chain_single_level() {
    // Test: obj.inventory.has_item(id) where inventory is a field of obj
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

pub fn check_item(state: GameState, id: string) -> bool {
    state.inventory.has_item(id)  // Should add & to id
}

pub fn main() {
    let state = GameState { inventory: Inventory { items: Vec::new() } }
    let result = check_item(state, "sword")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), 
        "Field-chain method signature resolution should work:\n{:?}", 
        result.err());
}

#[test]
fn test_match_arm_with_field_chain() {
    // Test: Match arm binding used with field-chain method call
    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string, qty: i32) -> bool {
        self.items.contains(item_id)
    }
}

pub struct GameState {
    pub inventory: Inventory,
}

pub enum Condition {
    HasItem(string, i32),
}

impl Condition {
    pub fn evaluate(self, state: GameState) -> bool {
        match self {
            Condition::HasItem(item_id, qty) => {
                state.inventory.has_item(item_id, qty)  // Should add & to item_id
            }
        }
    }
}

pub fn main() {
    let state = GameState { inventory: Inventory { items: Vec::new() } }
    let cond = Condition::HasItem("sword", 1)
    let result = cond.evaluate(state)
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), 
        "Match arm with field-chain should work:\n{:?}", 
        result.err());
}

#[test]
fn test_deep_field_chain() {
    // Test: Deeper nesting (a.b.c.method(x))
    let code = r#"
pub struct Player {
    pub name: string,
}

impl Player {
    pub fn has_name(self, n: string) -> bool {
        self.name == n
    }
}

pub struct GameData {
    pub player: Player,
}

pub struct GameState {
    pub data: GameData,
}

pub fn check_player(state: GameState, name: string) -> bool {
    state.data.player.has_name(name)  // Should add & to name
}

pub fn main() {
    let state = GameState { 
        data: GameData { 
            player: Player { name: "Alice" } 
        } 
    }
    let result = check_player(state, "Alice")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), 
        "Deep field-chain should work:\n{:?}", 
        result.err());
}

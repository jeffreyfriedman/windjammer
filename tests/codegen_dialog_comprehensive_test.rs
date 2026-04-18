/// TDD Test: Comprehensive dialog.wj pattern reproduction
/// 
/// This test reproduces ALL the compilation errors found in dialog.wj:
/// 1. E0308: String vs &str in match arm bindings (4 instances)
/// 2. E0614: Cannot dereference primitive in match pattern
/// 3. E0507: Ownership inference for methods returning Option
/// 4. E0594: Mutability inference for mutable references in loops

use std::path::PathBuf;
use std::fs;

fn get_compiler_path() -> PathBuf {
    // First try to find wj in the same directory as this test binary
    let exe = std::env::current_exe().unwrap();
    let test_dir = exe.parent().unwrap();
    
    // Try release binary
    let wj_path = test_dir.parent().unwrap().parent().unwrap().join("wj");
    if wj_path.exists() {
        return wj_path;
    }
    
    // Try debug binary  
    let wj_debug = test_dir.parent().unwrap().parent().unwrap().parent().unwrap()
        .join("debug/wj");
    if wj_debug.exists() {
        return wj_debug;
    }
    
    // Fallback to PATH
    PathBuf::from("wj")
}

fn compile_and_check(wj_code: &str, should_succeed: bool) -> (String, String) {
    let temp_dir = tempfile::tempdir().unwrap();
    let wj_file = temp_dir.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();
    
    let compiler = get_compiler_path();
    let output = std::process::Command::new(compiler)
        .arg("build")
        .arg(&wj_file)
        .current_dir(temp_dir.path())
        .output()
        .unwrap();
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    // Try to compile the generated Rust
    let build_dir = temp_dir.path().join("build");
    if build_dir.exists() {
        let cargo_output = std::process::Command::new("cargo")
            .arg("build")
            .current_dir(&build_dir)
            .output()
            .unwrap();
        
        let cargo_stderr = String::from_utf8_lossy(&cargo_output.stderr).to_string();
        
        if should_succeed {
            if !cargo_output.status.success() {
                panic!("Expected successful Rust compilation but got errors:\n{}", cargo_stderr);
            }
        } else {
            if cargo_output.status.success() {
                panic!("Expected Rust compilation errors but it succeeded!");
            }
        }
        
        (stdout + &stderr, cargo_stderr)
    } else {
        panic!("Build directory not created");
    }
}

#[test]
fn test_dialog_pattern_string_to_str_match_arms() {
    // Issue 1: E0308 - String in match arm binding passed to &str parameter
    let code = r#"
pub struct Inventory {
    items: Vec<(string, i32)>,
}

impl Inventory {
    pub fn has_item(self, item_id: string, min_qty: i32) -> bool {
        for (id, qty) in self.items {
            if id == item_id && qty >= min_qty {
                return true
            }
        }
        false
    }
}

pub enum Condition {
    HasItem(string, i32),
}

impl Condition {
    pub fn evaluate(self, inv: Inventory) -> bool {
        match self {
            Condition::HasItem(item_id, qty) => {
                inv.has_item(item_id, qty)  // Should auto-convert String → &str
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

    let (_, cargo_stderr) = compile_and_check(code, true);
    
    // Verify no E0308 errors
    assert!(!cargo_stderr.contains("error[E0308]"), 
        "Should not have E0308 String vs &str error:\n{}", cargo_stderr);
}

#[test]
fn test_dialog_pattern_primitive_match_deref() {
    // Issue 2: E0614 - Compiler incorrectly adds * for primitives in match patterns
    let code = r#"
pub struct Player {
    pub gold: i32,
}

pub enum Cost {
    Gold(i32),
}

impl Cost {
    pub fn can_afford(self, player: Player) -> bool {
        match self {
            Cost::Gold(amount) => {
                player.gold >= amount  // Should NOT deref amount
            }
        }
    }
}

pub fn main() {
    let player = Player { gold: 100 }
    let cost = Cost::Gold(50)
    let can = cost.can_afford(player)
}
"#;

    let (_, cargo_stderr) = compile_and_check(code, true);
    
    // Verify no E0614 errors
    assert!(!cargo_stderr.contains("error[E0614]"), 
        "Should not have E0614 cannot dereference error:\n{}", cargo_stderr);
}

#[test]
fn test_dialog_pattern_option_return_ownership() {
    // Issue 3: E0507 - Method returning Option should infer correct self ownership
    let code = r#"
pub struct Node {
    pub id: string,
}

pub struct Tree {
    nodes: Vec<Node>,
    current_id: string,
}

impl Tree {
    pub fn get_current_node(self) -> Option<Node> {
        for node in self.nodes {
            if node.id == self.current_id {
                return Some(node)
            }
        }
        None
    }
    
    pub fn process(self) {
        if let Some(node) = self.get_current_node() {
            println!("{}", node.id)
        }
    }
}

pub fn main() {
    let tree = Tree {
        nodes: Vec::new(),
        current_id: "start",
    }
    tree.process()
}
"#;

    let (_, cargo_stderr) = compile_and_check(code, true);
    
    // Verify no E0507 errors
    assert!(!cargo_stderr.contains("error[E0507]"), 
        "Should not have E0507 cannot move error:\n{}", cargo_stderr);
}

#[test]
fn test_dialog_pattern_mutable_tuple_elements() {
    // Issue 4: E0594 - Tuple elements in loops should infer &mut when assigned
    let code = r#"
pub struct Inventory {
    items: Vec<(string, i32)>,
}

impl Inventory {
    pub fn add_item(self, item_id: string, quantity: i32) {
        for (id, qty) in self.items {
            if id == item_id {
                qty = qty + quantity  // Should infer &mut qty
                return
            }
        }
        self.items.push((item_id, quantity))
    }
}

pub fn main() {
    let mut inv = Inventory { items: Vec::new() }
    inv.add_item("sword", 1)
}
"#;

    let (_, cargo_stderr) = compile_and_check(code, true);
    
    // Verify no E0594 errors
    assert!(!cargo_stderr.contains("error[E0594]"), 
        "Should not have E0594 cannot assign error:\n{}", cargo_stderr);
}

#[test]
fn test_dialog_full_integration() {
    // Integration test: All patterns together
    let code = r#"
pub struct Player {
    pub gold: i32,
    attributes: Vec<(string, i32)>,
}

impl Player {
    pub fn get_attribute(self, name: string) -> i32 {
        for (attr, val) in self.attributes {
            if attr == name {
                return val
            }
        }
        0
    }
    
    pub fn set_attribute(self, name: string, value: i32) {
        for (attr, val) in self.attributes {
            if attr == name {
                val = value
                return
            }
        }
        self.attributes.push((name, value))
    }
}

pub struct GameState {
    pub player: Player,
}

pub enum Condition {
    AttributeCheck(string, i32),
    HasGold(i32),
}

impl Condition {
    pub fn evaluate(self, state: GameState) -> bool {
        match self {
            Condition::AttributeCheck(attr, min) => {
                state.player.get_attribute(attr) >= min
            },
            Condition::HasGold(amount) => {
                state.player.gold >= amount
            },
        }
    }
}

pub fn main() {
    let player = Player {
        gold: 100,
        attributes: Vec::new(),
    }
    let state = GameState { player: player }
    let cond = Condition::HasGold(50)
    let result = cond.evaluate(state)
}
"#;

    let (_, cargo_stderr) = compile_and_check(code, true);
    
    // Verify no compilation errors
    assert!(!cargo_stderr.contains("error[E"), 
        "Should have no compilation errors:\n{}", cargo_stderr);
}

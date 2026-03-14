//! TDD: E0507/E0596 ownership inference for Option patterns and get_mut
//!
//! Fixes for breach-protocol:
//! - E0507: if let Some(x) = self.field with &self → generate &self.field
//! - E0507: self.field.map(...) with &self → generate self.field.as_ref().map(...)
//! - E0596: self.nodes.get_mut(id) → infer &mut self

use std::process::Command;

fn get_wj_binary() -> String {
    env!("CARGO_BIN_EXE_wj").to_string()
}

fn compile_to_rust(wj_source: &str) -> Result<String, String> {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    std::fs::write(&wj_path, wj_source).expect("Failed to write test file");
    std::fs::create_dir(&out_dir).expect("Failed to create output dir");

    let output = Command::new(get_wj_binary())
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let rust_file = out_dir.join("test.rs");
    Ok(std::fs::read_to_string(rust_file).expect("Failed to read generated Rust"))
}

#[test]
fn test_option_pattern_if_let_borrows_self_field() {
    // E0507 fix: if let Some(stack) = self.weapon with &self must use &self.weapon
    let source = r#"
pub struct ItemStack {
    pub item: ItemStats,
}

pub struct ItemStats {
    pub health: i32,
    pub damage: i32,
}

pub struct Equipment {
    pub weapon: Option<ItemStack>,
}

impl Equipment {
    pub fn get_total_damage(self) -> i32 {
        let mut total = 0
        if let Some(stack) = self.weapon {
            total = total + stack.item.damage
        }
        total
    }
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should generate &self.weapon to avoid E0507 "cannot move out of borrowed"
    assert!(
        rust_code.contains("&self.weapon"),
        "Option pattern on self field should use &self.weapon when &self. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_option_map_uses_as_ref_for_self_field() {
    // E0507 fix: self.children.map(|c| ...) with &self must use self.children.as_ref().map(...)
    let source = r#"
pub struct OctreeNode {
    pub value: u8,
    pub children: Option<Vec<OctreeNode>>,
}

impl OctreeNode {
    pub fn child_count(self) -> i32 {
        self.children.map(|c| c.len() as i32).unwrap_or(0)
    }
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should generate .as_ref().map(...) to avoid E0507
    assert!(
        rust_code.contains(".as_ref().map("),
        "Option::map on self field should use .as_ref().map() when &self. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_get_mut_infers_mut_self() {
    // E0596 fix: self.nodes.get_mut(id) requires &mut self
    let source = r#"
use std::collections::HashMap

pub struct SceneNode {
    pub id: u64,
    pub children: Vec<u64>,
}

pub struct SceneGraph {
    pub nodes: HashMap<u64, SceneNode>,
}

impl SceneGraph {
    pub fn attach_child(self, parent_id: u64, child_id: u64) {
        if self.nodes.contains_key(parent_id) {
            let mut parent = self.nodes.get_mut(parent_id).unwrap()
            if !parent.children.contains(child_id) {
                parent.children.push(child_id)
            }
        }
    }
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should infer &mut self for attach_child
    assert!(
        rust_code.contains("fn attach_child(&mut self"),
        "Method calling get_mut on self field should infer &mut self. Got:\n{}",
        rust_code
    );
}

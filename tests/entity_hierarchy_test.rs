/// TDD Test: Entity Hierarchy System
///
/// This test verifies that entities can have parent/child relationships:
/// - Set parent for an entity
/// - Get children of an entity
/// - Detach entity from parent
/// - Traverse hierarchy
use std::process::Command;
use tempfile::TempDir;

fn compile_hierarchy_test(code: &str) -> Result<String, String> {
    use std::fs;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let output_dir = temp_dir.path().to_path_buf();

    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, code).map_err(|e| format!("Failed to write test file: {}", e))?;

    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .map_err(|e| format!("Failed to execute compiler: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        ));
    }

    Ok("Compilation successful".to_string())
}

#[test]
fn test_entity_parent_child_relationship() {
    let code = r#"
use windjammer_game::prelude::*;

struct GameObject {
    pub name: string,
}

fn main() {
    let scene = Scene::new("TestScene".to_string());
    
    // Create parent entity
    let parent = GameObject { name: "Parent".to_string() };
    let parent_id = scene.add_entity(parent);
    
    // Create child entities
    let child1 = GameObject { name: "Child1".to_string() };
    let child2 = GameObject { name: "Child2".to_string() };
    
    let child1_id = scene.add_entity(child1);
    let child2_id = scene.add_entity(child2);
    
    // Set parent-child relationships
    scene.set_parent(child1_id, parent_id);
    scene.set_parent(child2_id, parent_id);
    
    // Get children of parent
    let children = scene.get_children(parent_id);
    println!("Parent has {} children", children.len());
    
    // Get parent of child
    let parent_of_child = scene.get_parent(child1_id);
    if parent_of_child.is_some() {
        println!("Child has a parent");
    }
}
"#;

    let result = compile_hierarchy_test(code);
    assert!(
        result.is_ok(),
        "Entity hierarchy should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_entity_detach_from_parent() {
    let code = r#"
use windjammer_game::prelude::*;

struct Node {
    pub value: i64,
}

fn main() {
    let scene = Scene::new("TestScene".to_string());
    
    let parent_node = Node { value: 1 };
    let child_node = Node { value: 2 };
    
    let parent_id = scene.add_entity(parent_node);
    let child_id = scene.add_entity(child_node);
    
    // Attach child to parent
    scene.set_parent(child_id, parent_id);
    
    // Verify parent is set
    let has_parent = scene.get_parent(child_id).is_some();
    println!("Has parent: {}", has_parent);
    
    // Detach child from parent
    scene.detach_from_parent(child_id);
    
    // Verify parent is removed
    let has_parent_after = scene.get_parent(child_id).is_some();
    println!("Has parent after detach: {}", has_parent_after);
}
"#;

    let result = compile_hierarchy_test(code);
    assert!(
        result.is_ok(),
        "Entity detachment should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_entity_hierarchy_traversal() {
    let code = r#"
use windjammer_game::prelude::*;

struct TreeNode {
    pub label: string,
}

fn main() {
    let scene = Scene::new("TestScene".to_string());
    
    // Create a tree structure:
    //     Root
    //     /  \
    //   A     B
    //  / \
    // C   D
    
    let root = scene.add_entity(TreeNode { label: "Root".to_string() });
    let a = scene.add_entity(TreeNode { label: "A".to_string() });
    let b = scene.add_entity(TreeNode { label: "B".to_string() });
    let c = scene.add_entity(TreeNode { label: "C".to_string() });
    let d = scene.add_entity(TreeNode { label: "D".to_string() });
    
    scene.set_parent(a, root);
    scene.set_parent(b, root);
    scene.set_parent(c, a);
    scene.set_parent(d, a);
    
    // Get all descendants of root (should be 4: A, B, C, D)
    let descendants = scene.get_descendants(root);
    println!("Root has {} descendants", descendants.len());
    
    // Get ancestors of C (should be 2: A, Root)
    let ancestors = scene.get_ancestors(c);
    println!("C has {} ancestors", ancestors.len());
}
"#;

    let result = compile_hierarchy_test(code);
    assert!(
        result.is_ok(),
        "Entity hierarchy traversal should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_entity_sibling_relationships() {
    let code = r#"
use windjammer_game::prelude::*;

struct Item {
    pub name: string,
}

fn main() {
    let scene = Scene::new("TestScene".to_string());
    
    let parent = scene.add_entity(Item { name: "Parent".to_string() });
    let child1 = scene.add_entity(Item { name: "Child1".to_string() });
    let child2 = scene.add_entity(Item { name: "Child2".to_string() });
    let child3 = scene.add_entity(Item { name: "Child3".to_string() });
    
    scene.set_parent(child1, parent);
    scene.set_parent(child2, parent);
    scene.set_parent(child3, parent);
    
    // Get siblings of child1 (should be child2 and child3)
    let siblings = scene.get_siblings(child1);
    println!("Child1 has {} siblings", siblings.len());
}
"#;

    let result = compile_hierarchy_test(code);
    assert!(
        result.is_ok(),
        "Entity sibling relationships should compile successfully. Error: {:?}",
        result.err()
    );
}

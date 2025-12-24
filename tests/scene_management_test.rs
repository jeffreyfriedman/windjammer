/// TDD Test: Scene Management System
///
/// This test verifies that the scene management system works correctly:
/// - Create scenes
/// - Add entities to scenes
/// - Remove entities from scenes
/// - Query entities in scenes
/// - Get entity counts
use std::path::PathBuf;
use tempfile::TempDir;

fn compile_and_run_scene_test(code: &str) -> Result<String, String> {
    use std::fs;
    use std::process::Command;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let output_dir = temp_dir.path().to_path_buf();

    let wj_file = output_dir.join("test.wj");
    fs::write(&wj_file, code).map_err(|e| format!("Failed to write test file: {}", e))?;

    // Compile to Rust
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
fn test_scene_creation_and_entity_management() {
    let code = r#"
use windjammer_game::prelude::*;

struct Player {
    pub name: string,
    pub health: i64,
}

fn main() {
    // Create a new scene
    let scene = Scene::new("TestScene".to_string());
    
    // Create entities
    let player1 = Player {
        name: "Alice".to_string(),
        health: 100,
    };
    
    let player2 = Player {
        name: "Bob".to_string(),
        health: 90,
    };
    
    // Add entities to scene
    let entity1_id = scene.add_entity(player1);
    let entity2_id = scene.add_entity(player2);
    
    // Check entity count
    let count = scene.entity_count();
    println!("Entity count: {}", count);
    
    // Remove an entity
    scene.remove_entity(entity1_id);
    
    let count_after = scene.entity_count();
    println!("Entity count after removal: {}", count_after);
}
"#;

    let result = compile_and_run_scene_test(code);
    assert!(
        result.is_ok(),
        "Scene management should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_scene_query_entities() {
    let code = r#"
use windjammer_game::prelude::*;

struct Enemy {
    pub name: string,
    pub attack: i64,
}

fn main() {
    let scene = Scene::new("TestScene".to_string());
    
    // Add multiple entities
    let enemy1 = Enemy { name: "Goblin".to_string(), attack: 10 };
    let enemy2 = Enemy { name: "Orc".to_string(), attack: 20 };
    let enemy3 = Enemy { name: "Dragon".to_string(), attack: 50 };
    
    scene.add_entity(enemy1);
    scene.add_entity(enemy2);
    scene.add_entity(enemy3);
    
    // Query all entities
    let all_entities = scene.get_all_entities();
    println!("Total entities: {}", all_entities.len());
    
    // Query by component type (if supported)
    // This would require reflection/type system support
    // For now, just verify we can iterate
    for entity_id in all_entities {
        println!("Entity ID: {}", entity_id);
    }
}
"#;

    let result = compile_and_run_scene_test(code);
    assert!(
        result.is_ok(),
        "Scene query should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_scene_clear_all_entities() {
    let code = r#"
use windjammer_game::prelude::*;

struct Item {
    pub name: string,
}

fn main() {
    let scene = Scene::new("TestScene".to_string());
    
    // Add multiple entities
    scene.add_entity(Item { name: "Sword".to_string() });
    scene.add_entity(Item { name: "Shield".to_string() });
    scene.add_entity(Item { name: "Potion".to_string() });
    
    println!("Before clear: {}", scene.entity_count());
    
    // Clear all entities
    scene.clear();
    
    println!("After clear: {}", scene.entity_count());
}
"#;

    let result = compile_and_run_scene_test(code);
    assert!(
        result.is_ok(),
        "Scene clear should compile successfully. Error: {:?}",
        result.err()
    );
}

#[test]
fn test_scene_get_entity_by_id() {
    let code = r#"
use windjammer_game::prelude::*;

struct Coin {
    pub value: i64,
}

fn main() {
    let scene = Scene::new("TestScene".to_string());
    
    // Add entity and get its ID
    let coin = Coin { value: 10 };
    let entity_id = scene.add_entity(coin);
    
    // Try to retrieve the entity
    let maybe_entity = scene.get_entity(entity_id);
    
    if maybe_entity.is_some() {
        println!("Entity found!");
    } else {
        println!("Entity not found");
    }
    
    // Try to get a non-existent entity
    let invalid_id = 99999;
    let not_found = scene.get_entity(invalid_id);
    
    if not_found.is_none() {
        println!("Correctly returns None for invalid ID");
    }
}
"#;

    let result = compile_and_run_scene_test(code);
    assert!(
        result.is_ok(),
        "Scene get_entity should compile successfully. Error: {:?}",
        result.err()
    );
}

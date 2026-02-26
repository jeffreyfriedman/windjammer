//! Comprehensive dogfooding tests - Find remaining compiler bugs
//! These tests exercise common patterns in game code to discover issues

use std::path::PathBuf;
use std::fs;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_iteration_and_mutation() {
    // Common game pattern: iterate and mutate collections
    let test_dir = std::env::temp_dir().join(format!(
        "wj_vec_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct Enemy {
    x: f32,
    y: f32,
    health: i32,
}

fn update_enemies(enemies: Vec<Enemy>, dt: f32) {
    for enemy in enemies {
        enemy.x += dt * 10.0;
        if enemy.health <= 0 {
            // Remove dead enemies
        }
    }
}

fn main() {
    let mut enemies = Vec::new();
    enemies.push(Enemy { x: 0.0, y: 0.0, health: 100 });
    update_enemies(enemies, 0.016);
}
"#;
    
    fs::write(test_dir.join("vec_test.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("vec_test.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for vec iteration");

    let rust_code = fs::read_to_string(test_dir.join("build/vec_test.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_chaining() {
    // Common game pattern: method chaining (builder pattern)
    let test_dir = std::env::temp_dir().join(format!(
        "wj_chain_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
    
    fn add(self, other: Vec2) -> Vec2 {
        Vec2 { x: self.x + other.x, y: self.y + other.y }
    }
    
    fn scale(self, factor: f32) -> Vec2 {
        Vec2 { x: self.x * factor, y: self.y * factor }
    }
}

fn main() {
    let pos = Vec2::new(1.0, 2.0)
        .add(Vec2::new(3.0, 4.0))
        .scale(2.0);
    println("Position: {}, {}", pos.x, pos.y);
}
"#;
    
    fs::write(test_dir.join("chain_test.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("chain_test.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for method chaining");

    let rust_code = fs::read_to_string(test_dir.join("build/chain_test.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_option_result_handling() {
    // Common game pattern: Option/Result usage
    let test_dir = std::env::temp_dir().join(format!(
        "wj_option_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
fn find_enemy(id: i32) -> Option<i32> {
    if id > 0 {
        Some(id * 10)
    } else {
        None
    }
}

fn main() {
    let result = find_enemy(5);
    match result {
        Some(value) => println("Found: {}", value),
        None => println("Not found"),
    }
    
    if let Some(val) = find_enemy(3) {
        println("Got: {}", val);
    }
}
"#;
    
    fs::write(test_dir.join("option_test.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("option_test.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for Option handling");

    let rust_code = fs::read_to_string(test_dir.join("build/option_test.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_struct_field_access() {
    // Common game pattern: nested struct field access
    let test_dir = std::env::temp_dir().join(format!(
        "wj_nested_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct Transform {
    x: f32,
    y: f32,
}

struct Entity {
    transform: Transform,
    health: i32,
}

fn update(entity: Entity, dx: f32) {
    entity.transform.x += dx;
    entity.transform.y += dx * 0.5;
}

fn main() {
    let mut e = Entity {
        transform: Transform { x: 0.0, y: 0.0 },
        health: 100,
    };
    update(e, 5.0);
    println("Position: {}, {}", e.transform.x, e.transform.y);
}
"#;
    
    fs::write(test_dir.join("nested_test.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("nested_test.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for nested field access");

    let rust_code = fs::read_to_string(test_dir.join("build/nested_test.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_array_indexing() {
    // Common game pattern: array indexing
    let test_dir = std::env::temp_dir().join(format!(
        "wj_array_test_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
fn main() {
    let mut arr = [1, 2, 3, 4, 5];
    arr[2] = 10;
    let val = arr[2];
    println("Value: {}", val);
    
    let mut vec = Vec::new();
    vec.push(100);
    vec.push(200);
    vec[0] = 150;
    println("First: {}", vec[0]);
}
"#;
    
    fs::write(test_dir.join("array_test.wj"), windjammer_code).unwrap();
    
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("array_test.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        println!("STDERR:\n{}", String::from_utf8_lossy(&output.stderr));
        println!("STDOUT:\n{}", String::from_utf8_lossy(&output.stdout));
    }

    assert!(output.status.success(),
            "wj build should succeed for array indexing");

    let rust_code = fs::read_to_string(test_dir.join("build/array_test.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    fs::remove_dir_all(test_dir).ok();
}

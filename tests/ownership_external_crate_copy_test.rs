use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// TDD Test: Ownership inference for Copy types from external crates
/// 
/// Bug: When calling methods from external crates where a parameter
/// is a Copy type passed by value (owned), the compiler incorrectly
/// infers & (reference) instead of owned.
/// 
/// Example from breach-protocol:
/// - External crate: windjammer_game_core::physics::collision::AABB
/// - Method: pub fn intersects_aabb(&self, other: AABB) -> bool
/// - Call: future_box.intersects_aabb(wall)
/// - Generated (WRONG): future_box.intersects_aabb(&wall)
/// - Should generate: future_box.intersects_aabb(wall)

#[test]
fn test_external_crate_copy_type_owned_param() {
    let temp_dir = TempDir::new().unwrap();
    
    // Step 1: Create a library crate with Copy type and method
    let lib_dir = temp_dir.path().join("my_physics");
    fs::create_dir(&lib_dir).unwrap();
    fs::create_dir(lib_dir.join("src")).unwrap();
    
    // Library: defines AABB (Copy) with intersects_aabb method
    fs::write(lib_dir.join("src/collision.wj"), r#"
pub struct AABB {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32
}

impl AABB {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> AABB {
        AABB { min_x: min_x, min_y: min_y, max_x: max_x, max_y: max_y }
    }
    
    // Method takes AABB by value (owned) - AABB is Copy
    pub fn intersects_aabb(self, other: AABB) -> bool {
        self.max_x >= other.min_x && 
        self.min_x <= other.max_x &&
        self.max_y >= other.min_y &&
        self.min_y <= other.max_y
    }
}
"#).unwrap();
    
    fs::write(lib_dir.join("src/mod.wj"), r#"
pub mod collision
"#).unwrap();
    
    // Step 2: Build the library crate
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(&lib_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg("src/mod.wj")
        .output()
        .unwrap();
    
    assert!(
        output.status.success(),
        "Library crate should build! Stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    // Step 3: Create binary crate that imports from library
    let bin_dir = temp_dir.path().join("my_game");
    fs::create_dir(&bin_dir).unwrap();
    fs::create_dir(bin_dir.join("src")).unwrap();
    
    // Binary: imports AABB and calls intersects_aabb
    fs::write(bin_dir.join("src/player.wj"), r#"
use my_physics::collision::AABB

pub struct Player {
    pub x: f32,
    pub y: f32
}

impl Player {
    // EXACT game pattern: owned AABB parameter from external crate
    pub fn will_collide_with(self, wall: AABB) -> bool {
        let future_box = AABB::new(
            self.x - 0.5,
            self.y,
            self.x + 0.5,
            self.y + 1.8
        )
        
        // BUG: Compiler incorrectly generates: future_box.intersects_aabb(&wall)
        // SHOULD generate: future_box.intersects_aabb(wall)
        // Because AABB is Copy and method expects owned
        future_box.intersects_aabb(wall)
    }
}
"#).unwrap();
    
    // Step 4: Build the binary (expect it to work once fix is in)
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(&bin_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg("src/player.wj")
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should compile without errors
    assert!(
        output.status.success(),
        "Binary should build when calling external crate Copy type method!\n\nStderr:\n{}",
        stderr
    );
    
    // Verify generated code doesn't have unnecessary &
    let build_dir = bin_dir.join("build");
    let generated = fs::read_to_string(build_dir.join("player.rs")).unwrap();
    
    // Should NOT generate: future_box.intersects_aabb(&wall)
    // Should generate: future_box.intersects_aabb(wall)
    assert!(
        !generated.contains("intersects_aabb(&wall)") && 
        !generated.contains("intersects_aabb(& wall)"),
        "Generated code should NOT add & when passing owned Copy parameter from external crate!\nGenerated:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("intersects_aabb"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_external_crate_copy_vs_noncopy() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create library with both Copy and non-Copy types
    let lib_dir = temp_dir.path().join("test_lib");
    fs::create_dir(&lib_dir).unwrap();
    fs::create_dir(lib_dir.join("src")).unwrap();
    
    fs::write(lib_dir.join("src/types.wj"), r#"
// Copy type (all fields are Copy)
pub struct Point {
    pub x: f32,
    pub y: f32
}

// Non-Copy type (contains Vec, which is not Copy)
pub struct Shape {
    pub vertices: Vec<Point>
}

impl Point {
    pub fn distance_to(self, other: Point) -> f32 {
        let dx = self.x - other.x
        let dy = self.y - other.y
        dx * dx + dy * dy
    }
}

impl Shape {
    pub fn contains_point(self, p: Point) -> bool {
        self.vertices.len() > 0
    }
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(&lib_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg("src/types.wj")
        .output()
        .unwrap();
    
    assert!(output.status.success(), "Library should build");
    
    // Create binary using both types
    let bin_dir = temp_dir.path().join("test_bin");
    fs::create_dir(&bin_dir).unwrap();
    fs::create_dir(bin_dir.join("src")).unwrap();
    
    fs::write(bin_dir.join("src/main.wj"), r#"
use test_lib::types::Point
use test_lib::types::Shape

pub fn main() {
    let p1 = Point { x: 0.0, y: 0.0 }
    let p2 = Point { x: 1.0, y: 1.0 }
    
    // Copy type - should pass by value (no &)
    let dist = p1.distance_to(p2)
    
    let shape = Shape { vertices: Vec::new() }
    
    // Copy type as param - should pass by value (no &)
    let contains = shape.contains_point(p1)
    
    println("{}", dist)
    println("{}", contains)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(&bin_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg("src/main.wj")
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    assert!(
        output.status.success(),
        "Should handle both Copy and non-Copy types from external crates!\n\nStderr:\n{}",
        stderr
    );
    
    let build_dir = bin_dir.join("build");
    let generated = fs::read_to_string(build_dir.join("main.rs")).unwrap();
    
    // Verify Copy types are passed by value
    assert!(
        !generated.contains("distance_to(&p2)") && 
        !generated.contains("contains_point(&p1)"),
        "Copy types should be passed by value, not reference!\nGenerated:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("distance_to") || l.contains("contains_point"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

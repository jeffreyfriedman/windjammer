use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// TDD Test: Ownership inference for Copy types in method calls
/// 
/// Bug: When calling a method that takes a Copy type by value,
/// the compiler incorrectly infers & (reference) instead of owned.
/// 
/// Example: `box.intersects(wall)` where wall: AABB (Copy)
/// - Method signature: `fn intersects(&self, other: AABB) -> bool`
/// - Generated (wrong): `box.intersects(&wall)`
/// - Should generate: `box.intersects(wall)`

#[test]
fn test_copy_type_method_param_no_explicit_ref() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    // Copy type (simple struct - auto-derives Copy)
    // Method takes Copy type by value (owned)
    // Windjammer code passes it without & (owned)
    // Compiler should generate: value (not &value)
    fs::write(&test_file, r#"
struct Point {
    x: f32,
    y: f32
}

struct Shape {
    center: Point
}

impl Shape {
    fn contains_point(self, p: Point) -> bool {
        let dx = self.center.x - p.x
        let dy = self.center.y - p.y
        dx * dx + dy * dy < 1.0
    }
}

pub fn main() {
    let shape = Shape { center: Point { x: 0.0, y: 0.0 } }
    let test_point = Point { x: 0.5, y: 0.5 }
    
    // Call method with Copy type (no explicit &)
    // Compiler should pass as owned (Copy happens automatically)
    let result = shape.contains_point(test_point)
    println("{}", result)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should compile without ownership errors
    assert!(
        output.status.success(),
        "Build should SUCCEED! Windjammer code doesn't use &, compiler shouldn't add it.\n\nStderr:\n{}",
        stderr
    );
    
    // Verify generated code doesn't have unnecessary &
    let build_dir = temp_dir.path().join("build");
    let generated = fs::read_to_string(build_dir.join("test.rs")).unwrap();
    
    // Should generate: shape.contains_point(test_point)
    // NOT: shape.contains_point(&test_point)
    assert!(
        generated.contains("contains_point(test_point)") || 
        !generated.contains("contains_point(&test_point)"),
        "Generated code should pass Copy type by value, not reference!\nFound:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("contains_point"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_copy_type_owned_param_to_method() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    // Matches the EXACT game pattern:
    // - Function parameter: wall: AABB (owned)
    // - Passed to method: intersects_aabb(wall) - no &
    // - Method expects: other: AABB (owned)
    fs::write(&test_file, r#"
struct AABB {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32
}

impl AABB {
    fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> AABB {
        AABB { min_x: min_x, min_y: min_y, max_x: max_x, max_y: max_y }
    }
    
    fn intersects_aabb(self, other: AABB) -> bool {
        self.max_x >= other.min_x && 
        self.min_x <= other.max_x &&
        self.max_y >= other.min_y &&
        self.min_y <= other.max_y
    }
}

struct Player {
    x: f32,
    y: f32
}

impl Player {
    // EXACT game pattern: owned AABB parameter
    fn will_collide_with(self, wall: AABB) -> bool {
        let future_box = AABB::new(
            self.x - 0.5,
            self.y,
            self.x + 0.5,
            self.y + 1.8
        )
        
        // Pass owned parameter to method (no &)
        // Compiler should generate: future_box.intersects_aabb(wall)
        // NOT: future_box.intersects_aabb(&wall)
        future_box.intersects_aabb(wall)
    }
}

pub fn main() {
    let player = Player { x: 0.0, y: 0.0 }
    let wall = AABB::new(0.5, 0.5, 1.5, 1.5)
    
    let collides = player.will_collide_with(wall)
    println("{}", collides)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should compile without "expected AABB, found &AABB" errors
    assert!(
        output.status.success(),
        "Build should SUCCEED! AABB is Copy, no & needed.\n\nStderr:\n{}",
        stderr
    );
    
    let build_dir = temp_dir.path().join("build");
    let generated = fs::read_to_string(build_dir.join("test.rs")).unwrap();
    
    // Should NOT generate: future_box.intersects_aabb(&wall)
    // Should generate: future_box.intersects_aabb(wall)
    // This is the EXACT pattern that breaks in the game!
    assert!(
        !generated.contains("intersects_aabb(&wall)") && 
        !generated.contains("intersects_aabb(& wall)"),
        "Generated code should NOT add & when passing owned Copy parameter!\nGenerated pattern:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("intersects_aabb"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

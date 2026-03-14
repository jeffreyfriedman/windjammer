/// TDD: CircleCollider/AABB f32 type consistency
///
/// Bug: CircleCollider used f64 while AABB used f32, causing type errors in
/// intersects_aabb (self.x < aabb.x etc - f64 vs f32 comparison).
///
/// Fix: CircleCollider uses f32 throughout for consistency with AABB.
///
/// Verifies: Code with CircleCollider + AABB compiles without f32/f64 mismatch.

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("circle_aabb_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));

    std::fs::create_dir_all(&output_dir).ok();
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let status = Command::new(&wj_path)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj compiler");

    assert!(status.success(), "Compilation failed");

    let rust_code = std::fs::read_to_string(&output_file).expect("Failed to read generated Rust");

    let _ = std::fs::remove_file(&test_file);
    let _ = std::fs::remove_dir_all(&output_dir);

    rust_code
}

fn verify_rust_compiles(rust_code: &str) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("circle_aabb_rust_{}_{}", std::process::id(), unique_id);
    let rust_file = temp_dir.join(format!("{}.rs", test_name));

    std::fs::write(&rust_file, rust_code).expect("Failed to write Rust file");

    let output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            rust_file.to_str().unwrap(),
            "-o",
            temp_dir.join(format!("{}.rlib", test_name)).to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let _ = std::fs::remove_file(&rust_file);
    let _ = std::fs::remove_file(temp_dir.join(format!("{}.rlib", test_name)));

    assert!(
        output.status.success(),
        "Generated Rust should compile (no f32/f64 type error):\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_circle_aabb_intersection_compiles_without_type_error() {
    // CircleCollider and AABB both use f32 - no mixing
    let source = r#"
pub struct AABB {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl AABB {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> AABB {
        AABB { x, y, width, height }
    }
}

pub struct CircleCollider {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
}

impl CircleCollider {
    pub fn new(x: f32, y: f32, radius: f32) -> CircleCollider {
        CircleCollider { x, y, radius }
    }

    pub fn intersects_aabb(self, aabb: AABB) -> bool {
        let mut closest_x = self.x;
        let mut closest_y = self.y;
        if self.x < aabb.x {
            closest_x = aabb.x;
        } else if self.x > aabb.x + aabb.width {
            closest_x = aabb.x + aabb.width;
        }
        if self.y < aabb.y {
            closest_y = aabb.y;
        } else if self.y > aabb.y + aabb.height {
            closest_y = aabb.y + aabb.height;
        }
        let dx = self.x - closest_x;
        let dy = self.y - closest_y;
        let distance_squared = dx * dx + dy * dy;
        distance_squared <= self.radius * self.radius
    }
}

pub fn test_intersection() -> bool {
    let circle = CircleCollider::new(5.0, 5.0, 2.0);
    let aabb = AABB::new(3.0, 3.0, 4.0, 4.0);
    circle.intersects_aabb(aabb)
}
"#;

    let rust_code = compile_and_get_rust(source);
    verify_rust_compiles(&rust_code);
}

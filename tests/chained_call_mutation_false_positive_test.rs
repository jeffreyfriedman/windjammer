use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// TDD Test: Chained method call mutation false positive
///
/// Bug: When a Copy parameter is passed as an ARGUMENT to a method in a chain,
/// the mutation detector falsely considers it part of the receiver chain.
///
/// Example: `f.cross(up).normalize()`
///   - `up` is an argument to `cross`, NOT the receiver of `normalize`
///   - But `expr_contains_identifier("up", f.cross(up))` returned true
///   - Analyzer then checked if `normalize` mutates → false positive
///
/// Fix: Use `is_in_receiver_chain` which only follows the object path,
/// not arguments of nested method calls.

#[test]
fn test_copy_param_as_argument_in_chained_call_stays_owned() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    fs::write(
        &test_file,
        r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x
        )
    }

    pub fn normalize(self) -> Vec3 {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
        if len > 0.0 {
            Vec3::new(self.x / len, self.y / len, self.z / len)
        } else {
            Vec3::new(0.0, 0.0, 0.0)
        }
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }
}

pub fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Vec3 {
    let f = (target - eye).normalize()
    let s = f.cross(up).normalize()
    s
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wj build failed:\nstderr: {}",
        stderr
    );

    let build_dir = temp_dir.path().join("build");
    let generated = fs::read_to_string(build_dir.join("test.rs")).unwrap();

    // up should be owned Vec3 (Copy type, not mutated), NOT &mut Vec3
    assert!(
        !generated.contains("up: &mut Vec3"),
        "BUG: up should NOT be &mut Vec3. It's only used as an argument to cross(), \
         not as a receiver of normalize(). Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("up: &Vec3"),
        "up should be owned Vec3 (Copy type), not &Vec3. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("up: Vec3"),
        "up should be owned Vec3 (passed by value, Copy type). Generated:\n{}",
        generated
    );
}

#[test]
fn test_direct_receiver_still_gets_mut_inference() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");

    fs::write(
        &test_file,
        r#"
struct Items {
    data: Vec<i32>
}

impl Items {
    pub fn new() -> Items {
        Items { data: Vec::new() }
    }
    pub fn add(self, item: i32) {
        self.data.push(item)
    }
}

pub fn add_items(items: Items) {
    items.add(42)
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wj build failed:\nstderr: {}",
        stderr
    );

    let build_dir = temp_dir.path().join("build");
    let generated = fs::read_to_string(build_dir.join("test.rs")).unwrap();

    // items is the DIRECT receiver of add() which mutates, so should be &mut
    assert!(
        generated.contains("items: &mut Items"),
        "items should be &mut Items since it's the direct receiver of a mutating method. Generated:\n{}",
        generated
    );
}

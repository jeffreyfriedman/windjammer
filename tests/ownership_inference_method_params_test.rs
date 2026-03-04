/// TDD: Test that ownership inference correctly infers &mut for parameters
/// used in method calls that require &mut self
///
/// Bug discovered during Breach Protocol dogfooding:
/// When a parameter is passed to a method that requires &mut, the compiler
/// should infer that the parameter itself needs &mut.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn get_wj_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn compile_to_rust(source: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir(&out_dir).expect("Failed to create output dir");

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
    let rust_code = fs::read_to_string(rust_file).expect("Failed to read generated Rust");
    Ok(rust_code)
}

#[test]
fn test_infer_mut_from_method_call() {
    let source = r#"
struct Grid {
    data: Vec<i32>,
}

impl Grid {
    fn set(self, value: i32) {
        self.data.push(value)
    }
}

fn fill_grid(grid: Grid) {
    grid.set(42)
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should infer &mut for grid parameter
    assert!(
        rust_code.contains("fn fill_grid") && rust_code.contains("grid: &mut Grid"),
        "Should infer &mut Grid for parameter used in mutating method call.\n\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_infer_mut_self_from_field_method_call() {
    let source = r#"
struct Camera {
    x: f32,
}

impl Camera {
    fn move_to(self, x: f32) {
        self.x = x
    }
}

struct Game {
    camera: Camera,
}

impl Game {
    fn update_camera(self) {
        self.camera.move_to(10.0)
    }
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should infer &mut self for update_camera
    // Bug: analyzer only checks self.method(), not self.field.method()
    assert!(
        rust_code.contains("fn update_camera(&mut self)"),
        "Should infer &mut self when calling mutating method on field.\n\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_infer_mut_self_from_field_extern_method() {
    // Test with extern method (like smooth_follow from windjammer-app)
    let source = r#"
extern fn camera_smooth_follow(camera: &mut Camera, x: f32, y: f32, z: f32, speed: f32)

struct Camera {
    x: f32,
}

impl Camera {
    fn smooth_follow(self, x: f32, y: f32, z: f32, speed: f32) {
        camera_smooth_follow(self, x, y, z, speed)
    }
}

struct Game {
    camera: Camera,
}

impl Game {
    fn update(self) {
        self.camera.smooth_follow(10.0, 5.0, -10.0, 0.1)
    }
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should infer &mut self for update method
    assert!(
        rust_code.contains("fn update(&mut self)"),
        "Should infer &mut self when calling field method that requires &mut.\n\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_usize_index_not_inferred_as_mut() {
    // Bug: usize parameters used as array indices incorrectly inferred as &mut usize
    let source = r#"
struct Skill {
    unlocked: bool,
    points_required: u32,
}

impl Skill {
    fn can_unlock(self, points: u32) -> bool {
        !self.unlocked && points >= self.points_required
    }
    
    fn unlock(self) {
        self.unlocked = true
    }
}

struct SkillTree {
    skills: Vec<Skill>,
    total_points: u32,
}

impl SkillTree {
    fn unlock_skill(self, index: usize, points: u32) -> bool {
        if index >= self.skills.len() {
            return false
        }
        if !self.skills[index].can_unlock(points) {
            return false
        }
        self.skills[index].unlock()
        self.total_points = self.total_points + self.skills[index].points_required
        true
    }
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should keep index as usize, NOT &mut usize
    assert!(
        rust_code.contains("index: usize"),
        "Should NOT infer &mut for usize parameters used as indices.\n\nGenerated:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("index: &mut usize"),
        "Should NOT generate &mut usize for index parameter.\n\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_infer_mut_nested_field_method_call() {
    let source = r#"
struct Inner {
    value: i32,
}

impl Inner {
    fn set(self, v: i32) {
        self.value = v
    }
}

struct Outer {
    inner: Inner,
}

fn modify_nested(outer: Outer) {
    outer.inner.set(42)
}
"#;

    let rust_code = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should infer &mut for outer parameter
    assert!(
        rust_code.contains("fn modify_nested") && rust_code.contains("outer: &mut Outer"),
        "Should infer &mut when mutating nested field via method call.\n\nGenerated:\n{}",
        rust_code
    );
}

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Test: Static helper method + instance method calls on same parameter
///
/// Bug scenario from dogfooding: FpsCamera::update(self, dt, grid) calls both:
///   - FpsCamera::collides_aabb(grid, pos) [static, borrows grid]
///   - FpsCamera::log_blocking_voxel(grid, x, y, z) [static, borrows grid]
///   - grid.get(x, y, z) [instance, borrows grid]
///
/// The compiler incorrectly inferred `grid` as Owned when a static helper
/// method on the SAME type was introduced. The grid parameter should remain
/// Borrowed because all callees only borrow it.
///
/// This tests the specific combination of:
/// 1. Parameter passed to static method on same type (Type::method(param, ...))
/// 2. Parameter used as receiver for instance methods (param.method(...))
/// 3. Multiple static calls with the same parameter
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_static_helper_with_instance_method_on_param() {
    let source = r#"
struct VoxelGrid {
    data: Vec<i32>
}

impl VoxelGrid {
    fn get(self, x: i32, y: i32, z: i32) -> u8 {
        0u8
    }
}

struct Vec3 {
    x: f32,
    y: f32,
    z: f32
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x: x, y: y, z: z }
    }
}

struct Camera {
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
    blocked: bool
}

impl Camera {
    fn collides_aabb(grid: VoxelGrid, pos: Vec3) -> bool {
        let ix = pos.x as i32
        let iy = pos.y as i32
        let iz = pos.z as i32
        let mat = grid.get(ix, iy, iz)
        mat != 0u8
    }

    fn log_blocking_voxel(grid: VoxelGrid, x: i32, y: i32, z: i32) {
        let mat = grid.get(x, y, z)
        if mat != 0u8 {
            println!("Blocked at ({}, {}, {}) mat={}", x, y, z, mat as i32)
        }
    }

    fn update(self, dt: f32, grid: VoxelGrid) {
        let dx = self.pos_x * dt
        let test_pos = Vec3::new(self.pos_x + dx, self.pos_y, self.pos_z)
        if !Camera::collides_aabb(grid, test_pos) {
            self.pos_x = self.pos_x + dx
        } else {
            self.blocked = true
            let bx = test_pos.x as i32
            let by = test_pos.y as i32
            let bz = test_pos.z as i32
            Camera::log_blocking_voxel(grid, bx, by, bz)
        }
    }
}
"#;

    let test_dir = tempdir().expect("tempdir");
    let source_file = test_dir.path().join("camera_test.wj");
    fs::write(&source_file, source).unwrap();

    let output_dir = test_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg(source_file.to_str().unwrap())
        .output()
        .expect("failed to run wj");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    let generated = fs::read_to_string(output_dir.join("camera_test.rs"))
        .expect("Generated .rs file not found");

    // grid in update() should be &VoxelGrid (Borrowed) because:
    // - Camera::collides_aabb borrows grid (calls grid.get which is &self)
    // - Camera::log_blocking_voxel borrows grid (calls grid.get which is &self)
    // Neither consumes grid.
    assert!(
        generated.contains("fn update(&mut self, dt: f32, grid: &VoxelGrid)")
            || generated.contains("fn update(&mut self, dt: f32, grid: &VoxelGrid,"),
        "Expected update() to have grid: &VoxelGrid (Borrowed).\n\
         Bug: static helper Camera::log_blocking_voxel(grid, ...) caused grid\n\
         to be inferred as Owned instead of Borrowed.\n\
         Generated code:\n{}",
        generated
    );

    // collides_aabb is static, grid should be &VoxelGrid
    assert!(
        generated.contains("fn collides_aabb(grid: &VoxelGrid"),
        "Expected collides_aabb to borrow grid.\nGenerated:\n{}",
        generated
    );

    // log_blocking_voxel is static, grid should be &VoxelGrid
    assert!(
        generated.contains("fn log_blocking_voxel(grid: &VoxelGrid"),
        "Expected log_blocking_voxel to borrow grid.\nGenerated:\n{}",
        generated
    );

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_static_helper_with_mutation_infers_mut_borrow() {
    let source = r#"
struct Grid {
    data: Vec<i32>
}

impl Grid {
    fn set(self, x: i32, val: i32) {
        self.data.push(val)
    }

    fn get(self, x: i32) -> i32 {
        0
    }
}

struct Builder {
    count: i32
}

impl Builder {
    fn fill_at(grid: Grid, x: i32, val: i32) {
        grid.set(x, val)
    }

    fn build(self, grid: Grid) {
        let mut i = 0
        while i < self.count {
            Builder::fill_at(grid, i, i * 2)
            i = i + 1
        }
    }
}
"#;

    let test_dir = tempdir().expect("tempdir");
    let source_file = test_dir.path().join("builder_test.wj");
    fs::write(&source_file, source).unwrap();

    let output_dir = test_dir.path().join("build");
    fs::create_dir_all(&output_dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg("--output")
        .arg(output_dir.to_str().unwrap())
        .arg(source_file.to_str().unwrap())
        .output()
        .expect("failed to run wj");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "wj build failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    let generated = fs::read_to_string(output_dir.join("builder_test.rs"))
        .expect("Generated .rs file not found");

    // grid in build() should be &mut Grid because fill_at calls grid.set which is &mut self
    assert!(
        generated.contains("fn build(&mut self, grid: &mut Grid)")
            || generated.contains("fn build(&self, grid: &mut Grid"),
        "Expected build() to have grid: &mut Grid.\n\
         Static helper fill_at mutates grid, so build should infer &mut.\n\
         Generated code:\n{}",
        generated
    );

    // fill_at is static, grid should be &mut Grid
    assert!(
        generated.contains("fn fill_at(grid: &mut Grid"),
        "Expected fill_at to have grid: &mut Grid.\nGenerated:\n{}",
        generated
    );

    let _ = fs::remove_dir_all(&test_dir);
}

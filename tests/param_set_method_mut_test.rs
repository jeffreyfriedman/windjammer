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

/// TDD: Parameters with .set() method calls should be inferred as &mut
///
/// Bug: grid.set(x, y, z, material) calls VoxelGrid::set(&mut self, ...)
/// but the compiler doesn't recognize "set" as a mutating method name.
/// Result: grid parameter inferred as & instead of &mut.
use windjammer::method_registry;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_set_is_mutating_method() {
    assert!(
        method_registry::mutates_receiver("set"),
        "\"set\" should be recognized as a mutating method"
    );
}

#[test]
fn test_known_mutating_methods_include_set() {
    let mutating = vec!["push", "pop", "insert", "remove", "clear", "set", "fill"];
    for method in mutating {
        assert!(
            method_registry::mutates_receiver(method),
            "\"{}\" should be recognized as a mutating method",
            method
        );
    }
}

#[test]
fn test_param_with_set_call_inferred_mut() {
    let source = r#"
struct VoxelGrid {
    data: Vec<u8>,
}

impl VoxelGrid {
    pub fn set(self, x: i32, y: i32, z: i32, value: u8) {
        self.data[0] = value
    }
    pub fn get(self, x: i32, y: i32, z: i32) -> u8 {
        self.data[0]
    }
}

pub fn stamp_voxel(grid: VoxelGrid, x: i32, y: i32, z: i32, material: u8) {
    grid.set(x, y, z, material)
}
"#;
    let compiled = test_utils::compile_single(source);
    assert!(
        compiled.contains("grid: &mut VoxelGrid"),
        "grid parameter should be &mut VoxelGrid since grid.set() mutates. Got:\n{}",
        compiled
    );
}

#[test]
fn test_param_with_get_call_inferred_borrowed() {
    let source = r#"
struct VoxelGrid {
    data: Vec<u8>,
}

impl VoxelGrid {
    pub fn set(self, x: i32, y: i32, z: i32, value: u8) {
        self.data[0] = value
    }
    pub fn get(self, x: i32, y: i32, z: i32) -> u8 {
        self.data[0]
    }
}

pub fn read_voxel(grid: VoxelGrid, x: i32, y: i32, z: i32) -> u8 {
    grid.get(x, y, z)
}
"#;
    let compiled = test_utils::compile_single(source);
    assert!(
        compiled.contains("grid: &VoxelGrid"),
        "grid parameter should be &VoxelGrid since grid.get() only reads. Got:\n{}",
        compiled
    );
}

#[test]
fn test_param_with_set_in_while_loop() {
    let source = r#"
struct Grid {
    data: Vec<u8>,
}

impl Grid {
    pub fn set(self, x: i32, y: i32, z: i32, value: u8) {
        self.data[0] = value
    }
}

pub fn fill_cube_free(grid: Grid, size: i32, material: u8) {
    let mut x: i32 = 0
    while x < size {
        let mut y: i32 = 0
        while y < size {
            let mut z: i32 = 0
            while z < size {
                grid.set(x, y, z, material)
                z = z + 1
            }
            y = y + 1
        }
        x = x + 1
    }
}
"#;
    let compiled = test_utils::compile_single(source);
    assert!(
        compiled.contains("grid: &mut Grid"),
        "grid parameter in nested while loops calling .set() should be &mut. Got:\n{}",
        compiled
    );
}

#[test]
fn test_param_with_set_in_impl_associated_fn() {
    let source = r#"
struct Grid {
    data: Vec<u8>,
}

impl Grid {
    pub fn set(self, x: i32, y: i32, z: i32, value: u8) {
        self.data[0] = value
    }
}

pub struct Renderer {}

impl Renderer {
    fn stamp_cube(grid: Grid, cx: i32, cy: i32, cz: i32, material: u8) {
        let mut dx: i32 = -1
        while dx <= 1 {
            grid.set(cx + dx, cy, cz, material)
            dx = dx + 1
        }
    }
}
"#;
    let compiled = test_utils::compile_single(source);
    assert!(
        compiled.contains("grid: &mut Grid"),
        "grid param in impl-associated function calling .set() should be &mut. Got:\n{}",
        compiled
    );
}

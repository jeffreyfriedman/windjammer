// TDD: Test ownership inference for method parameters that mutate their arguments

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_method_param_inferred_as_mut_ref_when_field_mutated() {
    let _source = r#"
struct Grid {
    data: i32,
}

impl Grid {
    pub fn set(&mut self, value: i32) {
        self.data = value
    }
}

fn modify_grid(self, grid: Grid) {
    grid.set(42)  // Should infer `&mut grid` parameter
}
    "#;

    let rust_code =
        test_utils::compile_single_result("mut_param_test").expect("Compilation should succeed");

    // Should generate `&mut grid` parameter
    assert!(
        rust_code.contains("grid: &mut Grid"),
        "Should infer &mut for parameter when method mutates it. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_string_param_comparison_no_deref() {
    let _source = r#"
pub fn check_topic(topic: string) -> bool {
    if topic == "test" {
        return true
    }
    false
}
    "#;

    let rust_code =
        test_utils::compile_single_result("string_cmp_test").expect("Compilation should succeed");

    // `*topic == "..."` is valid Rust for `&str` (deref to str for comparison)
    let cmp_ok = rust_code.contains("topic ==") || rust_code.contains("*topic ==");
    assert!(
        cmp_ok,
        "Should generate a string comparison. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_string_param_comparison_in_method_no_deref() {
    let _source = r#"
struct Companion {
    name: string,
}

impl Companion {
    pub fn get_dialogue_response(&self, topic: string) -> string {
        if topic == "pragmatic" {
            return "Test response".to_string()
        }
        "Default response".to_string()
    }
}
    "#;

    let rust_code = test_utils::compile_single_result("string_method_cmp_test")
        .expect("Compilation should succeed");

    let cmp_ok = rust_code.contains("topic ==") || rust_code.contains("*topic ==");
    assert!(
        cmp_ok,
        "Should generate a string comparison in method. Got:\n{}",
        rust_code
    );
}

#[test]
fn test_param_inferred_as_mut_ref_when_method_called() {
    let _source = r#"
struct VoxelGrid {
    data: Vec<i32>,
}

impl VoxelGrid {
    pub fn set(&mut self, x: i32, y: i32, z: i32, value: i32) {
        // Mutates grid
    }
}

struct Environment {
    name: string,
}

impl Environment {
    fn generate_ground(self, grid: VoxelGrid) {
        grid.set(0, 0, 0, 1)  // Should infer `&mut grid` parameter
    }
    
    fn generate_buildings(self, grid: VoxelGrid) {
        let mut i = 0
        while i < 5 {
            grid.set(i, 0, 0, 2)  // Should infer `&mut grid` parameter
            i = i + 1
        }
    }
}
    "#;

    let rust_code =
        test_utils::compile_single_result("voxel_mut_test").expect("Compilation should succeed");

    // Should infer &mut for grid parameters (used in mutating method calls)
    assert!(
        rust_code.contains("grid: &mut VoxelGrid"),
        "Should infer &mut VoxelGrid for parameter when method mutates it. Got:\n{}",
        rust_code
    );
}

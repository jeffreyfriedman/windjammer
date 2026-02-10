/// Bug #18: Tuple field access (.0, .1, .2, etc.)
///
/// When accessing tuple fields using numeric indices,
/// the parser fails with "Expected field or method name"
///
/// This is essential for destructuring and accessing tuple elements.
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn compile_wj_code(code: &str) -> Result<String, String> {
    let temp_dir = tempdir().map_err(|e| e.to_string())?;
    let test_file = temp_dir.path().join("test.wj");

    fs::write(&test_file, code).map_err(|e| e.to_string())?;

    let output = Command::new(get_wj_compiler())
        .args(["build", "--no-cargo"])
        .arg(&test_file)
        .output()
        .map_err(|e| e.to_string())?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstderr: {}\nstdout: {}",
            stderr, stdout
        ));
    }

    Ok(stdout.to_string())
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_first_element() {
    let code = r#"
pub fn get_first() -> i32 {
    let tuple = (1, 2, 3);
    tuple.0
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Tuple .0 access should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_second_element() {
    let code = r#"
pub fn get_second() -> i32 {
    let tuple = (1, 2, 3);
    tuple.1
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Tuple .1 access should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_third_element() {
    let code = r#"
pub fn get_third() -> i32 {
    let tuple = (1, 2, 3);
    tuple.2
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Tuple .2 access should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_in_expression() {
    let code = r#"
pub fn sum_tuple(tuple: (i32, i32, i32)) -> i32 {
    tuple.0 + tuple.1 + tuple.2
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Multiple tuple accesses should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_from_function_return() {
    let code = r#"
pub fn get_coords() -> (i32, i32) {
    (10, 20)
}

pub fn use_coords() -> i32 {
    let coords = get_coords();
    coords.0 + coords.1
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Tuple from function return should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
#[ignore] // Known limitation: nested tuple access requires parentheses: (outer.0).0
fn test_nested_tuple_access() {
    let code = r#"
pub fn nested() -> i32 {
    let outer = ((1, 2), (3, 4));
    outer.0.0 + outer.1.1
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Nested tuple access should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_nested_tuple_access_with_parens() {
    // Workaround: use parentheses for nested tuple access
    let code = r#"
pub fn nested() -> i32 {
    let outer = ((1, 2), (3, 4));
    (outer.0).0 + (outer.1).1
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Nested tuple access with parens should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_as_function_parameter() {
    let code = r#"
pub fn process(data: (i32, i32, i32)) -> i32 {
    data.0 * data.1 + data.2
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Tuple parameter access should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_voxel_coordinate_pattern() {
    // This is the exact pattern we hit in voxel_world.wj
    let code = r#"
pub struct VoxelWorld {
    chunk_size: i32,
}

impl VoxelWorld {
    pub fn world_to_chunk(self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        (
            x / self.chunk_size,
            y / self.chunk_size,
            z / self.chunk_size,
        )
    }
    
    pub fn world_to_local(self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        (
            x % self.chunk_size,
            y % self.chunk_size,
            z % self.chunk_size,
        )
    }
    
    pub fn convert(self, x: i32, y: i32, z: i32) -> (i32, i32, i32, i32, i32, i32) {
        let chunk_pos = self.world_to_chunk(x, y, z);
        let local_pos = self.world_to_local(x, y, z);
        
        (chunk_pos.0, chunk_pos.1, chunk_pos.2, local_pos.0, local_pos.1, local_pos.2)
    }
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Voxel coordinate pattern should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_with_struct_field() {
    let code = r#"
pub struct Point {
    coords: (i32, i32, i32),
}

impl Point {
    pub fn x(self) -> i32 {
        self.coords.0
    }
    
    pub fn y(self) -> i32 {
        self.coords.1
    }
    
    pub fn z(self) -> i32 {
        self.coords.2
    }
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Tuple field on struct should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_tuple_destructuring_alternative() {
    let code = r#"
pub fn destructure() -> i32 {
    let tuple = (1, 2, 3);
    let (a, b, c) = tuple;
    a + b + c
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Tuple destructuring should compile. Error: {:?}",
        result.err()
    );
}

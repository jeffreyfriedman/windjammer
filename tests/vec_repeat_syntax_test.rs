/// Bug #20: vec![item; count] repetition syntax not supported
///
/// Windjammer should support Rust's vec! macro repetition syntax:
/// - `vec![value; count]` creates a vector with `count` copies of `value`
/// - Nested repetition should also work: `vec![vec![x; 3]; 2]`
///
/// This is essential for initializing fixed-size data structures like
/// voxel chunk masks, 2D grids, etc.
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn compile_wj_code(code: &str) -> Result<String, String> {
    let temp_dir = tempdir().map_err(|e| e.to_string())?;
    let test_file = temp_dir.path().join("test.wj");

    fs::write(&test_file, code).map_err(|e| e.to_string())?;

    let output = Command::new("cargo")
        .args(["run", "--release", "--", "build", "--no-cargo"])
        .arg(&test_file)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
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
fn test_simple_vec_repeat() {
    let code = r#"
        fn test() -> Vec<int> {
            let v = vec![0; 5];
            v
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Simple vec![0; 5] should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_vec_repeat_with_expression() {
    let code = r#"
        fn test() -> Vec<int> {
            let size = 10;
            let v = vec![42; size];
            v
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "vec![42; size] with variable should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_vec_repeat_with_cast() {
    let code = r#"
        fn test() -> Vec<int> {
            let size = 16;
            let v = vec![0; size as usize];
            v
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "vec![0; size as usize] should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_nested_vec_repeat() {
    let code = r#"
        fn test() -> Vec<Vec<int>> {
            let inner = vec![0; 3];
            let outer = vec![inner; 2];
            outer
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Nested vec! with repeat should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_nested_vec_repeat_inline() {
    let code = r#"
        fn test() -> Vec<Vec<int>> {
            let grid = vec![vec![0; 3]; 2];
            grid
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Inline nested vec![vec![0; 3]; 2] should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_vec_repeat_with_option_none() {
    let code = r#"
        fn test() -> Vec<Option<int>> {
            let v = vec![None; 10];
            v
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "vec![None; 10] should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_voxel_mask_pattern() {
    let code = r#"
        enum VoxelType {
            Air,
            Stone,
        }
        
        fn create_mask(size: int) -> Vec<Vec<Option<VoxelType>>> {
            let mask = vec![vec![None; size as usize]; size as usize];
            mask
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Voxel mask pattern (nested vec with Option<Enum>) should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_vec_repeat_with_bool() {
    let code = r#"
        fn create_merged_flags(size: int) -> Vec<Vec<bool>> {
            let merged = vec![vec![false; size as usize]; size as usize];
            merged
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Bool grid pattern should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_mixed_vec_syntax() {
    let code = r#"
        fn test() -> Vec<int> {
            let v1 = vec![1, 2, 3];
            let v2 = vec![0; 5];
            let mut result = vec![];
            result.extend(v1);
            result.extend(v2);
            result
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Mixed vec! syntax (list + repeat + empty) should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_vec_repeat_in_struct_field() {
    let code = r#"
        struct Grid {
            data: Vec<Vec<int>>,
        }
        
        fn new_grid(size: int) -> Grid {
            Grid {
                data: vec![vec![0; size as usize]; size as usize],
            }
        }
    "#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "vec! repeat in struct initialization should compile. Error: {:?}",
        result.err()
    );
}

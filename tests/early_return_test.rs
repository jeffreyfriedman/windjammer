/// Bug #13: Early return statement parsing
///
/// When a function has an early return followed by other statements,
/// the parser fails with "Unexpected token: Semicolon"
///
/// This is a common pattern in Rust for guard clauses and error handling.
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
fn test_early_return_with_subsequent_statements() {
    let code = r#"
pub fn check_positive(x: i32) -> bool {
    if x < 0 {
        return false;
    }
    
    // Should be able to continue after early return
    println!("x is positive: {}", x);
    true
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Early return with subsequent statements should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_multiple_early_returns() {
    let code = r#"
pub fn classify(x: i32) -> String {
    if x < 0 {
        return String::from("negative");
    }
    
    if x == 0 {
        return String::from("zero");
    }
    
    String::from("positive")
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Multiple early returns should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_early_return_in_nested_blocks() {
    let code = r#"
pub fn validate(x: i32, y: i32) -> bool {
    if x < 0 {
        if y < 0 {
            return false;
        }
        return true;
    }
    
    x + y > 0
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Early returns in nested blocks should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_early_return_with_variable_assignment() {
    let code = r#"
pub fn compute(x: i32) -> i32 {
    if x < 0 {
        return 0;
    }
    
    let result = x * 2;
    result + 1
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Early return followed by variable assignment should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_early_return_in_match_arm() {
    let code = r#"
pub fn handle(x: Option<i32>) -> i32 {
    match x {
        Some(val) => {
            if val < 0 {
                return 0;
            }
            val * 2
        }
        None => 0,
    }
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Early return in match arm should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_early_return_in_loop() {
    let code = r#"
pub fn find_first_positive(nums: Vec<i32>) -> Option<i32> {
    for num in nums {
        if num > 0 {
            return Some(num);
        }
    }
    None
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Early return in loop should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_early_return_with_method_call() {
    let code = r#"
pub struct Data {
    value: i32,
}

impl Data {
    pub fn process(self) -> i32 {
        if self.value < 0 {
            return 0;
        }
        
        self.value * 2
    }
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Early return in method should compile. Error: {:?}",
        result.err()
    );
}

#[cfg_attr(tarpaulin, ignore)]
#[test]
fn test_voxel_bounds_check_pattern() {
    // This is the exact pattern we hit in voxel_chunk.wj
    let code = r#"
use std::collections::HashMap;

pub struct VoxelChunk {
    size: i32,
    data: HashMap<(i32, i32, i32), i32>,
}

impl VoxelChunk {
    pub fn set_voxel(self, x: i32, y: i32, z: i32, value: i32) {
        if x < 0 || x >= self.size || y < 0 || y >= self.size || z < 0 || z >= self.size {
            return;
        }
        
        self.data.insert((x, y, z), value);
    }
}
"#;

    let result = compile_wj_code(code);
    assert!(
        result.is_ok(),
        "Voxel bounds check pattern should compile. Error: {:?}",
        result.err()
    );
}

/// TDD Test: Vec element type inference in integer inference
/// 
/// Bug: When accessing Vec<u16>[index], the return type should be inferred as u16,
/// not usize (which is the index type).
/// 
/// Example failing code:
/// ```
/// fn get(self) -> u16 {
///     let index = 0 as usize
///     self.data[index]  // Should infer as u16, but infers as usize
/// }
/// ```

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_vec_u16_element_type_inferred_correctly() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    fs::write(&test_file, r#"
struct Container {
    data: Vec<u16>,
}

impl Container {
    fn get_value(self, idx: i32) -> u16 {
        let index = idx as usize
        self.data[index]
    }
}

pub fn main() {
    let mut container = Container { data: Vec::new() }
    container.data.push(42)
    let value = container.get_value(0)
    println!("{}", value)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should NOT have integer inference error about u16 vs usize
    assert!(
        !stderr.contains("Type conflict: must be U16") || !stderr.contains("but was Usize"),
        "Integer inference should recognize Vec<u16>[index] returns u16, not usize!\n\nStderr:\n{}",
        stderr
    );
    
    assert!(
        output.status.success(),
        "Compilation should succeed!\n\nStderr:\n{}",
        stderr
    );
}

#[test]
fn test_vec_u8_element_type_inferred_correctly() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    fs::write(&test_file, r#"
struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    fn read(self, idx: usize) -> u8 {
        self.data[idx]
    }
}

pub fn main() {
    let mut buf = Buffer { data: Vec::new() }
    buf.data.push(255)
    let byte = buf.read(0)
    println!("{}", byte)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should NOT have integer inference error about u8 vs usize
    assert!(
        !stderr.contains("Type conflict: must be U8") || !stderr.contains("but was Usize"),
        "Integer inference should recognize Vec<u8>[index] returns u8, not usize!\n\nStderr:\n{}",
        stderr
    );
    
    assert!(
        output.status.success(),
        "Compilation should succeed!\n\nStderr:\n{}",
        stderr
    );
}

#[test]
fn test_vec_i32_element_type_inferred_correctly() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    fs::write(&test_file, r#"
struct Numbers {
    values: Vec<i32>,
}

impl Numbers {
    fn get(self, idx: usize) -> i32 {
        self.values[idx]
    }
}

pub fn main() {
    let mut nums = Numbers { values: Vec::new() }
    nums.values.push(-42)
    let num = nums.get(0)
    println!("{}", num)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should NOT have integer inference error
    assert!(
        !stderr.contains("Type conflict: must be I32") || !stderr.contains("but was Usize"),
        "Integer inference should recognize Vec<i32>[index] returns i32, not usize!\n\nStderr:\n{}",
        stderr
    );
    
    assert!(
        output.status.success(),
        "Compilation should succeed!\n\nStderr:\n{}",
        stderr
    );
}

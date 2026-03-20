/// TDD Test: Binary operation integer type propagation
/// 
/// Bug: When a binary operation involves a typed value (u64) and an unsuffixed literal (60),
/// the literal should be inferred as matching the typed value's type (u64), not default to i32.
/// 
/// Example failing code:
/// ```
/// struct Counter {
///     count: u64,
/// }
/// 
/// impl Counter {
///     fn is_milestone(self) -> bool {
///         self.count % 60 == 0  // 60 should be u64, not i32!
///     }
/// }
/// ```

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_u64_modulo_literal_infers_u64() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    fs::write(&test_file, r#"
struct Counter {
    count: u64
}

impl Counter {
    fn check(self) {
        if self.count % 60 == 0 {
            println("Milestone!")
        }
    }
}

pub fn main() {
    let c = Counter { count: 120 }
    c.check()
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should succeed with no integer inference errors
    assert!(
        !stderr.contains("Int inference error"),
        "Should not have integer inference errors!\n\nStderr:\n{}",
        stderr
    );
    
    assert!(
        output.status.success(),
        "Build should SUCCEED with correct type inference!\n\nStderr:\n{}",
        stderr
    );
    
    // Verify generated code has correct suffixes
    let build_dir = temp_dir.path().join("build");
    let generated = fs::read_to_string(build_dir.join("test.rs")).unwrap();
    
    assert!(
        generated.contains("60_u64") && generated.contains("0_u64"),
        "Generated code should use u64 suffixes for both literals!\nFound:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("% 60") || l.contains("== 0"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_u32_comparison_literal_infers_u32() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    fs::write(&test_file, r#"
struct Timer {
    elapsed: u32
}

impl Timer {
    fn expired(self) -> bool {
        self.elapsed > 100
    }
}

pub fn main() {
    let t = Timer { elapsed: 150 }
    let result = t.expired()
    println("{}", result)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should succeed with no integer inference errors
    assert!(
        !stderr.contains("Int inference error"),
        "Should not have integer inference errors!\n\nStderr:\n{}",
        stderr
    );
    
    assert!(
        output.status.success(),
        "Build should SUCCEED with correct type inference!\n\nStderr:\n{}",
        stderr
    );
    
    // Verify generated code has correct suffixes
    let build_dir = temp_dir.path().join("build");
    let generated = fs::read_to_string(build_dir.join("test.rs")).unwrap();
    
    assert!(
        generated.contains("100_u32") || generated.contains("100u32"),
        "Generated code should use u32 suffix for comparison literal!\nFound:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("elapsed"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_u16_arithmetic_literal_infers_u16() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    
    fs::write(&test_file, r#"
struct SmallCounter {
    value: u16
}

impl SmallCounter {
    fn increment(self) {
        self.value = self.value + 1
    }
    
    fn reset(self) {
        self.value = 0
    }
}

pub fn main() {
    let mut c = SmallCounter { value: 5 }
    c.increment()
    println("{}", c.value)
}
"#).unwrap();
    
    let mut cmd = Command::cargo_bin("wj").unwrap();
    let output = cmd
        .current_dir(temp_dir.path())
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should succeed with no integer inference errors
    assert!(
        !stderr.contains("Int inference error"),
        "Should not have integer inference errors!\n\nStderr:\n{}",
        stderr
    );
    
    assert!(
        output.status.success(),
        "Build should SUCCEED with correct type inference!\n\nStderr:\n{}",
        stderr
    );
    
    // Verify generated code has correct suffixes
    let build_dir = temp_dir.path().join("build");
    let generated = fs::read_to_string(build_dir.join("test.rs")).unwrap();
    
    assert!(
        generated.contains("1_u16") || generated.contains("1u16"),
        "Generated code should use u16 suffix for arithmetic literal!\nFound:\n{}",
        generated
            .lines()
            .filter(|l| l.contains("value") && l.contains("+ 1"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

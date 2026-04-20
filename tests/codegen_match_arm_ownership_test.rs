/// TDD Test: Match arm should produce owned value, not borrowed
///
/// Bug: When matching on an enum, the bound variable in match arms is being
/// transpiled as a borrow (&T) instead of owned (T), causing E0606 cast errors.
///
/// Example:
/// ```windjammer
/// match value {
///     Value::Int(v) => v as f32,  // ERROR: can't cast &i32 to f32
/// }
/// ```
///
/// The compiler should recognize that `v` is owned (moved from enum variant),
/// not borrowed, allowing the cast to work.
use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_produces_owned_value_not_borrowed() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_match_owned_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let test_content = r#"
enum Value {
    Int(i32),
    Float(f32),
}

impl Value {
    pub fn as_float(self) -> f32 {
        match self {
            Value::Int(v) => v as f32,
            Value::Float(v) => v,
        }
    }
}

fn main() {
    let val = Value::Int(42)
    let f = val.as_float()
    println!("Float: {}", f)
}
"#;

    fs::write(test_dir.join("match_owned.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("match_owned.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let rust_file = test_dir.join("build").join("match_owned.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_default();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // The generated code should NOT have `&i32` in match arms
    assert!(
        !rust_code.contains("Value::Int(ref v)") && !rust_code.contains("Value::Int(&v)"),
        "Match arm should bind owned value, not borrow.\nGenerated:\n{}",
        rust_code
    );

    // The key assertion: generated code should have `pub fn as_float(self)`, not `&self`
    assert!(
        rust_code.contains("pub fn as_float(self)"),
        "Method signature should be `as_float(self)` (owned), not `&self`.\nGenerated:\n{}",
        rust_code
    );

    // And the match should be on `self`, not `&self`
    assert!(
        rust_code.contains("match self {"),
        "Match should be on `self` (owned), not `&self`.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arm_ownership_with_string() {
    let test_dir = std::env::temp_dir().join(format!(
        "wj_match_string_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let test_content = r#"
enum Message {
    Text(String),
    Number(i32),
}

impl Message {
    pub fn get_length(self) -> usize {
        match self {
            Message::Text(s) => s.len(),
            Message::Number(n) => 0,
        }
    }
}

fn main() {
    let msg = Message::Text("hello".to_string())
    let len = msg.get_length()
    println!("Length: {}", len)
}
"#;

    fs::write(test_dir.join("match_string.wj"), test_content).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg("match_string.wj")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    let rust_file = test_dir.join("build").join("match_string.rs");
    let rust_code = fs::read_to_string(&rust_file).unwrap_or_default();

    // Cleanup
    let _ = fs::remove_dir_all(&test_dir);

    // Method should consume self (owned)
    assert!(
        rust_code.contains("pub fn get_length(self)"),
        "Method signature should be `get_length(self)` (owned).\nGenerated:\n{}",
        rust_code
    );

    // Match should be on owned self
    assert!(
        rust_code.contains("match self {"),
        "Match should be on `self` (owned).\nGenerated:\n{}",
        rust_code
    );
}

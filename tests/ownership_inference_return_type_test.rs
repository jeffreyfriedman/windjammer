/// TDD: Test that ownership inference infers Owned when return type matches parameter type.
///
/// Bug: save_migration.wj - migrate(data: GameSaveData) -> Result<GameSaveData, string>
/// was incorrectly inferring &GameSaveData because we only read data fields.
/// When returning the same type (directly or wrapped in Result/Option), we need owned.
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
fn test_owned_when_returned_same_type() {
    // When a function returns the same type as a parameter,
    // that parameter should be owned, not borrowed.
    let source = r#"
pub fn transform(data: Data) -> Result<Data, string> {
    let mut result = data
    result.value = result.value + 1
    Ok(result)
}

struct Data {
    value: i32,
}
"#;

    let rust = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Should generate owned parameter, not &Data
    assert!(
        rust.contains("pub fn transform(data: Data)"),
        "Should infer owned when returning same type.\n\nGenerated:\n{}",
        rust
    );
    assert!(
        !rust.contains("pub fn transform(data: &Data)"),
        "Should NOT infer &Data when returning same type.\n\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_borrowed_when_not_returned() {
    // When a function doesn't return the parameter type,
    // borrowing is fine.
    let source = r#"
pub fn get_value(data: Data) -> i32 {
    data.value
}

struct Data {
    value: i32,
}
"#;

    let rust = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Can be borrowed since we're only reading (Data is not Copy - struct with i32)
    assert!(
        rust.contains("pub fn get_value(data: &Data)") || rust.contains("pub fn get_value(data: Data)"),
        "Should infer borrowed or owned when not returning param type.\n\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_owned_when_wrapped_in_result() {
    // Result<T, E> counts as returning T
    let source = r#"
pub fn migrate(data: GameSaveData) -> Result<GameSaveData, string> {
    if data.version < 2 {
        return Err("Too old".to_string())
    }
    Ok(data)
}

struct GameSaveData {
    version: i32,
}
"#;

    let rust = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    assert!(
        rust.contains("pub fn migrate(data: GameSaveData)"),
        "Should infer owned when returning Result<param_type, _>.\n\nGenerated:\n{}",
        rust
    );
    assert!(
        !rust.contains("pub fn migrate(data: &GameSaveData)"),
        "Should NOT infer &GameSaveData when returning Result.\n\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_owned_when_wrapped_in_option() {
    // Option<T> counts as returning T
    let source = r#"
pub fn validate(data: Config) -> Option<Config> {
    if data.valid {
        Some(data)
    } else {
        None
    }
}

struct Config {
    valid: bool,
}
"#;

    let rust = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    assert!(
        rust.contains("pub fn validate(data: Config)"),
        "Should infer owned when returning Option<param_type>.\n\nGenerated:\n{}",
        rust
    );
    assert!(
        !rust.contains("pub fn validate(data: &Config)"),
        "Should NOT infer &Config when returning Option.\n\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_borrowed_when_cloned_internally() {
    // If we see .clone() on the parameter, borrowing is OK
    let source = r#"
pub fn duplicate(data: Data) -> Data {
    data.clone()
}

struct Data {
    value: i32,
}
"#;

    let rust = match compile_to_rust(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Can be borrowed since we're cloning
    assert!(
        rust.contains("pub fn duplicate(data: &Data)") || rust.contains("pub fn duplicate(data: Data)"),
        "Should infer borrowed or owned when cloning.\n\nGenerated:\n{}",
        rust
    );
}

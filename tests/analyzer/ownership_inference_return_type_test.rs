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

/// TDD: Test that ownership inference infers Owned when return type matches parameter type.
///
/// Bug: save_migration.wj - migrate(data: GameSaveData) -> Result<GameSaveData, string>
/// was incorrectly inferring &GameSaveData because we only read data fields.
/// When returning the same type (directly or wrapped in Result/Option), we need owned.
#[path = "../common/test_utils.rs"]
mod test_utils;

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

    let rust = match test_utils::compile_single_result(source) {
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

    let rust = match test_utils::compile_single_result(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Can be borrowed since we're only reading (Data is not Copy - struct with i32)
    assert!(
        rust.contains("pub fn get_value(data: &Data)")
            || rust.contains("pub fn get_value(data: Data)"),
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

    let rust = match test_utils::compile_single_result(source) {
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

    let rust = match test_utils::compile_single_result(source) {
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

    let rust = match test_utils::compile_single_result(source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation failed: {}", e),
    };

    // Can be borrowed since we're cloning
    assert!(
        rust.contains("pub fn duplicate(data: &Data)")
            || rust.contains("pub fn duplicate(data: Data)"),
        "Should infer borrowed or owned when cloning.\n\nGenerated:\n{}",
        rust
    );
}

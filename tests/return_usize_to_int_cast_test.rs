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

// TDD Test: Compiler should auto-cast usize to i64 in return statements
// Functions returning int should accept .len() (usize) without explicit casts

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_vec_len_should_cast_to_int() {
    // BUG: Compiler doesn't auto-cast .len() to i64 in return
    let code = r#"
    pub fn get_length(items: Vec<i32>) -> int {
        return items.len()
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should auto-cast .len() (usize) to i64
    assert!(
        generated.contains("items.len() as i64") || generated.contains("(items.len() as i64)"),
        "Should auto-cast .len() to i64 when returning int, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_len_from_method() {
    // Real case from components.rs
    let code = r#"
    pub struct ComponentArray {
        pub dense: Vec<i32>,
    }
    
    impl ComponentArray {
        pub fn len(&self) -> int {
            return self.dense.len()
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should auto-cast
    assert!(
        generated.contains("self.dense.len() as i64")
            || generated.contains("(self.dense.len() as i64)"),
        "Should auto-cast .len() to i64, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_implicit_return_len_should_cast() {
    // Test implicit returns (no return keyword)
    let code = r#"
    pub fn count(items: Vec<i32>) -> int {
        items.len()
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should auto-cast implicit return
    assert!(
        generated.contains("items.len() as i64"),
        "Should auto-cast implicit return of .len() to i64, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_usize_variable_to_int() {
    // When a usize variable is returned as int
    let code = r#"
    pub fn process(items: Vec<i32>) -> int {
        let count: usize = items.len()
        return count
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should auto-cast usize variable to i64 (explicit or implicit return)
    assert!(
        generated.contains("count as i64"),
        "Should auto-cast usize variable to i64, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_return_computed_usize_to_int() {
    // Return expression with usize operations
    let code = r#"
    pub fn get_half_length(items: Vec<i32>) -> int {
        return items.len() / 2
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should cast the entire expression
    assert!(
        generated.contains("as i64") || generated.contains("as usize"),
        "Should handle usize arithmetic in return, got:\n{}",
        generated
    );
}

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

// TDD Test: Compiler incorrectly adds 'as i32' casts to .len() in comparisons
// This test should FAIL until the bug is fixed

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_len_comparison_should_not_cast_to_i32() {
    // BUG: Compiler incorrectly adds (len as i32) when comparing with usize variable
    let code = r#"
    pub fn check_bounds(items: Vec<i32>, index: usize) -> bool {
        return items.len() > index
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT cast .len() to i32 when comparing with usize
    assert!(
        !generated.contains("(items.len() as i32)") && !generated.contains(".len() as i32"),
        "Should NOT cast .len() to i32 when comparing with usize variable, got:\n{}",
        generated
    );

    // Should either keep both as usize or cast the usize variable
    assert!(
        generated.contains("items.len() > index")
            || generated.contains("items.len() > (index as i64)"),
        "Should compare without incorrect casts, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_sparse_vec_len_comparison_with_usize() {
    // Real-world case from components.rs
    let code = r#"
    pub struct ComponentArray {
        pub sparse: Vec<i64>,
    }
    
    impl ComponentArray {
        pub fn contains(&self, entity_idx: usize) -> bool {
            if self.sparse.len() <= entity_idx {
                return false
            }
            return true
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT cast .len() to i32
    assert!(
        !generated.contains("(self.sparse.len() as i32)"),
        "Should NOT cast .len() to i32, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_usize_variable_in_comparison_keeps_type() {
    // Ensure usize variables stay usize in comparisons
    let code = r#"
    pub fn process(data: Vec<i32>) {
        let len: usize = data.len()
        let idx: usize = 5
        
        if len > idx {
            println("valid")
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT add any i32 casts to usize comparisons
    assert!(
        !generated.contains("as i32") || !generated.contains("len") && !generated.contains("idx"),
        "Should NOT cast usize variables to i32 in comparisons, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_len_in_while_loop_condition() {
    // Test .len() in while loop conditions
    let code = r#"
    pub fn iterate(items: Vec<i32>) {
        let mut i: usize = 0
        while i < items.len() {
            i = i + 1
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT cast .len() to i32 when comparing with usize
    assert!(
        !generated.contains("(items.len() as i32)"),
        "Should NOT cast .len() to i32 in while loop, got:\n{}",
        generated
    );
}

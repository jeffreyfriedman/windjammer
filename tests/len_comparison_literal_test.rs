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

// TDD Test: Literal comparisons with .len() should use usize
//
// Bug: if vec.len() > 0 generates len() > 0_i32 → type error
// Rust: usize > i32 = type mismatch
//
// Fix: Infer literals as usize when compared with .len()

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_len_comparison_with_zero() {
    let test_wj = r#"
fn has_items(items: Vec<i32>) -> bool {
    if items.len() > 0 {
        return true
    }
    false
}
"#;

    let rust_code = test_utils::compile_single(test_wj);

    println!("Generated Rust:\n{}", rust_code);

    // Should generate 0_usize or just 0 (Rust infers from context)
    // Should NOT generate 0_i32
    assert!(
        !rust_code.contains("0_i32"),
        "Should NOT generate i32 literal in len() comparison\nGenerated:\n{}",
        rust_code
    );

    println!("✅ len() > 0 test PASSED");
}

#[test]
fn test_len_comparison_with_constant() {
    let test_wj = r#"
fn is_valid_team(team: Vec<string>) -> bool {
    team.len() >= 2
}
"#;

    let rust_code = test_utils::compile_single(test_wj);

    println!("Generated Rust:\n{}", rust_code);

    // Should NOT generate i32 literals
    assert!(
        !rust_code.contains("2_i32"),
        "Should NOT generate i32 literal in len() comparison\nGenerated:\n{}",
        rust_code
    );

    println!("✅ len() >= 2 test PASSED");
}

#[test]
fn test_len_assignment_to_usize() {
    let test_wj = r#"
struct Animation {
    current_frame_index: usize
}

impl Animation {
    fn reset(self) {
        self.current_frame_index = 0
    }
}
"#;

    let rust_code = test_utils::compile_single(test_wj);

    println!("Generated Rust:\n{}", rust_code);

    // When assigning to usize field, literal should be usize
    assert!(
        rust_code.contains("0_usize") || !rust_code.contains("0_i32"),
        "Should generate usize literal for usize field\nGenerated:\n{}",
        rust_code
    );

    println!("✅ usize field assignment test PASSED");
}

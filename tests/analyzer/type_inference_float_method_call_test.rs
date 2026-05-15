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

// TDD Test: Float literal inference with method call returning f32
//
// Bug: self.get_cost() * 1.414 generates 1.414_f64 instead of 1.414_f32
// Expected: Binary op with f32 method return should constrain literal to f32
//
// Dogfooding Win: This is a real bug found in astar_grid.wj

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_float_literal_in_binary_op_with_method_return() {
    let wj_source = r#"
struct Grid {
    pub cost: f32,
}

impl Grid {
    fn get_cost(self) -> f32 {
        self.cost
    }
    
    fn scaled_cost(self) -> f32 {
        self.get_cost() * 1.414
    }
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    eprintln!("Generated Rust:\n{}", rust_code);

    // The literal 1.414 in `self.get_cost() * 1.414` should be f32 (from get_cost: f32)
    assert!(
        !rust_code.contains("1.414_f64"),
        "1.414 should NOT be f64 when multiplying f32 method return, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("1.414_f32") || rust_code.contains("1.414f32"),
        "1.414 should be f32 when multiplying f32 method return, got:\n{}",
        rust_code
    );
}

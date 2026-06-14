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

//! TDD Test: Methods using self.field in binary ops should be &self, not self
//!
//! When a method reads self.field in arithmetic expressions like `x + self.offset`,
//! the method should be inferred as `&self` (borrowed), not `self` (owned).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_self_field_in_binary_op_should_borrow() {
    // Use a non-Copy type (has String field) to avoid Copy type special casing
    let code = r#"
pub struct Editor {
    name: string,
    offset_x: f32,
    offset_y: f32,
}

impl Editor {
    pub fn translate_x(self, x: f32) -> f32 {
        x + self.offset_x
    }
    
    pub fn translate_point(self, x: f32, y: f32) -> (f32, f32) {
        (x + self.offset_x, y + self.offset_y)
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Compile error:\n{}", err);
    }

    assert!(success, "Compilation should succeed");

    // Methods should have &self, not self
    assert!(
        generated.contains("fn translate_x(&self"),
        "translate_x should be &self. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn translate_point(&self"),
        "translate_point should be &self. Generated:\n{}",
        generated
    );
}

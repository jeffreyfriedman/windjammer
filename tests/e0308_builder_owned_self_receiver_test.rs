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

//! E0308: `expected VoxelScene, found &VoxelScene` when fluent methods use `let mut result = self`
//! and the analyzer inferred `&self` for an explicit consuming `self` parameter.
//!
//! Rust receiver must be `mut self` so `result` is owned and matches the declared return type.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_fluent_method_owned_self_emits_mut_self_when_returning_impl_struct() {
    let source = r#"
pub struct Widget {
    count: i32,
}

impl Widget {
    pub fn with_count(self, n: i32) -> Widget {
        let mut result = self
        result.count = n
        result
    }
}
"#;

    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("fn with_count(mut self"),
        "expected mut self for builder-style return; got:\n{rust}"
    );
    assert!(
        !rust.contains("fn with_count(&self"),
        "must not emit &self when returning owned struct via result binding; got:\n{rust}"
    );
}

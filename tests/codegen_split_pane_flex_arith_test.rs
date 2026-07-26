#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! FAILING REPRO: arithmetic in format args for SplitPanel right pane flex.
//!
//! Desired: `let right_flex = 100 - self.initial_size` then interpolate,
//! not inline `100 - self.initial_size` that may mis-codegen for Copy i32 fields.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn split_pane_flex_arithmetic_should_bind_local() {
    let source = r#"
pub struct SplitPanel {
    pub initial_size: int,
    pub left: string,
    pub right: string,
}

impl SplitPanel {
    pub fn render(self) -> string {
        let right_flex = 100 - self.initial_size
        format!("L{} R{}", self.initial_size, right_flex)
    }
}

fn main() {
    let p = SplitPanel { initial_size: 28, left: "a".to_string(), right: "b".to_string() }
    let _ = p.render()
}
"#;

    let result = test_utils::compile_single(source);
    let has_local = result.contains("let right_flex")
        || result.contains("right_flex =");
    let uses_both = result.contains("initial_size") && (result.contains("right_flex") || result.contains("100"));

    assert!(
        has_local && uses_both && !result.contains("error"),
        "codegen should bind right_flex = 100 - initial_size for split panes. Got:\n{}",
        result
    );
}

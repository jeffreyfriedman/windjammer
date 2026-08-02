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

//! Gate: owned String literals / temps must not be passed as `&String`.
//!
//! Seen in windjammer-ui generated Spacer helpers:
//!   Spacer::vertical(&"4px".to_string())  // E0308 expected String, found &String
//!
//! Expected: pass owned String (`"4px".to_string()`) when the param type is String.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn string_arg_to_owned_param_should_not_add_shared_ref() {
    let source = r#"
pub struct Spacer {
    pub height: string,
}

impl Spacer {
    pub fn vertical(height: string) -> Spacer {
        Spacer { height: height }
    }

    pub fn xxs() -> Spacer {
        Spacer::vertical("4px")
    }
}

fn main() {
    let _ = Spacer::xxs()
}
"#;

    let result = test_utils::compile_single(source);

    assert!(
        !result.contains("vertical(&") && !result.contains("vertical(&\""),
        "codegen must not pass &String into owned string param. Got:\n{}",
        result
    );
    assert!(
        result.contains("vertical(\"4px\"")
            || result.contains("vertical(\"4px\".to_string()")
            || result.contains("vertical(String::from(\"4px\")"),
        "expected owned string arg to Spacer::vertical. Got:\n{}",
        result
    );
}

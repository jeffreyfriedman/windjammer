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

//! TDD Test: Methods that don't access self at all should still be &self
//!
//! When a method doesn't access self.field at all, it should still be &self
//! since there's no reason to consume it.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_no_self_access_should_borrow() {
    // Method that doesn't access self at all should be &self
    let code = r#"
pub struct Helper {
    data: string,
}

impl Helper {
    pub fn format_label(self, label: string) -> string {
        format!("Label: {}", label)
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

    // Method should have &self, not self - even though it doesn't use self
    assert!(
        generated.contains("fn format_label(&self"),
        "format_label should be &self. Generated:\n{}",
        generated
    );
}

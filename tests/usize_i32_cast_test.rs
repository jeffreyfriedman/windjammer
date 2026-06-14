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

//! TDD Test: Auto-cast between usize and int in comparisons and assignments
//!
//! When comparing/assigning int variable with .len() (usize), auto-cast.
//! Note: Windjammer's `int` type maps to `i64` in Rust.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_i32_compare_with_len() {
    // int compared with .len() should auto-cast
    let code = r#"
pub struct Container {
    items: Vec<i32>,
    selected: int,
}

impl Container {
    pub fn check_bounds(self) -> bool {
        let count = self.items.len()
        if self.selected >= count {
            return false
        }
        true
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_i32_assign_from_len() {
    // Assigning len() result to int should auto-cast
    let code = r#"
pub struct Container {
    items: Vec<i32>,
    count: int,
}

impl Container {
    pub fn update_count(self) {
        self.count = self.items.len() - 1
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

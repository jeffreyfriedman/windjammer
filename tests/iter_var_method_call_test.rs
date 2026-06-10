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

// TDD: Iteration variable method calls and comparisons
//
// Tests that iteration variables work correctly in method calls
// and comparisons without needing Rust-specific .as_str() calls.
// Windjammer infers string types and comparisons automatically.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_string_comparison() {
    // Idiomatic Windjammer: compare iteration variable directly to string literal
    let code = r#"
    pub fn process_strings(items: Vec<string>) -> Vec<bool> {
        let mut results = Vec::new()
        for item in items {
            let matches = item == "test"
            results.push(matches)
        }
        return results
    }
    "#;
    let generated = test_utils::compile_single(code);
    // The comparison should be clean, no .as_str() needed
    assert!(
        !generated.contains(".as_str()"),
        "Should not generate .as_str() - Windjammer handles string comparison automatically: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_comparison_with_struct_field() {
    // Idiomatic Windjammer: compare iteration variable with struct field
    let code = r#"
    struct ThemeSwitcher {
        themes: Vec<string>,
        current_theme: string,
    }
    impl ThemeSwitcher {
        pub fn render(self) -> string {
            let mut output = ""
            for t in self.themes {
                let selected = if t == self.current_theme { "selected" } else { "" }
                output.push_str(selected)
            }
            return output
        }
    }
    "#;
    let generated = test_utils::compile_single(code);
    // No .as_str(), no type mismatch errors
    assert!(
        !generated.contains(".as_str()"),
        "Should not generate .as_str(): {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iter_var_method_call_on_string() {
    // Iteration variable should support string method calls like .len(), .trim()
    let code = r#"
    pub fn count_long_strings(items: Vec<string>, min_len: i32) -> i32 {
        let mut count = 0
        for item in items {
            if item.len() as i32 > min_len {
                count = count + 1
            }
        }
        return count
    }
    "#;
    let generated = test_utils::compile_single(code);
    assert!(
        generated.contains(".len()"),
        "Should generate .len() method call on iteration variable: {}",
        generated
    );
}

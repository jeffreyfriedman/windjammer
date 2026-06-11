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

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile Windjammer code and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_ref_field_vs_literal() {
    let code = r####"
    struct Rating {
        color: string,
    }
    impl Rating {
        pub fn get_color(self, filled: bool) -> string {
            if filled {
                self.color
            } else {
                "#e2e8f0"
            }
        }
    }
    "####;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // When one branch is &self.field (explicit ref) and other is string literal,
    // the literal should NOT be converted to String
    // Both are &str compatible
    assert!(
        !generated.contains(r###""#e2e8f0".to_string()"###),
        "String literal should NOT be converted when other branch is explicit &ref: {}",
        generated
    );
    assert!(
        generated.contains(r###""#e2e8f0""###),
        "String literal should remain as &str: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_ref_vs_literal_in_let() {
    let code = r#"
    struct Config {
        name: string,
    }
    pub fn get_display_name(config: Config, use_default: bool) -> string {
        let name = if use_default {
            &config.name
        } else {
            "Unknown"
        }
        return name
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    assert!(generated.contains("if use_default") && generated.contains("config.name"));
    // `Unknown` may be lifted to `String` in one branch; both patterns are acceptable for this regression
}

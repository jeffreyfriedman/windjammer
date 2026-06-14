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

/// TDD: Extern functions expecting &T should auto-borrow owned arguments
///
/// Bug: When calling extern fn json::get(value: &Value, key: &str),
/// the compiler generates json::get(v, "key") instead of json::get(&v, "key").
/// The owned `v: Value` needs to be auto-borrowed to `&v: &Value`.
///
/// Dogfooding source: playtest-mcp calling json::get and json::stringify
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_extern_fn_ref_param_auto_borrow() {
    let source = r#"
use std::json

pub fn parse_field(line: string) -> string {
    match json::parse(line) {
        Ok(v) => {
            if let Some(f) = json::get(v, "field") {
                if let Some(s) = json::as_str(f) {
                    return s
                }
            }
            return ""
        }
        Err(_) => return ""
    }
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // json::get expects &Value. When v is an owned Value (from json::parse),
    // the compiler should auto-borrow: json::get(&v, "field")
    assert!(
        rust.contains("json::get(&v,") || rust.contains("json::get(& v,"),
        "Expected json::get(&v, ...) but got:\n{}",
        rust
    );
}

#[test]
fn test_extern_fn_stringify_auto_borrow() {
    let source = r#"
use std::json

pub fn to_str(n: int) -> string {
    let val = json::number_i64(n as i64)
    match json::stringify(val) {
        Ok(s) => s,
        Err(_) => "0",
    }
}
"#;
    let (rust, _stderr) = test_utils::compile_via_cli_with_stderr(source);

    // json::stringify expects &Value. val is an owned Value.
    // Compiler should auto-borrow: json::stringify(&val)
    assert!(
        rust.contains("json::stringify(&val") || rust.contains("json::stringify(& val"),
        "Expected json::stringify(&val) but got:\n{}",
        rust
    );
}

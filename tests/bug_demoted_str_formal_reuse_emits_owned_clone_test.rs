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
    feature = "integration_tests",
))]

//! P3.225 (`wj-scheduler` crontab): demoted `string`→`&str` formal + later reuse
//! of the same binding emits `arg.clone()` (owned String) at the call site → E0308.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str = r#"
use std::strings

pub fn keep_if_nonempty(expr: string) -> Result<string, string> {
    match peek(expr) {
        Ok(_) => Ok(expr),
        Err(e) => Err(e),
    }
}

fn peek(text: string) -> Result<int, string> {
    if strings.len(text) == 0 {
        return Err("empty")
    }
    Ok(strings.len(text) as int)
}
"#;

#[test]
fn demoted_str_formal_reuse_must_borrow_not_owned_clone() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let peek_borrowed = rs.contains("fn peek(text: &str)") || rs.contains("fn peek(text:&str)");
    let call_owned_clone = rs.contains("peek(expr.clone())") || rs.contains("peek(text.clone())");
    assert!(
        !(peek_borrowed && call_owned_clone),
        "RED: demoted &str formal must not be called with expr.clone() (E0308). Got:\n{rs}"
    );
    assert!(
        ok,
        "RED: fixture must cargo-check after borrow fix. Generated:\n{rs}"
    );
}

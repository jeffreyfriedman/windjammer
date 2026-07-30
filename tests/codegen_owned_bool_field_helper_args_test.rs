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

//! FAILING REPRO (dogfood): bool field accesses must not be auto-borrowed
//! into `bool` helper parameters (`&bool` vs `bool`).
//! Workaround in product code: bind `let balanced = summary.balanced` then pass `balanced`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn bool_field_helper_arg_should_not_auto_borrow() {
    let source = r##"
pub struct ReconciliationSummary {
    balanced: bool,
}

pub fn validate_reconciliation_finish(balanced: bool) -> Result<(), string> {
    if !balanced {
        return Err("reconciliation is not balanced".to_string())
    }
    Ok(())
}

fn main() {
    let summary = ReconciliationSummary { balanced: true }
    match validate_reconciliation_finish(summary.balanced) {
        Ok(_) => println!("ok"),
        Err(msg) => println!("{}", msg),
    }
}
"##;

    let result = test_utils::compile_single(source);
    let ok = result.contains("ok")
        && !result.contains("error[E0308]")
        && !result.contains("expected `bool`, found `&bool`");
    assert!(
        ok,
        "bool field helper args should codegen without & borrow. Got:\n{}",
        result
    );
}

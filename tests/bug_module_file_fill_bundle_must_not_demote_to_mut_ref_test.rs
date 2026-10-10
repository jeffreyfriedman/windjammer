#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! `fill_bundle_from_six` must not demote fills to `&mut OptOperatorFill`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multipass_fill_bundle_must_keep_owned_fills() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod fills
pub mod bundle
"#,
    );
    test.add_file(
        "fills.wj",
        r#"
pub struct OptOperatorFill {
    pub done: bool,
}

pub fn opt_operator_fill_is_complete(fill: OptOperatorFill) -> bool {
    fill.done
}
"#,
    );
    test.add_file(
        "bundle.wj",
        r#"
use crate::fills::{OptOperatorFill, opt_operator_fill_is_complete}

pub fn fill_bundle_from_six(
    q1: OptOperatorFill,
    q6: OptOperatorFill,
) -> bool {
    let a = opt_operator_fill_is_complete(q1)
    let b = opt_operator_fill_is_complete(q6)
    a && b
}
"#,
    );
    let map = test.compile().expect("multipass");
    let bundle = map.get("bundle.rs").expect("bundle.rs");
    eprintln!("bundle.rs:\n{bundle}");
    assert!(
        !bundle.contains("q1: &mut OptOperatorFill")
            && !bundle.contains("q6: &mut OptOperatorFill"),
        "RED multipass demotes fill_bundle formals to &mut. Got:\n{bundle}"
    );
    assert!(
        bundle.contains("q1: OptOperatorFill") && bundle.contains("q6: OptOperatorFill"),
        "formals must stay owned OptOperatorFill. Got:\n{bundle}"
    );
}

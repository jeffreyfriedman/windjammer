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

//! When the same owned param is passed to two owned-`String` formals, the compiler
//! must emit `.clone()` on the first use (or all but last).
//!
//! Dogfood (`hub.wj`): `own_title(title)` then `own_title(title)` — both move `String`.
//! (Helpers that only call `&str` methods demote to `&str` under Phase 2; this gate
//! uses identity moves so the formals stay owned.)

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multi_use_owned_param_must_clone_on_first_use() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "html.wj",
        r#"
pub fn own_title(s: string) -> string {
    s
}
pub fn crumbs_for(title: string) -> string {
    own_title(title)
}
"#,
    );
    test.add_file(
        "hub.wj",
        r#"
use crate::html::{own_title, crumbs_for}

pub fn render_panel(title: string, blurb: string) -> string {
    let crumbs = crumbs_for(title)
    let title_e = own_title(title)
    let blurb_e = own_title(blurb)
    crumbs + title_e + blurb_e
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let rs = map.get("hub.rs").expect("hub.rs output");
    let html = map.get("html.rs").expect("html.rs output");

    assert!(
        html.contains("s: String"),
        "repro needs owned formals (identity move). Got:\n{html}"
    );

    // Must either clone title before first use, or emit title.clone() somewhere
    let has_clone = rs.contains("title.clone()") || rs.contains(".clone(),");
    assert!(
        has_clone,
        "multi-use owned param must auto-clone to avoid E0382. Got:\n{rs}"
    );

    test.cargo_check()
        .expect("multi-use owned param must cargo check");
}

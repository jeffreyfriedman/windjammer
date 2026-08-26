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

//! FAILING REPRO — when the same owned param is passed to two owned-`String` formals,
//! the compiler must emit `.clone()` on the first use (or all but last).
//!
//! Dogfood (`hub.wj`): `crumbs_for(title)` then `escape_html(title)` — both own `String`.
//! Tip currently emits bare `title` twice → E0382 use-after-move.
//! Product workaround: `+ ""` to re-own before each call.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn multi_use_owned_param_must_clone_on_first_use() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "html.wj",
        r#"
pub fn escape_html(s: string) -> string {
    s.replace("&", "&amp;").replace("<", "&lt;")
}
pub fn crumbs_for(title: string) -> string {
    escape_html(title)
}
"#,
    );
    test.add_file(
        "hub.wj",
        r#"
use crate::html::{escape_html, crumbs_for}

pub fn render_panel(title: string, blurb: string) -> string {
    let crumbs = crumbs_for(title)
    let title_e = escape_html(title)
    let blurb_e = escape_html(blurb)
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
        "repro needs owned formals. Got:\n{html}"
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

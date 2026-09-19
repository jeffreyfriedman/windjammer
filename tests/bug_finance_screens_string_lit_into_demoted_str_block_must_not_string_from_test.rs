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

//! P3.376: finance-screens `render_post_to_gl_block` — string lits / owned locals into
//! demoted `&str` formals must not emit `String::from("…")` / bare `String` (tip E0308).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod panels
"#;

const PANELS: &str = r##"
fn render_post_to_gl_block(
    kind_label: string,
    demo_id: string,
    post_path: string,
    body_id: string,
    button_id: string,
    refresh_sel: string,
    token: string,
) -> string {
    // Readonly uses only — demotes formals to `&str` (finance-screens list_panels).
    let _ = kind_label.len()
    let _ = demo_id.len()
    let _ = post_path.len()
    let _ = body_id.len()
    let _ = button_id.len()
    let _ = refresh_sel.len()
    let _ = token.len()
    kind_label
}

fn make_demo_id() -> string {
    "demo-1"
}

fn post_path_for(id: string) -> string {
    "/post/" + id
}

pub fn render_bills_panel(token: string) -> string {
    let demo = make_demo_id()
    let post_path = post_path_for(make_demo_id())
    render_post_to_gl_block(
        "bill",
        demo,
        post_path,
        "billPostBody",
        "postBillGl",
        "#loadBills",
        token,
    )
}
"##;

fn bad_string_from_into_str(rs: &str) -> bool {
    let demoted = rs.contains("kind_label: &str")
        || rs.contains("fn render_post_to_gl_block(") && rs.contains(": &str");
    let call_bad = rs.contains("render_post_to_gl_block(String::from(")
        || (rs.contains("render_post_to_gl_block(")
            && rs.contains("String::from(\"bill\")"));
    demoted && call_bad
}

fn bad_owned_local_into_str(rs: &str) -> bool {
    let demoted = rs.contains("kind: &str") || rs.contains("fn aging_report(") && rs.contains(": &str");
    // owned `kind` (String) passed bare into &str formal
    demoted
        && rs.contains("aging_report(")
        && rs.contains(", kind)")
        && !rs.contains(", &kind)")
}

#[test]
fn finance_screens_string_lit_into_demoted_str_block_must_not_string_from() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("panels.wj", PANELS);
    let map = test.compile().expect("P3.376 compile");
    let rs = map.get("panels.rs").expect("panels.rs");
    if bad_string_from_into_str(rs) {
        eprintln!("P3.376 RED String::from into &str:\n{rs}");
    }
    assert!(
        !bad_string_from_into_str(rs),
        "P3.376: string lit into demoted &str must not String::from:\n{rs}"
    );
    test.cargo_check().expect("P3.376 cargo-check");
}

const AGING_MOD: &str = r#"
pub mod aging
"#;

const AGING: &str = r#"
fn aging_report(title: string, kind: string) -> string {
    title + kind
}

fn kind_from_json(json: string) -> string {
    json
}

pub fn render_aging(json: string) -> string {
    let title = "Aging"
    let kind = kind_from_json(json)
    aging_report(title, kind)
}
"#;

#[test]
fn finance_screens_owned_string_into_demoted_str_must_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", AGING_MOD);
    test.add_file("aging.wj", AGING);
    let map = test.compile().expect("P3.376 aging compile");
    let rs = map.get("aging.rs").expect("aging.rs");
    if bad_owned_local_into_str(rs) {
        eprintln!("P3.376 RED owned kind into &str:\n{rs}");
    }
    // Prefer cargo-check as ground truth when demotion is active.
    test.cargo_check().expect("P3.376 aging cargo-check");
    assert!(
        !bad_owned_local_into_str(rs),
        "P3.376: owned String into demoted &str must borrow:\n{rs}"
    );
}

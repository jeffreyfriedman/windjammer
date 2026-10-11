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

//! An owned binding placed inside a `vec![..]` is MOVED into the collection (even when the
//! collection itself is passed to a borrowing callee). A later use of the binding therefore
//! needs an automatic clone; the user must never see WJ0007 / rustc E0382 for it.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn check(src: &str) {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", src);
    test.cargo_check().unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn vec_into_borrowing_callee_then_reuse_of_element_binding() {
    check(
        r#"
pub fn query(args: Vec<string>) -> int { args.len() }

pub fn find(id: string) -> string {
    let n = query(vec![id])
    let got_id = id
    if n > 0 { got_id } else { "none" }
}
"#,
    );
}

#[test]
fn vec_let_then_reuse_of_element_binding() {
    check(
        r#"
pub fn count(a: string, b: string) -> int {
    let items = vec![a, b]
    let again = a
    items.len() + again.len() + b.len()
}
"#,
    );
}

#[test]
fn vec_of_struct_values_then_reuse() {
    check(
        r#"
pub struct Tag { pub name: string }

pub fn total(t: Tag) -> int {
    let tags = vec![t]
    let n = tags.len()
    n + t.name.len()
}
"#,
    );
}

#[test]
fn vec_element_used_only_once_is_not_cloned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn once(id: string) -> int {
    let items = vec![id]
    items.len()
}
"#,
    );
    let map = test.compile().expect("compile");
    let rs = map.get("mod.rs").expect("mod.rs");
    assert!(!rs.contains("id.clone()"), "single use must move:\n{rs}");
}

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

//! A method whose `string` formal is only forwarded into an owned slot (struct field /
//! owned callee) is emitted with an owned formal. A call site must then MOVE the argument:
//! an ordinary user method is never a "map key lookup", whatever the first formal's
//! (possibly stale) analysis ownership says. Only the map-key vocabulary may borrow keys.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn check_method(method: &str) {
    let src = format!(
        r#"
pub struct Record {{
    pub name: string,
}}

impl Record {{
    pub fn make(name: string) -> Record {{
        Record {{ name: name }}
    }}

    pub fn {method}(self, name: string) -> bool {{
        let _built = Record::make(name)
        true
    }}
}}

pub fn run(name: string) -> bool {{
    let r = Record {{ name: "seed" }}
    r.{method}(name)
}}
"#
    );
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", &src);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    assert!(
        !rs.contains(&format!(".{method}(&name)")),
        "owned forwarding formal must be moved, not borrowed:\n{rs}"
    );
    test.cargo_check().unwrap_or_else(|e| panic!("{method}: {e}"));
}

#[test]
fn validate_forwarding_into_owned_constructor() {
    check_method("validate");
}

#[test]
fn check_forwarding_into_owned_constructor() {
    check_method("check");
}

#[test]
fn rebuild_forwarding_into_owned_constructor() {
    check_method("rebuild");
}

#[test]
fn apply_forwarding_into_owned_constructor() {
    check_method("apply");
}

#[test]
fn free_function_forwarding_chain_is_still_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "lib.wj",
        r#"
pub struct Holder { pub id: string }

pub fn build(id: string) -> Holder { Holder { id: id } }
pub fn middle(id: string) -> bool {
    let _h = build(id)
    true
}
pub fn outer(id: string) -> bool { middle(id) }
"#,
    );
    test.cargo_check().unwrap_or_else(|e| panic!("{e}"));
}

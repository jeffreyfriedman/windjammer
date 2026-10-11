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
    feature = "codegen_tests",
))]

//! a local `string` passed into a call on one `if let` arm and
//! returned on the other must not `.clone()`. The arms are exclusive.
//!
//! Product `dialogue_system.rs` `current_text`:
//!   `self.substitute_variables(text.clone().clone(), world)`
//!   then `return text`
//! WJ is `return self.substitute_variables(text, world)` or `return text`.
//! Distinct from (a `for` binding into `push`) and (a string
//! pushed and then used again, where one clone is required).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Node {
    pub body: string,
}

pub fn node_text(node: Node) -> string {
    node.body
}

pub struct World {
    pub flag: int,
}

pub fn substitute(text: string, world: World) -> string {
    text
}

pub fn current_text(node: Option<Node>, world: Option<World>) -> string {
    if let Some(n) = node {
        let text = node_text(n)
        if let Some(w) = world {
            return substitute(text, w)
        }
        return text
    }
    ""
}
"#;

#[test]
fn module_file_exclusive_arm_string_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("text.clone()"),
        "exclusive-arm string cloned into call:\n{rs}"
    );
    let _ = test.cargo_check();
}

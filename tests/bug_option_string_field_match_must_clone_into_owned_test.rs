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

//! matching `Option<string>` through `&self` must clone into owned `Option<string>`.
//!
//! `animation/controller.wj` `current_animation` matches `self.current_animation` and
//! which is `Option<&String>` (expected `String`, found `&String`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn borrowed_some_without_clone(body: &str) -> bool {
    body.contains("match &self.current_animation") && body.contains("Some(s) => Some(s)")
        && !body.contains("Some(s.clone())")
}

#[test]
fn option_string_field_match_must_clone_into_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Anim {
    pub current_animation: Option<string>,
}
impl Anim {
    pub fn current_animation(&self) -> Option<string> {
        match self.current_animation {
            Some(s) => Some(s),
            None => None,
        }
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("Option<string> field match must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !borrowed_some_without_clone(body),
        "borrowed Option<String> match must clone into owned Option; got:\n{body}"
    );
}


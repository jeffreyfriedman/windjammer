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

//! Regression: f32 times len minus one cast must parenthesize.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_gap(body: &str) -> bool {
    // `as` binds tighter than `*`, so `gap * (n - 1_usize) as f32` is
    // `gap * ((n - 1_usize) as f32)`. Reject an i32 compare and a length term
    // that is not cast to f32.
    body.contains("> 1_i32") || !body.contains("(child_count - 1_usize) as f32")
}

#[test]
fn f32_times_len_minus_one_cast_must_parenthesize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Flex {
    pub gap: f32,
    pub children: Vec<i32>,
}

pub fn total_gap(container: Flex) -> f32 {
    let child_count = container.children.len()
    if child_count > 1 {
        container.gap * ((child_count - 1) as f32)
    } else {
        0.0
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.779: flex gap fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_gap(body),
        "f32 times (len - 1) as f32 must not use 1_i32 or an ungrouped cast; got:\n{body}"
    );
}

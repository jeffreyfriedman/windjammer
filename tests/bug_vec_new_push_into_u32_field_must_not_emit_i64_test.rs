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

//! P3.780: `Vec::new()` plus `push` of an untyped int, stored in `Vec<u32>`,
//! must not emit `Vec<i64>`.
//!
//! `streaming_coordinator_test.wj` pushes `100` and `200` into a local vec
//! that is stored in `StreamingTileBatch.add_ids: Vec<u32>`. Tip-out emits
//! `let mut adds: Vec<i64>` and `adds.push(100_i64)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i64_vec(body: &str) -> bool {
    body.contains("Vec<i64>") || body.contains("100_i64")
}

#[test]
fn vec_new_push_into_u32_field_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Batch {
    pub add_ids: Vec<u32>,
}

pub fn make() -> Batch {
    let mut adds = Vec::new()
    adds.push(100)
    adds.push(200)
    Batch { add_ids: adds }
}
"#,
    );
    let map = test
        .compile()
        .expect("Vec<u32> push fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_vec(body),
        "push into a Vec<u32> field must not emit i64; got:\n{body}"
    );
}

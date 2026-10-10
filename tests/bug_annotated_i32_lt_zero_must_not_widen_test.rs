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

//! P3.787: a declared `i32` compared with `< 0` must not widen zero to `i64`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_widen(body: &str) -> bool {
    body.contains("0_i64") || body.contains("as i64")
}

#[test]
fn annotated_i32_lt_zero_must_not_widen() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Snap {
    pub timestamp: f32,
}

pub fn find(snaps: Vec<Snap>, render_time: f32) -> i32 {
    let mut from_idx: i32 = -1
    let mut i = 0
    while i < snaps.len() {
        if snaps[i].timestamp <= render_time {
            from_idx = i as i32
        }
        i = i + 1
    }
    if from_idx < 0 {
        return -1
    }
    from_idx
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.787: annotated i32 < 0 fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_widen(body),
        "declared i32 compared to 0 must not widen; got:\n{body}"
    );
}

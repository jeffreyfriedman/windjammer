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

//! an i32 neighbor-offset tuple array must not mix `0_i64`.
//!
//! `tps_camera.wj` writes `[(-1, 0), (1, 0), (0, -1), (0, 1)]` and adds the
//! `[(-1, 0_i64), (1, 0), (0_i64, -1), (0, 1)]`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_mixed_i64(body: &str) -> bool {
    body.contains("0_i64")
}

#[test]
fn tuple_i32_offset_array_must_not_mix_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn neighbor(cx: i32, cz: i32) -> i32 {
    let offsets = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    let mut i = 0
    let mut found = 0
    while i < 4 {
        let (ox, oz) = offsets[i]
        let check_x = cx + ox
        let check_z = cz + oz
        if check_x != 0 || check_z != 0 {
            found = found + 1
        }
        i = i + 1
    }
    found
}
"#,
    );
    let map = test
        .compile()
        .expect("i32 offset-array fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_mixed_i64(body),
        "i32 offset tuples must not emit 0_i64; got:\n{body}"
    );
}

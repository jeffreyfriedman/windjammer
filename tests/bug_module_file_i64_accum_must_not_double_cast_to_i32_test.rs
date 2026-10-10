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

//! Regression: module file i64 accum must not double cast to i32.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// untyped `total = 0` under Custom return (not annotated i64).
const SRC: &str = r#"
pub struct LccEngine {
    pub total_triangles: u64,
}

pub fn sum_triangles(counts: Vec<u32>) -> LccEngine {
    let mut total = 0
    let mut i = 0
    while i < counts.len() {
        let triangles = counts[i]
        total = total + triangles as i64
        i = i + 1
    }
    LccEngine {
        total_triangles: (total / 3) as u64,
    }
}
"#;

#[test]
fn module_file_i64_accum_must_not_double_cast_to_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as i64 as i32")
        || rs.contains("as i64 as u64")
        || rs.contains("/ 3_u64");
    assert!(
        !bad,
        "RED: MultiFile emitted double-cast / u64 divisor into i64 accum:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

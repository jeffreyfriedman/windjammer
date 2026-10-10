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

//! P3.403 Product: `ai/npc_behavior.wj` SearchState — `let mut i = -1` / `while i <= 1`
//! emits `let mut i = -1_i64; while i <= 1_i32` (E0308 / E0277 i64 vs i32).
//!
//! Tip gen: `let mut i = -1_i64; while i <= 1_i32 { ... if i != 0_i32 ...`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn search_offsets(radius: f32) -> i32 {
    let mut count = 0
    let mut i = -1
    while i <= 1 {
        let mut j = -1
        while j <= 1 {
            if i != 0 || j != 0 {
                let _ = i as f32 * radius
                count = count + 1
            }
            j = j + 1
        }
        i = i + 1
    }
    count
}
"#;

#[test]
fn i32_neg_while_literal_peers_must_not_widen_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    // Consistent i32 (`-1_i32` + `1_i32`) is GREEN. Split is i64 counter + i32 lit peers.
    let split = (rs.contains("-1_i64") || rs.contains("= -1_i64"))
        && (rs.contains("1_i32") || rs.contains("0_i32") || rs.contains("while i <= 1_i32"));
    assert!(
        !split,
        "neg while counter must stay one int width (no i64 vs i32 lit):\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

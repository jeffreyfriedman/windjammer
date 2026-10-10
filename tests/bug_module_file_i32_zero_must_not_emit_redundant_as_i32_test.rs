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

//! WDB-352: i32 zero return must not emit redundant `0_i32 as i32`.
//!
//! Tip output for a game project cluster (14+ files), e.g. `audio/audio_mixer.rs`:
//!   `0_i32 as i32` on else / early-return arms already typed as i32.
//! Prefer bare `0` / `0_i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn fallback(ok: bool) -> i32 {
    if ok {
        1
    } else {
        0
    }
}
"#;

#[test]
fn module_file_i32_zero_must_not_emit_redundant_as_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_i32 as i32") || rs.contains("0 as i32");
    assert!(
        !bad,
        "redundant i32 cast on zero:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

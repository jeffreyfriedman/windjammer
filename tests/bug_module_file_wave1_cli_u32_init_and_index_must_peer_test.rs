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

//! wave1 CLI residual — `u32 = 0_usize` and `args[i + 1]` index by u32.
//!
//! Twin of for remaining tip-out/gen sites (artifact/bench/publish CLI):
//!   `let mut i: u32 = 0_usize;` + `args[i + 1]` → E0308 / E0277.
//! Product WJ is untyped `let mut i = 0` with `-> u32` and `args.len()` / `args[i+1]`.
//! Adjacency/CDLP/LCC were synced (P3.390); wave1 CLI MultiFile+tip GREEN (P3.400).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn parse_samples(args: Vec<string>) -> u32 {
    let mut i = 0
    while i < args.len() {
        if args[i] == "--samples" {
            if i + 1 < args.len() {
                let _n = args[i + 1]
                return 1
            }
        }
        i = i + 1
    }
    0
}
"#;

#[test]
fn module_file_u32_return_index_loop_must_not_emit_0_usize_or_bare_i_plus_1() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad_init = rs.contains("u32 = 0_usize") || rs.contains(": u32 = 0_usize");
    // Bare `args[i + 1]` or wrong-precedence `i + 1 as usize` (must be `(i + 1) as usize`).
    let bad_index = rs.contains("args[i + 1]")
        || rs.contains("args[i+1]")
        || rs.contains("i + 1 as usize")
        || rs.contains("i+1 as usize");
    assert!(
        !bad_init && !bad_index,
        "MultiFile u32 return + len() loop emitted init={bad_init} index={bad_index}:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

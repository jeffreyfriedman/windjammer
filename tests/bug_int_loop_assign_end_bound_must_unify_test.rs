//! Index loop `j = end` must unify when `end` was set from another index / `len`.
//!
//! Ecosystem `wj-url` authority scanner: `auth_end = i` then later `j = auth_end`
//! emits `j: i32` vs `auth_end: usize` → E0308 (`packages/wj-url`).

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn int_loop_assign_end_bound_must_cargo_check() {
    let source = r#"
use std::strings

pub fn colon_before_slash(text: string) -> int {
    let n = strings.len(text)
    let mut auth_end = 0
    let mut found_end = false
    let mut i = 0
    while i < n {
        let ch = strings.substring(text, i, i + 1)
        if ch == "/" {
            auth_end = i
            found_end = true
            i = n
        } else {
            i = i + 1
        }
    }
    if !found_end {
        return -1
    }
    let mut colon_at = -1
    let mut j = 0
    while j < auth_end {
        if strings.substring(text, j, j + 1) == ":" {
            colon_at = j
            j = auth_end
        } else {
            j = j + 1
        }
    }
    colon_at
}
"#;
    // needles: substring used; primary assertion is cargo check success
    test_utils::assert_stdlib_runtime_links(source, &["strings::substring"]);
}

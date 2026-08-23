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

//! FAILING REPRO — `for (k, v) in map` must not emit a post-loop `drop(map)`.
//!
//! Ecosystem `wj-cookie` helpers:
//! ```ignore
//! fn map_count(map: HashMap<string, string>) -> int {
//!     let mut n = 0
//!     for (_k, _v) in map {
//!         n = n + 1
//!     }
//!     n
//! }
//! ```
//! Codegen moves `map` into `into_iter()`, then still emits
//! `std::thread::spawn(move || drop(map))` (E0382 use of moved value).

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn hashmap_for_in_must_not_emit_post_loop_drop() {
    let generated = test_utils::compile_single(
        r#"
use std::collections::HashMap

pub fn count_entries(map: HashMap<string, string>) -> int {
    let mut n = 0
    for (_k, _v) in map {
        n = n + 1
    }
    n
}
"#,
    );

    assert!(
        !generated.contains("thread::spawn")
            && !generated.contains("drop(map)"),
        "for-in over HashMap must not emit post-loop drop(map):\n{generated}"
    );
}

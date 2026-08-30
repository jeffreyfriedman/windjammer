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

//! FAILING REPRO — `pub mod` declarations at end of `lib.wj` truncate root codegen.
//!
//! Ecosystem `wj-migrate` had `pub mod db_apply` after domain fns; generated `lib.rs`
//! contained only `Migration` struct (~21 lines). Moving `pub mod` to top of `lib.wj`
//! restores full `lib.rs` (~192 lines) with `pending`, `parse_filename`, etc.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn pub_mod_at_end_of_lib_must_not_truncate_root_functions() {
    let generated = test_utils::compile_single(
        r#"
pub mod child

use std::strings

pub struct Item {
    pub id: int,
}

pub fn label(item: Item) -> string {
    item.id
}

pub mod child {
    pub fn bump(n: int) -> int {
        n + 1
    }
}
"#,
    );

    assert!(
        generated.contains("pub fn label"),
        "root lib functions must appear in generated lib.rs when pub mod is at end:\n{generated}"
    );
}

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

//! FAILING REPRO — HashMap get None arm `0` must match value width (i64 for `int`).
//! Tip multipass emits `0_i32` into `(bool, i64)` for SharedMap-shaped lookup.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str = r#"
use std::collections::HashMap
use std::sync::{Arc, Mutex}

pub struct SharedMapSI {
    inner: Arc<Mutex<HashMap<string, int>>>,
}

pub fn shared_map_get(m: SharedMapSI, key: string) -> (SharedMapSI, bool, int) {
    let result = match m.inner.lock() {
        Ok(g) => {
            match g.get(key) {
                Some(v) => (true, *v),
                None => (false, 0),
            }
        }
        Err(_) => (false, 0),
    }
    (m, result.0, result.1)
}
"#;

#[test]
fn hashmap_none_zero_must_match_i64_value_width() {
    let generated = test_utils::compile_single(SOURCE);
    assert!(
        !generated.contains("0_i32"),
        "None/Err zero for int HashMap values must be i64, not 0_i32:\n{generated}"
    );
    assert!(
        generated.contains("0_i64") || generated.contains("(false, 0)"),
        "expected i64-width zero in None arm:\n{generated}"
    );
}

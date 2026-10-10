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

//! P3.783: `while b + 1 < pairs.len()` must keep `b` as `usize`.
//!
//! `limit_influences` writes `let mut b = 0`, compares `b + 1` with
//! `pairs.len()`, and indexes `pairs[b]` / `pairs[b + 1]`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i64_counter(body: &str) -> bool {
    body.contains("b = 0_i64")
        || body.contains("1_i64")
        || body.contains("as i64")
        || body.contains("b as usize")
}

#[test]
fn while_plus_one_len_index_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Pair {
    pub weight: int,
}

pub fn limit_influences(pairs: Vec<Pair>) {
    let mut b = 0
    while b + 1 < pairs.len() {
        let left = pairs[b]
        let right = pairs[b + 1]
        if left.weight > right.weight {
            b = b + 1
        }
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.783: while-plus-one len index fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_counter(body),
        "while counter compared to Vec::len() and used as an index must not be i64; got:\n{body}"
    );
}

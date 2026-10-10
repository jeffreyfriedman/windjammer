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

//! an untyped `for`-loop counter compared with `.len()` must not be `i64`.
//!
//! `shader_graph_executor.wj` writes `let mut idx = 0` then, inside
//! `for pass in sorted`, `idx < barriers.len()` and `barriers[idx]`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i64_counter(body: &str) -> bool {
    body.contains("idx = 0_i64") || body.contains("idx as usize")
}

#[test]
fn for_loop_len_counter_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Barrier {
    pub needs_barrier_before: bool,
}

pub struct Pass {
    pub enabled: bool,
}

pub fn run(sorted: Vec<Pass>, barriers: Vec<Barrier>) {
    let mut idx = 0
    for pass in sorted {
        if pass.enabled {
            if idx < barriers.len() && barriers[idx].needs_barrier_before {
                idx = idx + 1
            }
        }
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("for-loop len counter fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_counter(body),
        "for-loop counter compared to Vec::len() must not be i64; got:\n{body}"
    );
}


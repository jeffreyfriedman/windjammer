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

//! P3.773: an untyped `for`-loop counter compared with `.len()` must not be `i64`.
//!
//! `shader_graph_executor.wj` writes `let mut idx = 0` then, inside
//! `for pass in sorted`, `idx < barriers.len()` and `barriers[idx]`.
//! Tip-out emits `let mut idx = 0_i64` and `barriers[(idx as usize)]`.

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
        .expect("P3.773: for-loop len counter fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_counter(body),
        "for-loop counter compared to Vec::len() must not be i64; got:\n{body}"
    );
}

fn search_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    let mut walked = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn for_loop_len_counter_tip_out_shader_graph_executor() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/rendering/shader_graph_executor.rs",
        ));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("shader_graph_executor.rs");
        if text.contains("let mut idx = 0_i64") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(
        saw,
        "P3.773: shader_graph_executor.rs product file missing"
    );
    assert!(
        bad_paths.is_empty(),
        "P3.773 RED: tip-out for-loop counter is 0_i64:\n  {}",
        bad_paths.join("\n  ")
    );
}

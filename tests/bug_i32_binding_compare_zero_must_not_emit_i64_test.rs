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

//! P3.750: an `i32` binding compared to untyped `0` must not emit `0_i64`.
//!
//! `shader_graph_compiler.wj` has `let mut existing_group: i32 = -1` then
//! `if existing_group >= 0`. Tip-out emits `existing_group >= 0_i64`
//! (expected `i32`, found `i64`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_zero(body: &str) -> bool {
    body.contains(">= 0_i64") || body.contains(">= 0_i64)")
}

#[test]
fn i32_binding_compare_zero_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn grouped() -> bool {
    let mut existing_group: i32 = -1
    existing_group >= 0
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.750: i32 compare-zero fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_zero(body),
        "i32 binding compared to 0 must not use 0_i64; got:\n{body}"
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
fn i32_binding_compare_zero_tip_out_shader_graph_compiler() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/rendering/shader_graph_compiler.rs",
        ));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("shader_graph_compiler.rs");
        if bad_zero(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.750: shader_graph_compiler.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.750 RED: tip-out compares i32 to 0_i64:\n  {}",
        bad_paths.join("\n  ")
    );
}

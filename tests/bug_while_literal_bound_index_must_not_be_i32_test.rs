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

//! P3.798: `while i < 6` used as an array/vec index must not be `i32`.
//!
//! `fps_camera.wj` writes `let mut i = 0` then `while i < 6` and `offsets[i]`.
//! Tip-out emits `let mut i = 0_i32`, `while i < 6_i32`, and `offsets[(i as usize)]`.
//! The bound is an int literal, not `.len()`, and `i` is not returned as `i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i32_index(body: &str) -> bool {
    body.contains("i = 0_i32") || body.contains("6_i32") || body.contains("i as usize")
}

#[test]
fn while_literal_bound_index_must_not_be_i32() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn pick(offsets: Vec<f32>) -> f32 {
    let mut i = 0
    let mut best = 0.0
    while i < 6 {
        best = offsets[i]
        i = i + 1
    }
    best
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.798: while literal-bound index must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i32_index(body),
        "while i < 6 used as an index must not be i32; got:\n{body}"
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
fn while_literal_bound_index_tip_out_fps_camera() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/camera/fps_camera.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("fps_camera.rs");
        if text.contains("while i < 6_i32") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.798: fps_camera.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.798 RED: tip-out while-index bound is 6_i32:\n  {}",
        bad_paths.join("\n  ")
    );
}

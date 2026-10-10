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

//! P3.775: an untyped counter incremented by `1` must not emit `0_i64`.
//!
//! `material.wj` `log_palette_summary` writes `let mut configured_count = 0`
//! then `configured_count = configured_count + 1` inside `while i < 256u32`.
//! Tip-out emits `let mut configured_count = 0_i64`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i64_counter(body: &str) -> bool {
    body.contains("configured_count = 0_i64") || body.contains("configured_count: i64")
}

#[test]
fn untyped_plus_one_counter_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn count_slots() {
    let mut configured_count = 0
    let mut i: u32 = 0u32
    while i < 256u32 {
        configured_count = configured_count + 1
        i = i + 1
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.775: plus-one counter fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_counter(body),
        "untyped + 1 counter must not emit 0_i64; got:\n{body}"
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
fn untyped_plus_one_counter_tip_out_material_palette() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/voxel/material.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("material.rs");
        if text.contains("let mut configured_count = 0_i64") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.775: material.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.775 RED: tip-out palette counter is 0_i64:\n  {}",
        bad_paths.join("\n  ")
    );
}

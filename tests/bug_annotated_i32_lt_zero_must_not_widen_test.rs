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

//! P3.787: a declared `i32` compared with `< 0` must not widen zero to `i64`.
//!
//! `networking/snapshot.wj` writes `let mut from_idx: i32 = -1` then
//! `if from_idx < 0`. Tip-out emits `from_idx < (0_i64 as i64)` while the
//! sibling `eidx >= 0` in the same function stays `0_i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_widen(body: &str) -> bool {
    body.contains("0_i64") || body.contains("as i64")
}

#[test]
fn annotated_i32_lt_zero_must_not_widen() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Snap {
    pub timestamp: f32,
}

pub fn find(snaps: Vec<Snap>, render_time: f32) -> i32 {
    let mut from_idx: i32 = -1
    let mut i = 0
    while i < snaps.len() {
        if snaps[i].timestamp <= render_time {
            from_idx = i as i32
        }
        i = i + 1
    }
    if from_idx < 0 {
        return -1
    }
    from_idx
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.787: annotated i32 < 0 fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_widen(body),
        "declared i32 compared to 0 must not widen; got:\n{body}"
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
fn annotated_i32_lt_zero_tip_out_snapshot() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/networking/snapshot.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("snapshot.rs");
        if text.contains("from_idx < (0_i64 as i64)") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.787: snapshot.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.787 RED: tip-out widens i32 < 0:\n  {}",
        bad_paths.join("\n  ")
    );
}

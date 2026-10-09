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

//! P3.770: an i32 neighbor-offset tuple array must not mix `0_i64`.
//!
//! `tps_camera.wj` writes `[(-1, 0), (1, 0), (0, -1), (0, 1)]` and adds the
//! components to `i32` cell coordinates. Tip-out emits
//! `[(-1, 0_i64), (1, 0), (0_i64, -1), (0, 1)]`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_mixed_i64(body: &str) -> bool {
    body.contains("0_i64")
}

#[test]
fn tuple_i32_offset_array_must_not_mix_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn neighbor(cx: i32, cz: i32) -> i32 {
    let offsets = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    let mut i = 0
    let mut found = 0
    while i < 4 {
        let (ox, oz) = offsets[i]
        let check_x = cx + ox
        let check_z = cz + oz
        if check_x != 0 || check_z != 0 {
            found = found + 1
        }
        i = i + 1
    }
    found
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.770: i32 offset-array fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_mixed_i64(body),
        "i32 offset tuples must not emit 0_i64; got:\n{body}"
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
fn tuple_i32_offset_array_tip_out_tps_camera() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/camera/tps_camera.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("tps_camera.rs");
        if text.contains("(-1, 0_i64)") || text.contains("(0_i64, -1)") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.770: tps_camera.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.770 RED: tip-out mixes i64 into an i32 offset array:\n  {}",
        bad_paths.join("\n  ")
    );
}

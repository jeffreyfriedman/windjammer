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
    feature = "codegen_tests",
))]

//! WDB-383: u32 index into `[]` must not emit `as i64 as usize`.
//!
//! Product tip game-core `frame_analysis.rs`:
//!   `bins[clamped as i64 as usize] += 1` where `clamped` is `u32`.
//! Prefer `bins[clamped as usize]`. Twin of WDB-353 (`as i32 as usize`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn bump(bins: Vec<u32>, clamped: u32) {
    bins[clamped as usize] = bins[clamped as usize] + 1
}
"#;

#[test]
fn wdb383_module_file_u32_index_must_not_cast_via_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-383 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-383 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as i64 as usize") || rs.contains("as i32 as usize");
    assert!(
        !bad,
        "WDB-383 RED: u32 index double-cast via i64/i32:\n{rs}"
    );
    test.cargo_check().expect("WDB-383 cargo-check");
}

#[test]
fn wdb383_tip_out_game_core_frame_analysis_must_not_cast_via_i64() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("frame_analysis.rs"),
        tip.join("rendering/frame_analysis.rs"),
        game.join("gen/frame_analysis.rs"),
        game.join("gen/rendering/frame_analysis.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            !line.trim_start().starts_with("//") && line.contains("as i64 as usize")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-383: frame_analysis product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-383 RED: tip/product u32 index as i64 as usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}

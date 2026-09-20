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

//! WDB-339: i32 coord `cy + N` must not emit `N_i64 as i32`.
//!
//! Product tip game-core `scene/component_viewer_controls.rs`:
//!   `set_if(grid, x, (cy + 3_i64 as i32), z, …)` with `cy`/`x`/`z`: i32 → E0308
//!   noise / wrong width (prefer `3_i32` / same-width peers).
//! Distinct from WDB-328 (i64 loop vs i32 lit) — here i32 context widens lit to i64
//! then casts back.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn offset_y(cy: i32, delta: i32) -> i32 {
    cy + 3
}

pub fn place(cy: i32) -> i32 {
    offset_y(cy, 0) + 2
}
"#;

#[test]
fn wdb339_module_file_i32_coord_add_must_not_emit_i64_as_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-339 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-339 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_i64 as i32")
        || rs.contains("3_i64")
        || rs.contains("2_i64");
    assert!(
        !bad,
        "WDB-339 RED: i32 coord add emitted i64 lit / as i32:\n{rs}"
    );
    test.cargo_check().expect("WDB-339 cargo-check");
}

#[test]
fn wdb339_tip_out_game_core_component_viewer_must_not_emit_i64_as_i32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("component_viewer_controls.rs"),
        tip.join("scene/component_viewer_controls.rs"),
        game.join("gen/scene/component_viewer_controls.rs"),
        game.join("scene/component_viewer_controls.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("component_viewer");
        let bad = text.contains("_i64 as i32")
            || text.lines().any(|line| {
                line.contains("cy +") && line.contains("_i64")
            });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-339: game-core/tip component_viewer_controls missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-339 RED: tip/product emits i64-as-i32 coord peers in:\n  {}",
        bad_paths.join("\n  ")
    );
}

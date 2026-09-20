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

//! WDB-352: i32 zero return must not emit redundant `0_i32 as i32`.
//!
//! Product tip game-core cluster (14+ files), e.g. `audio/audio_mixer.rs`:
//!   `0_i32 as i32` on else / early-return arms already typed as i32.
//! Prefer bare `0` / `0_i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn fallback(ok: bool) -> i32 {
    if ok {
        1
    } else {
        0
    }
}
"#;

#[test]
fn wdb352_module_file_i32_zero_must_not_emit_redundant_as_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-352 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-352 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("0_i32 as i32") || rs.contains("0 as i32");
    assert!(
        !bad,
        "WDB-352 RED: redundant i32 cast on zero:\n{rs}"
    );
    test.cargo_check().expect("WDB-352 cargo-check");
}

#[test]
fn wdb352_tip_out_game_core_must_not_emit_0_i32_as_i32() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("audio_mixer.rs"),
        tip.join("audio/audio_mixer.rs"),
        tip.join("astar_grid.rs"),
        tip.join("ai/astar_grid.rs"),
        game.join("gen/audio/audio_mixer.rs"),
        game.join("gen/ai/astar_grid.rs"),
        game.join("gen/vgs/visibility.rs"),
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
            line.contains("0_i32 as i32") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-352: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-352 RED: tip/product 0_i32 as i32 in:\n  {}",
        bad_paths.join("\n  ")
    );
}

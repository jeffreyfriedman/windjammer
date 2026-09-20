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

//! WDB-328: i64 loop counters from negative init must not take `_i32` lit peers.
//!
//! Product tip game-core `ai/npc_behavior.rs` SearchState::new:
//!   `let mut i = -1_i64; while i <= 1_i32` / `i != 0_i32` → E0308/E0277.
//! Source WJ: `let mut i = -1` / `while i <= 1`. Prefer `1_i64` / `0_i64`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn neighbor_offsets() -> i64 {
    let mut i = -1
    let mut count = 0
    while i <= 1 {
        if i != 0 {
            count = count + 1
        }
        i = i + 1
    }
    count
}
"#;

#[test]
fn wdb328_module_file_i64_neg_init_loop_must_not_take_i32_lit_peers() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-328 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-328 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("1_i32") || rs.contains("0_i32") || rs.contains("-1_i32");
    assert!(
        !bad,
        "WDB-328 RED: i64 neg-init loop emitted i32 lit peers:\n{rs}"
    );
    test.cargo_check().expect("WDB-328 cargo-check");
}

#[test]
fn wdb328_tip_out_game_core_npc_search_must_not_mix_i64_i32_loop_lits() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("npc_behavior.rs"),
        tip.join("ai/npc_behavior.rs"),
        game.join("gen/ai/npc_behavior.rs"),
        game.join("ai/npc_behavior.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("npc");
        // Narrow: i64 binding with i32 compare lits in SearchState::new region.
        let bad = text.contains("-1_i64")
            && (text.contains("<= 1_i32") || text.contains("!= 0_i32"));
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-328: game-core/tip npc_behavior missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-328 RED: tip/product mixes i64 loop with i32 lits in:\n  {}",
        bad_paths.join("\n  ")
    );
}

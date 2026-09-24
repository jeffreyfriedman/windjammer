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

//! WDB-379: `format!` must not lower to `write!(&mut __s, …).unwrap()`.
//!
//! Product tip game-core (`scenes/scene_file.rs`, `asset_db.rs`, BT executor, …):
//!   `write!(&mut __s, "{{\"id\":\"{}\",\"name\":\"{}\"", self.id.clone(), self.name.clone()).unwrap()`
//! Prefer `format!(…)` (or equivalent) without `.unwrap()`. Rust leakage.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn label(name: string, n: i32) -> string {
    let s = format!("{}-{}", name, n)
    s
}

pub fn json_pair(id: string, name: string) -> string {
    format!("{{\"id\":\"{}\",\"name\":\"{}\"", id, name)
}
"#;

#[test]
fn wdb379_module_file_format_must_not_emit_write_unwrap() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-379 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-379 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("write!(&mut __s") || rs.contains(".unwrap()");
    assert!(
        !bad,
        "WDB-379 RED: format! lowered to write!/unwrap:\n{rs}"
    );
    test.cargo_check().expect("WDB-379 cargo-check");
}

#[test]
fn wdb379_tip_out_game_core_must_not_emit_write_unwrap() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("scenes/scene_file.rs"),
        tip.join("scene_file.rs"),
        tip.join("scenes/asset_db.rs"),
        tip.join("behavior_tree/executor.rs"),
        game.join("gen/scenes/scene_file.rs"),
        game.join("gen/scenes/asset_db.rs"),
        game.join("gen/behavior_tree/executor.rs"),
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
            !line.trim_start().starts_with("//") && line.contains("write!(&mut __s")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-379: scene/asset/BT product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-379 RED: tip/product write!(&mut __s in:\n  {}",
        bad_paths.join("\n  ")
    );
}

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

//! WDB-349: `matches!(opt.clone(), Some(_))` must not clone for presence check.
//!
//! Product tip game-core `dcc_pipeline/usd.rs`:
//!   `matches!(self.mesh.clone(), Some(_))` in `has_mesh`.
//! Prefer `self.mesh.is_some()` or `matches!(&self.mesh, Some(_))`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Mesh {
    pub id: i32,
}

pub struct Prim {
    pub mesh: Option<Mesh>,
}

pub fn has_mesh(prim: Prim) -> bool {
    match prim.mesh {
        Some(_) => true,
        None => false,
    }
}
"#;

#[test]
fn wdb349_module_file_option_presence_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-349 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-349 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()") && rs.contains("Some(");
    assert!(
        !bad,
        "WDB-349 RED: Option presence check cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-349 cargo-check");
}

#[test]
fn wdb349_tip_out_game_core_usd_must_not_matches_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("usd.rs"),
        tip.join("dcc_pipeline/usd.rs"),
        game.join("gen/dcc_pipeline/usd.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("usd");
        let bad = text.lines().any(|line| {
            line.contains("matches!(")
                && line.contains(".clone()")
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-349: game-core/tip usd missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-349 RED: tip/product matches!(…clone…) in:\n  {}",
        bad_paths.join("\n  ")
    );
}

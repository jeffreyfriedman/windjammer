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

//! P3.703: Breach Protocol `GameState::update_death_state` mutates `self.respawn_timer`
//! / camera / renderer HUD but tip product emit keeps owned immutable `self`
//! (`error[E0594]: cannot assign … as self is not declared as mutable`).
//!
//! Minimal isolate with the same field assigns is GREEN (`&mut self`). Product
//! tip-out / full game multipass still emits bare `self` — file tip-out gate.

use std::path::PathBuf;

#[test]
fn bp_tip_out_update_death_state_must_emit_mut_self() {
    let mut paths = Vec::new();
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(dir);
        // Sibling dogfood game (workspace layout).
        paths.push(
            root.parent()
                .unwrap_or(root.as_path())
                .join("breach-protocol/gen/game_state.rs"),
        );
        paths.push(
            root.parent()
                .unwrap_or(root.as_path())
                .join("breach-protocol/build/game_state.rs"),
        );
    }
    paths.push(PathBuf::from(
        "/Users/jeffreyfriedman/src/wj/breach-protocol/gen/game_state.rs",
    ));

    let path = paths.into_iter().find(|p| p.exists());
    let Some(path) = path else {
        eprintln!("skip: breach-protocol gen/game_state.rs not present");
        return;
    };
    let rs = std::fs::read_to_string(&path).expect("read game_state.rs");
    assert!(
        rs.contains("fn update_death_state(&mut self")
            || rs.contains("fn update_death_state(mut self"),
        "P3.703 tip-out: update_death_state must emit &mut self (or mut self), not owned immutable self. path={} snippet:\n{}",
        path.display(),
        rs.lines()
            .filter(|l| l.contains("update_death_state") || l.contains("respawn_timer"))
            .take(12)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

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

//! WDB-338: `&mut Mesh` formal must not receive owned `mesh.clone()`.
//!
//! Product tip game-core `rendering/placeholder_assets.rs`:
//!   `push_quad(mesh.clone(), …)` with `mesh: &mut PlaceholderMesh` → E0308
//!   (expected `&mut PlaceholderMesh`, found `PlaceholderMesh`).
//! Distinct from WDB-336 (`&mut data.clone()`): here the call omits `&mut` and
//! passes an owned clone. Prefer `push_quad(&mut mesh, …)` / `push_quad(mesh, …)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Mesh {
    pub n: i32,
}

pub fn push_quad(mesh: Mesh, v: i32) {
    mesh.n = mesh.n + v
}

pub fn cube(mesh: Mesh) {
    push_quad(mesh, 1)
    push_quad(mesh, 2)
}
"#;

#[test]
fn wdb338_module_file_mut_mesh_must_not_receive_owned_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-338 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-338 MultiFile lib.rs:\n{rs}");
    // Fail if call site passes mesh.clone() (owned) — whether or not formal is &mut.
    let bad = rs.contains("mesh.clone()")
        || rs.contains("push_quad(mesh.clone()");
    assert!(
        !bad,
        "WDB-338 RED: mut mesh received owned mesh.clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-338 cargo-check");
}

#[test]
fn wdb338_tip_out_game_core_placeholder_assets_must_not_pass_owned_mesh_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("placeholder_assets.rs"),
        tip.join("rendering/placeholder_assets.rs"),
        game.join("gen/rendering/placeholder_assets.rs"),
        game.join("rendering/placeholder_assets.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("placeholder_assets");
        let formal_mut = text.contains("mesh: &mut ") || text.contains("fn push_quad(mesh: &mut");
        let bad_call = text.lines().any(|line| {
            line.contains("push_quad(") && line.contains("mesh.clone()")
        });
        if formal_mut && bad_call {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-338: game-core/tip placeholder_assets missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-338 RED: tip/product passes mesh.clone() into &mut mesh in:\n  {}",
        bad_paths.join("\n  ")
    );
}

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

//! WDB-336: `&mut Vec` formal must not receive `&mut data.clone()` (temp).
//!
//! Product tip game-core `rendering/mesh_renderer.rs`:
//!   `push_mat4(&mut data.clone(), …)` / `push_mat4(&mut mat4_floats.clone(), …)`
//!   with `data: &mut Vec<f32>` → E0716 / discarded mutation.
//! Source WJ: mutate the real buffer (`push_mat4(data, …)` → `&mut data`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Mat4 {
    pub m: f32,
}

pub fn push_mat4(data: Vec<f32>, m: Mat4) {
    data.push(m.m)
}

// Product shape (mesh_renderer): local `let mut data` reused after demoted `&mut Vec`
// must emit `&mut data`, never `&mut data.clone()`.
pub fn upload(view: Mat4, proj: Mat4) -> Vec<f32> {
    let mut data: Vec<f32> = Vec::new()
    push_mat4(data, view)
    push_mat4(data, proj)
    data
}
"#;

#[test]
fn wdb336_module_file_mut_vec_must_not_borrow_clone_temp() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-336 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-336 MultiFile lib.rs:\n{rs}");
    let bad_temp = rs.contains("&mut data.clone()")
        || rs.contains("&mut ") && rs.contains(".clone()");
    assert!(
        !bad_temp,
        "WDB-336 RED: mut Vec received &mut <temp>.clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-336 cargo-check");
}

#[test]
fn wdb336_tip_out_game_core_mesh_renderer_must_not_mut_borrow_clone_temp() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("mesh_renderer.rs"),
        tip.join("rendering/mesh_renderer.rs"),
        game.join("gen/rendering/mesh_renderer.rs"),
        game.join("rendering/mesh_renderer.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("mesh_renderer");
        let bad = text.lines().any(|line| {
            // Only flag mut-arg clone temps (`&mut data.clone()`), not owned peers
            // like `push_mat4(&mut data, view.clone())`.
            line.contains("&mut ")
                && line
                    .split("&mut ")
                    .skip(1)
                    .any(|rest| rest.trim_start().contains(".clone()") && {
                        let place = rest.trim_start();
                        let end = place.find([',', ')']).unwrap_or(place.len());
                        place[..end].ends_with(".clone()")
                    })
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-336: game-core/tip mesh_renderer missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-336 RED: tip/product uses &mut <temp>.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}

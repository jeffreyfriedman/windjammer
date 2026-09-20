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

//! WDB-356: Copy `self` must not emit `self.clone().method()` into owned-self callee.
//!
//! Product tip game-core `math/mat4.rs` (Mat4 is Copy):
//!   `self.clone().multiply(other)` / `self.clone().transpose().to_array()`
//! Prefer bare `self.multiply(other)` (Copy) — no `.clone()`.
//! Broader product cluster also emits `self.clone().…` on non-Copy types for `&self`
//! wrappers; this gate focuses on Copy Mat4.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Mat4 {
    pub m00: f32,
}

impl Mat4 {
    pub fn multiply(self, other: Mat4) -> Mat4 {
        Mat4 { m00: self.m00 * other.m00 }
    }

    pub fn mul(self, other: Mat4) -> Mat4 {
        self.multiply(other)
    }
}
"#;

#[test]
fn wdb356_module_file_copy_self_must_not_clone_before_owned_method() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-356 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-356 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-356 RED: Copy Mat4 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-356 cargo-check");
}

#[test]
fn wdb356_tip_out_game_core_mat4_must_not_self_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("mat4.rs"),
        tip.join("math/mat4.rs"),
        game.join("gen/math/mat4.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("mat4");
        let bad = text.lines().any(|line| {
            line.contains("self.clone()") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-356: game-core/tip mat4 missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-356 RED: tip/product self.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}

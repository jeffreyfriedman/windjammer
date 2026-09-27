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

//! WDB-395: i32/i64 sentinel compares (`ci < 0`, `parent >= 0`) must not emit `0_usize`.
//!
//! Product tip game-core leftovers after WDB-364 / P3.369:
//!   `animation/skeleton.rs`: `if parent >= 0_usize as usize` while `Bone.parent: i32`
//!   `ecs/scene.rs`: `if ci < 0_usize` while `find_index -> i32`;
//!                   `if old_parent as i64 >= 0_usize` while `parents: Vec<i64>`
//!
//! P3.369 isolate is GREEN for a bare i64 field compare. Product still poisons the
//! zero literal when the same function also indexes with `as usize` and walks
//! `.len()` with a usize counter (including a shadowed `ci`).
//! WDB-364 isolate uses `parent: usize`, so `0_usize` is type-correct there.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Bone {
    pub parent: i32,
    pub children: Vec<u32>,
}

pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    pub fn remove_bone(self, bone_id: u32) -> bool {
        if (bone_id as usize) >= self.bones.len() {
            return false
        }
        let parent = self.bones[bone_id as usize].parent
        if parent >= 0 {
            let p = parent as usize
            let mut ci = 0
            while ci < self.bones[p].children.len() {
                if self.bones[p].children[ci] == bone_id {
                    self.bones[p].children.remove(ci)
                    break
                }
                ci = ci + 1
            }
        }
        true
    }
}

pub struct Scene {
    pub entities: Vec<i64>,
    pub parents: Vec<i64>,
    pub children: Vec<Vec<i64>>,
}

impl Scene {
    pub fn find_index(self, entity_id: i64) -> i32 {
        let mut i = 0
        while i < self.entities.len() {
            if self.entities[i] == entity_id {
                return i as i32
            }
            i = i + 1
        }
        -1
    }

    pub fn set_parent(self, child_id: i64, parent_id: i64) {
        let ci = self.find_index(child_id)
        if ci < 0 {
            return
        }
        let pi = self.find_index(parent_id)
        if pi < 0 {
            return
        }
        let old_parent = self.parents[ci as usize]
        if old_parent >= 0 {
            let opi = old_parent as usize
            let mut ci = 0
            while ci < self.children[opi].len() {
                let c = self.children[opi][ci]
                if c != child_id {
                    let _ = c
                }
                ci = ci + 1
            }
        }
        self.parents[ci as usize] = parent_id
    }
}
"#;

fn signed_zero_poisoned(rs: &str) -> bool {
    rs.lines().any(|line| {
        let t = line.trim();
        !t.starts_with("//")
            && (t.contains("< 0_usize")
                || t.contains(">= 0_usize")
                || t.contains("< (0_usize")
                || t.contains(">= (0_usize")
                || t.contains("< 0_usize as usize")
                || t.contains(">= 0_usize as usize"))
    })
}

#[test]
fn wdb395_module_file_i32_i64_compare_zero_must_not_emit_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-395 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-395 MultiFile lib.rs:\n{rs}");
    assert!(
        !signed_zero_poisoned(rs),
        "WDB-395 RED: signed sentinel compare emitted 0_usize:\n{rs}"
    );
    test.cargo_check().expect("WDB-395 cargo-check");
}

#[test]
fn wdb395_tip_out_game_core_signed_sentinel_must_not_be_usize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("animation/skeleton.rs"),
        tip.join("skeleton.rs"),
        tip.join("ecs/scene.rs"),
        tip.join("scene.rs"),
        tip.join("editor/half_edge.rs"),
        tip.join("rendering/spatial_index.rs"),
        game.join("gen/animation/skeleton.rs"),
        game.join("gen/ecs/scene.rs"),
        game.join("gen/editor/half_edge.rs"),
        game.join("gen/rendering/spatial_index.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        if signed_zero_poisoned(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-395: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-395 RED: tip/product signed sentinel 0_usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}

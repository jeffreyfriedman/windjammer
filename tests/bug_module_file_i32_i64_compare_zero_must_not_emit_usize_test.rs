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

//! i32/i64 sentinel compares (`ci < 0`, `parent >= 0`) must not emit `0_usize`.
//!
//! zero literal when the same function also indexes with `as usize` and walks
//! `.len()` with a usize counter (including a shadowed `ci`).
//! isolate uses `parent: usize`, so `0_usize` is type-correct there.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_i32_i64_compare_zero_must_not_emit_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !signed_zero_poisoned(rs),
        "RED signed sentinel compare emitted 0_usize:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

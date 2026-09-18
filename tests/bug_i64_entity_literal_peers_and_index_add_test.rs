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
))]

//! P3.369: i64 entity ids must peer-drive literals (`old_parent >= 0` → `_i64`);
//! `idx + 1` index arith with `(i * 4) as i64` must cast base to usize.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod scene
pub mod pixels
"#;

const SCENE: &str = r#"
pub struct Scene {
    pub parents: Vec<i64>,
}

impl Scene {
    pub fn check_parent(self) {
        let old_parent = self.parents[0]
        if old_parent >= 0 {
            let _ = old_parent
        }
    }
}

pub fn list_for_entity(entity: i64) {
    let _ = entity
}

pub fn call_five() {
    list_for_entity(5)
}
"#;

const PIXELS: &str = r#"
pub struct Frame {
    pub data: Vec<f32>,
}

pub fn sample_at(frame: Frame, i: int) {
    let idx = (i * 4) as i64
    let _ = frame.data[idx + 1]
}
"#;

#[test]
fn i64_entity_literal_peers_and_index_add_must_cast() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("scene.wj", SCENE);
    test.add_file("pixels.wj", PIXELS);
    let map = test.compile().expect("P3.369 compile");
    let scene = map.get("scene.rs").expect("scene.rs");
    let pixels = map.get("pixels.rs").expect("pixels.rs");
    assert!(
        !scene.contains(">= 0_i32"),
        "P3.369: i64 compare must not use _i32 literal:\n{scene}"
    );
    assert!(
        scene.contains(">= 0_i64") || scene.contains(">= 0 as i64"),
        "P3.369: i64 compare needs i64 literal/cast:\n{scene}"
    );
    assert!(
        scene.contains("list_for_entity(5_i64")
            || scene.contains("list_for_entity(5 as i64")
            || scene.contains("list_for_entity(5_i64"),
        "P3.369: int literal into i64 formal:\n{scene}"
    );
    assert!(
        !pixels.contains("idx + 1 as usize") && !pixels.contains("idx + 1_usize"),
        "P3.369: must not add usize to i64 index base:\n{pixels}"
    );
    assert!(
        pixels.contains("as usize") && pixels.contains("idx"),
        "P3.369: index offset must cast i64 base to usize:\n{pixels}"
    );
    test.cargo_check().expect("P3.369 cargo-check");
}

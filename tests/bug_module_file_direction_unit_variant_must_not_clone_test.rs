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

//! Copy `Direction::PosX` must not emit `.clone()` (path miss).
//!
//! tip list covers FaceDirection / AlertLevel / NPCBehavior /
//! ChunkLifecycleState / MaterialNodeKind, but product
//! gen/voxel/meshing.rs still has `Direction::PosX.clone()`.
//! WJ source is `direction: Direction::PosX`. Twin of .

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Direction {
    PosX,
    NegX,
}

pub struct VoxelFace {
    pub direction: Direction,
    pub color_id: u8,
}

pub fn east_face(color_id: u8) -> VoxelFace {
    VoxelFace {
        direction: Direction::PosX,
        color_id: color_id,
    }
}

pub fn flip(dir: Direction) -> Direction {
    Direction::NegX
}
"#;

#[test]
fn module_file_direction_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("Direction::PosX.clone()") || rs.contains("Direction::NegX.clone()");
    assert!(
        !bad,
        "Copy Direction unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

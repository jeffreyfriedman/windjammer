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

//! Copy enum unit variants must not emit `.clone()`.
//!
//! Tip output for a game project:
//!   `FaceDirection::PosX.clone()` (mesh_generator)
//!   `AlertLevel::Suspicious.clone()` / `NPCBehavior::Patrol.clone()`
//!   `ChunkLifecycleState::Loading.clone()` / `MaterialNodeKind::ColorNode.clone()`
//! WJ source is `FaceDirection::PosX` (Copy). Prefer the bare variant.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Face {
    PosX,
    NegX,
}

pub fn east() -> Face {
    Face::PosX
}

pub fn assign(cur: Face) -> Face {
    Face::NegX
}
"#;

#[test]
fn module_file_copy_enum_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("Face::PosX.clone()") || rs.contains("Face::NegX.clone()");
    assert!(
        !bad,
        "Copy enum variant cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

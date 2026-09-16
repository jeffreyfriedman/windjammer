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

//! P3.309: i32 loop counter vs int literal / i32 const / i32 field must stay i32 — no `3_i64 as i64`.
//!
//! Product (breach-protocol / windjammer-game-core):
//!   `while dy < 3` with `dy: i32` → `while dy < (3_i64 as i64)`
//!   `while i < MANNEQUIN_BONE_COUNT` → `while i < (MANNEQUIN_BONE_COUNT as i64)`
//!   `while ty < self.template.height` → `while ty < (self.template.height as i64)`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod camera
pub mod bones
pub mod locomotion
"#;

const CAMERA: &str = r#"
pub fn scan_rows() -> i32 {
    let mut dy: i32 = 0
    while dy < 3 {
        dy = dy + 1
    }
    dy
}
"#;

const LOCOMOTION: &str = r#"
// Product: tps_camera `let mut dy = 0` inferred i32, `while dy < 3`.
pub fn collides_pivot() -> bool {
    let mut dy = 0
    while dy < 3 {
        dy = dy + 1
    }
    dy == 3
}
"#;

const BONES: &str = r#"
pub const MANNEQUIN_BONE_COUNT: i32 = 64

pub struct Template {
    pub height: i32,
}

pub fn count_cells(t: Template) -> i32 {
    let mut i: i32 = 0
    while i < MANNEQUIN_BONE_COUNT {
        i = i + 1
    }
    let mut ty: i32 = 0
    while ty < t.height {
        ty = ty + 1
    }
    i + ty
}
"#;

fn p308_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("camera.wj", CAMERA);
    test.add_file("bones.wj", BONES);
    test.add_file("locomotion.wj", LOCOMOTION);
    test
}

fn bad_i32_while_compare(rs: &str) -> bool {
    rs.contains("_i64 as i64)")
        || rs.contains("while dy < 3_i64")
        || rs.contains("while dy < (3_i64")
        || (rs.contains(" as i64)")
            && (rs.contains("while dy <") || rs.contains("while i <") || rs.contains("while ty <")))
}

#[test]
fn i32_while_compare_must_not_cast_rhs_to_i64() {
    let mut test = p308_fixture();
    let map = test
        .compile()
        .expect("P3.309 multipass compile should succeed");
    let camera_rs = map.get("camera.rs").expect("camera.rs");
    let bones_rs = map.get("bones.rs").expect("bones.rs");
    let loco_rs = map.get("locomotion.rs").expect("locomotion.rs");
    let combined = format!("{camera_rs}\n{bones_rs}\n{loco_rs}");

    if bad_i32_while_compare(&combined) {
        eprintln!("P3.309 RED emit:\n{combined}");
    }

    assert!(
        !bad_i32_while_compare(&combined),
        "i32 while compare must not emit i64 casts on literal/const/i32 field RHS"
    );

    test.cargo_check().expect(
        "P3.309: i32 loop bounds must cargo-check without i32 vs i64 compare casts",
    );
}

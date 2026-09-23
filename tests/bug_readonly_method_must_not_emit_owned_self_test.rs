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

//! P3.424: nested `self.quality.steps` in a struct literal must not force owned `self`.
//! Product: voxel_gpu_passes `update_raymarch_params(self)` via `self.current_quality.max_raymarch_steps`.
//! Binding the Copy parent first (`let q = self.current_quality`) correctly yields `&self`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Quality {
    pub steps: u32,
}

pub struct Uni {
    pub a: f32,
    pub b: u32,
}

impl Uni {
    pub fn to_bytes(self) -> Vec<u8> {
        Vec::new()
    }
}

extern fn gpu_update(h: i32, p: *const u8, n: usize)

pub struct Resources {
    pub handle: i32,
}

pub struct Renderer {
    pub world_size: f32,
    pub current_quality: Quality,
    pub resources: Resources,
}

impl Renderer {
    pub fn update_params(self) {
        let params = Uni {
            a: self.world_size,
            b: self.current_quality.steps,
        }
        let bytes = params.to_bytes()
        gpu_update(self.resources.handle, bytes.as_ptr(), bytes.len())
    }

    pub fn tick(self) {
        self.update_params()
        self.update_params()
    }
}
"#;

#[test]
fn nested_self_field_in_struct_lit_must_not_force_owned_self() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.424 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.424 MultiFile lib.rs:\n{rs}");
    assert!(
        rs.contains("fn update_params(&self)") || rs.contains("fn update_params(&mut self)"),
        "P3.424 RED: nested self.field.subfield must not force owned self:\n{rs}"
    );
    assert!(
        !rs.contains("fn update_params(self)"),
        "P3.424 RED: owned self on nested-field read method:\n{rs}"
    );
    test.cargo_check().expect("P3.424 cargo-check");
}

#[test]
fn tip_out_voxel_gpu_passes_update_raymarch_must_not_be_owned_self() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/rendering/voxel_gpu_passes.rs");
    assert!(game.exists(), "missing tip product");
    let text = std::fs::read_to_string(&game).expect("product");
    assert!(
        !text.contains("fn update_raymarch_params(self)"),
        "P3.424 RED: tip still emits owned self for update_raymarch_params"
    );
}

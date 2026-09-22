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

//! P3.423: `for pass in self.passes { self.update() }` under `&mut self` must not emit
//! `for pass in &self.passes` then `self.update()` (E0505/E0507). Clone/copy the
//! collection first — product: voxel_gpu_passes::update_all_params.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Pass {
    A,
    B,
}

pub struct Renderer {
    pub passes: Vec<Pass>,
    pub n: i32,
}

impl Renderer {
    pub fn update_a(self) {
        self.n = self.n + 1
    }

    pub fn update_b(self) {
        self.n = self.n + 2
    }

    pub fn update_all(self) {
        for pass in self.passes {
            match pass {
                Pass::A => self.update_a(),
                Pass::B => self.update_b(),
            }
        }
    }
}
"#;

#[test]
fn for_in_self_field_mut_method_must_clone_or_index() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.423 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.423 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("for pass in &self.passes")
        || rs.contains("for pass in &self.render_pipeline.passes");
    assert!(
        !bad,
        "P3.423 RED: for-in shared-borrows self.passes while body needs &mut self:\n{rs}"
    );
    test.cargo_check().expect("P3.423 cargo-check");
}

#[test]
fn tip_out_voxel_gpu_passes_update_all_must_not_borrow_passes() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/rendering/voxel_gpu_passes.rs");
    assert!(game.exists(), "missing tip product {}", game.display());
    let text = std::fs::read_to_string(&game).expect("product");
    assert!(
        !text.contains("for pass in &self.render_pipeline.passes"),
        "P3.423 RED: tip still borrows passes under &mut self methods"
    );
}

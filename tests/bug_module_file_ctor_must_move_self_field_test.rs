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

//! WDB-414: `Type::new(self.scene)` must move the field, not `self.scene.clone()`.
//!
//! Product:
//!   `CsgVoxelizer::new(self.scene.clone())` in rifter_quarter / cathedral / humanoid_demo
//! WJ is `CsgVoxelizer::new(self.scene)` — scene is not used after.
//! Distinct from WDB-360 (`encode(self.grid.clone())`), WDB-410 (wither reconstruct),
//! and WDB-407 (`Vec` `new` demote).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Scene {
    pub label: string,
}

pub struct Vox {
    pub scene: Scene,
}

impl Vox {
    pub fn new(scene: Scene) -> Vox {
        Vox { scene: scene }
    }
}

pub struct Demo {
    pub scene: Scene,
    pub grid: i32,
}

impl Demo {
    pub fn initialize(self) {
        let v = Vox::new(self.scene)
        self.grid = 1
    }
}
"#;

/// Product shape: MutBorrowed helpers before `Vox::new(self.scene)` must not
/// demote `initialize` to `&mut self` (which forces `.clone()`).
const SRC_WITH_HELPERS: &str = r#"
pub struct Scene {
    pub label: string,
}

impl Scene {
    pub fn touch(self) {
        // no-op
    }
}

pub struct Vox {
    pub scene: Scene,
}

impl Vox {
    pub fn new(scene: Scene) -> Vox {
        Vox { scene: scene }
    }
}

pub struct Demo {
    pub scene: Scene,
    pub grid: i32,
    pub ready: bool,
}

impl Demo {
    pub fn setup_materials(self) {
        self.ready = true
    }

    pub fn build_scene(self) {
        self.scene.touch()
    }

    pub fn initialize(self) {
        self.setup_materials()
        self.build_scene()
        let v = Vox::new(self.scene)
        self.grid = 1
        self.ready = true
    }
}
"#;

#[test]
fn module_file_ctor_must_move_self_field() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("self.scene.clone()");
    assert!(
        !cloned,
        "constructor cloned self.scene:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_ctor_after_mut_helpers_must_move_self_field() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_WITH_HELPERS);
    let map = test.compile().expect("helpers compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("helpers MultiFile lib.rs:\n{rs}");
    assert!(
        rs.contains("pub fn initialize(mut self)") || rs.contains("pub fn initialize(self)"),
        "initialize demoted away from owned self:\n{rs}"
    );
    assert!(
        !rs.contains("self.scene.clone()"),
        "helpers path cloned self.scene:\n{rs}"
    );
    assert!(
        !rs.contains("pub fn initialize(&mut self)"),
        "initialize forced to &mut self by MutBorrowed helpers:\n{rs}"
    );
    test.cargo_check().expect("helpers cargo-check");
}

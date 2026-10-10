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

//!
//!   `matches!(self.mesh.clone(), Some(_))` in `has_mesh`.
//! Prefer `self.mesh.is_some()` or `matches!(&self.mesh, Some(_))`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Mesh {
    pub id: i32,
}

pub struct Prim {
    pub mesh: Option<Mesh>,
}

pub fn has_mesh(prim: Prim) -> bool {
    match prim.mesh {
        Some(_) => true,
        None => false,
    }
}
"#;

/// `Option` presence check. The compiler emitted `matches!(self.mesh.clone(), Some(_))`.
const SRC_SELF_PRESENCE: &str = r#"
pub struct UsdMesh {
    pub vertex_count: i32,
    pub name: string,
}

pub struct UsdNode {
    pub mesh: Option<UsdMesh>,
}

impl UsdNode {
    pub fn has_mesh(self) -> bool {
        match self.mesh {
            Some(_) => true,
            None => false,
        }
    }
}
"#;

#[test]
fn module_file_option_presence_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()") && rs.contains("Some(");
    assert!(
        !bad,
        "RED: Option presence check cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_self_option_presence_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_SELF_PRESENCE);
    let map = test.compile().expect("self presence compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("self-presence lib.rs:\n{rs}");
    let bad = rs.lines().any(|line| {
        line.contains("matches!(") && line.contains(".clone()") && line.contains("Some")
    });
    assert!(
        !bad,
        "RED: &self Option presence cloned:\n{rs}"
    );
    assert!(
        rs.contains("has_mesh")
            && (rs.contains("is_some()")
                || rs.contains("matches!(&self.mesh")
                || rs.contains("matches!(self.mesh, Some")),
        "expected presence check without clone. Got:\n{rs}"
    );
    test.cargo_check().expect("self presence cargo-check");
}


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

//! P3.302 / E0053: cross-file trait defines owned `Vec<T>` formal but thin impl
//! forwarder demotes to `&Vec<T>` when the callee borrows — trait impl signature
//! must match the trait definition (product: `RenderPort::upload_materials`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn trait_impl_owned_vec_forward_must_match_trait_formal() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "mod.wj",
        r#"
pub mod ports
pub mod inner
pub mod renderer
"#,
    );
    t.add_file(
        "ports.wj",
        r#"
pub struct MaterialData {
    id: i32,
}

pub trait RenderPort {
    fn upload_materials(materials: Vec<MaterialData>)
}
"#,
    );
    t.add_file(
        "inner.wj",
        r#"
use crate::ports::MaterialData

pub fn absorb_materials(materials: Vec<MaterialData>) {
    let _ = materials.len()
}
"#,
    );
    t.add_file(
        "renderer.wj",
        r#"
use crate::ports::{MaterialData, RenderPort}
use crate::inner::absorb_materials

pub struct GameRenderer {
    ready: bool,
}

impl RenderPort for GameRenderer {
    fn upload_materials(materials: Vec<MaterialData>) {
        absorb_materials(materials)
        self.ready = true
    }
}
"#,
    );

    let out = t.compile().expect("multipass compile");
    let rs = out.get("renderer.rs").expect("renderer.rs");
    assert!(
        !rs.contains("materials: &Vec<MaterialData>"),
        "trait impl must not demote owned Vec formal to &Vec (E0053)\n{}",
        rs
    );
    assert!(
        rs.contains("materials: Vec<MaterialData>"),
        "trait impl must keep owned Vec formal\n{}",
        rs
    );
    t.cargo_check().expect("cargo check fixture");
}

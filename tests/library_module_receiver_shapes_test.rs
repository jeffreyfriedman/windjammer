#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Receiver shapes found in a large library build (rustc E0596 / E0594): a method in a module inside a
//! subdirectory that writes a map field through an implicit `self`, and a trait impl whose declared `&mut self`
//! must not be replaced by a same-named trait of another module.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn implicit_self_writing_a_map_field_in_a_nested_module_is_mut() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "panel/tree.wj",
        "use std::collections::HashMap\n\npub struct Tree {\n    names: HashMap<i64, string>,\n    open: HashMap<i64, bool>,\n}\n\nimpl Tree {\n    pub fn new() -> Tree {\n        Tree { names: HashMap::new(), open: HashMap::new() }\n    }\n\n    pub fn expand(id: i64) {\n        self.open.insert(id, true)\n    }\n\n    pub fn forget(id: i64) {\n        self.open.remove(id)\n    }\n\n    pub fn expand_all() {\n        for (id, _) in self.names {\n            self.open.insert(id as i64, true)\n        }\n    }\n}\n",
    );
    let map = test.compile().expect("compile");
    let rs = map.get("panel/tree.rs").expect("panel/tree.rs");
    for f in ["expand", "forget", "expand_all"] {
        assert!(rs.contains(&format!("pub fn {f}(&mut self")), "{f} writes a map field: `&mut self`\n{rs}");
    }
    test.cargo_check().expect("cargo-check");
}

#[test]
fn a_declared_mut_receiver_of_a_trait_impl_survives_a_homonym_trait() {
    let mut test = MultiFileTest::new();
    test.add_file("legacy/port.wj", "pub trait Pointer {\n    fn delta() -> f32\n}\n");
    test.add_file(
        "ports/port.wj",
        "pub trait Pointer {\n    fn delta(&mut self, dx: f32)\n}\n\npub struct Fake {\n    pub acc: f32,\n}\n\nimpl Pointer for Fake {\n    fn delta(&mut self, dx: f32) {\n        self.acc = self.acc + dx\n    }\n}\n",
    );
    let map = test.compile().expect("compile");
    let rs = map.get("ports/port.rs").expect("ports/port.rs");
    assert!(rs.contains("fn delta(&mut self, dx: f32) {"), "the impl keeps its declared `&mut self`:\n{rs}");
    test.cargo_check().expect("cargo-check");
}

/// An impl split from its struct's file reads a Copy field of `self` into a module function: the field type comes
/// from the struct's own module, so the method borrows `self` (it was emitted owned, rustc E0507 at its `&mut self`
/// callers).
#[test]
fn a_split_impl_passing_a_copy_field_to_a_module_function_borrows_self() {
    let mut test = MultiFileTest::new();
    test.add_file("ffi/mod.wj", "pub mod api\n");
    test.add_file("ffi/api.wj", "pub fn set_buffers(a: u32, b: u32) {\n    let _ = a + b\n}\n\npub fn set_enabled(on: bool) {\n    let _ = on\n}\n");
    test.add_file("render/mod.wj", "pub mod config\npub mod renderer\npub mod passes\n");
    test.add_file("render/config.wj", "pub struct Resources {\n    pub gbuffer: u32,\n    pub color: u32,\n}\n");
    test.add_file(
        "render/renderer.wj",
        "use crate::render::config::Resources\n\npub struct Renderer {\n    pub resources: Resources,\n    pub width: u32,\n    pub ready: bool,\n}\n",
    );
    test.add_file(
        "render/passes.wj",
        "use crate::render::renderer::Renderer\nuse crate::ffi::api as gpu\n\nimpl Renderer {\n    pub fn sync_host(self) {\n        gpu::set_buffers(self.resources.gbuffer, self.resources.color)\n        gpu::set_enabled(self.ready)\n    }\n\n    pub fn resize(self, w: u32) {\n        self.width = w\n        self.sync_host()\n    }\n}\n",
    );
    let map = test.compile().expect("compile");
    let rs = map.get("render/passes.rs").expect("render/passes.rs");
    assert!(rs.contains("pub fn sync_host(&self)"), "a method reading Copy fields borrows `self`:\n{rs}");
    assert!(rs.contains("pub fn resize(&mut self"), "{rs}");
    test.cargo_check().expect("cargo-check");
}

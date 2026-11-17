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

/// A struct indexed out of a `Vec` field is cloned even when another module declares a unit enum of the same name
/// (that enum is Copy; this file's struct is not: rustc E0507 when the homonym decided it).
#[test]
fn a_struct_element_is_cloned_despite_a_homonym_unit_enum_elsewhere() {
    let mut test = MultiFileTest::new();
    test.add_file("kinds/mod.wj", "pub mod presets\n");
    test.add_file("kinds/presets.wj", "pub enum Preset {\n    Warm,\n    Cold,\n}\n\npub fn is_warm(p: Preset) -> bool {\n    match p {\n        Preset::Warm => true,\n        Preset::Cold => false,\n    }\n}\n");
    test.add_file("tools/mod.wj", "pub mod picker\n");
    test.add_file(
        "tools/picker.wj",
        "pub struct Preset {\n    pub name: string,\n    pub count: i32,\n}\n\nimpl Preset {\n    pub fn copy_of(other: Preset) -> Preset {\n        Preset { name: other.name, count: other.count }\n    }\n}\n\npub struct Picker {\n    pub items: Vec<Preset>,\n    pub current: Preset,\n}\n\nimpl Picker {\n    pub fn load(index: i32) {\n        if index >= 0 && (index as usize) < self.items.len() {\n            let preset = self.items[index as usize]\n            self.current = Preset::copy_of(preset)\n        }\n    }\n}\n",
    );
    let map = test.compile().expect("compile");
    let rs = map.get("tools/picker.rs").expect("tools/picker.rs");
    assert!(rs.contains("let preset = self.items[index as usize].clone();"), "a non-Copy element is cloned:\n{rs}");
    test.cargo_check().expect("cargo-check");
}

/// A method's emitted formals are read from its own signature (`Zone::contains`), never from a same-named free function
/// of the module or crate (`contains(lo, items)` borrows its second formal): the engine's `ReverbZone::contains` got
/// `x: &f32` and every caller passing `f32` failed (rustc E0308).
#[test]
fn a_method_does_not_take_the_formals_of_a_same_named_free_function() {
    let mut test = MultiFileTest::new();
    test.add_file("util/mod.wj", "pub mod lookup\n");
    test.add_file("util/lookup.wj", "pub fn contains(lo: f32, items: Vec<f32>) -> bool {\n    items.len() > 0 && lo > 0.0\n}\n");
    test.add_file("audio/mod.wj", "pub mod zone\n");
    test.add_file(
        "audio/zone.wj",
        "pub struct Zone {\n    pub min_x: f32,\n    pub max_x: f32,\n}\n\nimpl Zone {\n    pub fn contains(self, x: f32, y: f32) -> bool {\n        x >= self.min_x && x <= self.max_x && y > 0.0\n    }\n\n    pub fn blend(self, x: f32, y: f32) -> f32 {\n        if !self.contains(x, y) { return 0.0 }\n        1.0\n    }\n}\n",
    );
    let map = test.compile().expect("compile");
    let rs = map.get("audio/zone.rs").expect("audio/zone.rs");
    assert!(rs.contains("pub fn contains(&self, x: f32, y: f32)"), "the method keeps its own by-value f32 formals:\n{rs}");
    test.cargo_check().expect("cargo-check");
}

/// Stage 1 class 1 (docs/design/ownership-decided-once.md, OWN-PUB-TEXT-UNUSED): an unused `string` formal of a `pub`
/// free function is owned in the solver, so a caller in another file passes an owned value and the emitted callee
/// agrees (DRAFT, unbuilt).
#[test]
fn an_unused_pub_text_formal_is_owned_for_every_caller() {
    let mut test = MultiFileTest::new();
    test.add_file("api/mod.wj", "pub mod text\n");
    test.add_file("api/text.wj", "pub fn label(name: string) -> int {\n    3\n}\n");
    test.add_file("app/mod.wj", "pub mod caller\n");
    test.add_file(
        "app/caller.wj",
        "use crate::api::text::label\n\npub fn run(n: string) -> int {\n    let a = label(\"x\")\n    let b = label(n)\n    let c = label(n)\n    a + b + c\n}\n",
    );
    let map = test.compile().expect("compile");
    let api = map.get("api/text.rs").expect("api/text.rs");
    assert!(api.contains("pub fn label(name: String)") || api.contains("pub fn label(_name: String)"), "{api}");
    test.cargo_check().expect("cargo-check");
}

#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! An extern's `string` parameter is an `FfiString` at the boundary: every call converts its argument with the
//! runtime's `string_to_ffi`, also when the call names the extern through its parent module (`ffi::draw_label`
//! for an extern of `ffi/api.wj`, emitted `ffi::api::draw_label`). It was passed as a `String` (rustc E0308
//! expected `FfiString`, found `String`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn extern_string_parameter_converts_through_a_parent_module_call() {
    let mut test = MultiFileTest::new();
    test.add_file("ffi/mod.wj", "pub mod api\n");
    test.add_file("ffi/api.wj", "extern fn draw_label(handle: u32, text: string, size: f32)\n");
    test.add_file("render/mod.wj", "pub mod text\n");
    test.add_file(
        "render/text.wj",
        "use crate::ffi\n\npub fn label(text: string, size: f32) {\n    ffi::draw_label(0, text, size)\n}\n\npub fn shout(text: string) {\n    let loud = text + \"!\"\n    ffi::draw_label(1, loud, 2.0)\n}\n",
    );
    let map = test.compile().expect("compile");
    let rs = map.get("render/text.rs").expect("render/text.rs");
    assert_eq!(rs.matches("windjammer_runtime::ffi::string_to_ffi(").count(), 2, "both calls convert their text:\n{rs}");
    test.cargo_check().expect("cargo-check");
}

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

//! P3.753: a `u32` product cast to `u32` must not insert `as f32`.
//!
//! `vgs_rasterization.wj` writes
//! `create_empty_storage_buffer((self.width * self.height * 16) as u32)`
//! with `width` and `height` as `u32`. Tip-out emits
//! `(((self.width * self.height) as f32 * 16_u32) as f32) as u32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn u32_product_cast_must_not_insert_f32() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Raster {
    pub width: u32,
    pub height: u32,
    gbuffer_id: u32,
}

pub fn create_empty_storage_buffer(size: u32) -> u32 {
    size
}

impl Raster {
    pub fn create_gbuffer(self) {
        self.gbuffer_id = create_empty_storage_buffer((self.width * self.height * 16) as u32)
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.753: u32 product cast fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !body.contains("as f32"),
        "u32 product cast must not insert as f32; got:\n{body}"
    );
}

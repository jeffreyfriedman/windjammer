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

//! Regression: int mul into u32 formal must cast product.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_cast(body: &str) -> bool {
    body.contains("48_i32 as u32") || body.contains("* 48 as u32")
}

fn product_reaches_u32(body: &str) -> bool {
    body.contains("48_u32") || body.contains("as u32")
}

#[test]
fn int_mul_into_u32_formal_must_cast_product() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn create_empty_storage_buffer(size: u32) -> u32 {
    size
}
pub fn alloc() -> u32 {
    let max_triangles: i32 = 100000
    create_empty_storage_buffer(max_triangles * 48)
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.734: int*48 into u32 formal must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_cast(body),
        "cast must cover the product, not only the literal 48; got:\n{body}"
    );
    assert!(
        body.contains("as u32"),
        "i32 product into u32 formal must cast to u32; got:\n{body}"
    );
}

#[test]
fn untyped_mul_into_u32_formal_uses_u32_literal_width() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn create_empty_storage_buffer(size: u32) -> u32 {
    size
}
pub fn alloc() -> u32 {
    let max_triangles = 100000
    create_empty_storage_buffer(max_triangles * 48)
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.734: untyped mul into u32 formal must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_cast(body),
        "u32 formal must not emit 48_i32 as u32; got:\n{body}"
    );
    assert!(
        product_reaches_u32(body),
        "u32 formal must drive u32 width; got:\n{body}"
    );
}

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

//! P3.734: `n * 48` into a `u32` formal must cast the product, not `48 as u32`.
//!
//! `hybrid_renderer.wj` writes `create_empty_storage_buffer(max_triangles * 48)`.
//! Tip-out emits `(max_triangles * 48_i32 as u32)`, which is `i32 * u32`
//! (expected `i32`, found `u32` on the literal).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_cast(body: &str) -> bool {
    body.contains("48_i32 as u32") || body.contains("* 48 as u32")
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
    let max_triangles = 100000
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
        "product into u32 formal must cast to u32; got:\n{body}"
    );
}

fn search_roots() -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    let mut walked = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn int_mul_into_u32_formal_tip_out_hybrid_renderer() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/rendering/hybrid_renderer.rs",
        ));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("hybrid_renderer.rs");
        if bad_cast(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.734: hybrid_renderer.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.734 RED: tip-out casts only the 48 literal:\n  {}",
        bad_paths.join("\n  ")
    );
}

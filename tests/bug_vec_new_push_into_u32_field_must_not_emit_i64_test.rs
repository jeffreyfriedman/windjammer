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

//! P3.780: `Vec::new()` plus `push` of an untyped int, stored in `Vec<u32>`,
//! must not emit `Vec<i64>`.
//!
//! `streaming_coordinator_test.wj` pushes `100` and `200` into a local vec
//! that is stored in `StreamingTileBatch.add_ids: Vec<u32>`. Tip-out emits
//! `let mut adds: Vec<i64>` and `adds.push(100_i64)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i64_vec(body: &str) -> bool {
    body.contains("Vec<i64>") || body.contains("100_i64")
}

#[test]
fn vec_new_push_into_u32_field_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Batch {
    pub add_ids: Vec<u32>,
}

pub fn make() -> Batch {
    let mut adds = Vec::new()
    adds.push(100)
    adds.push(200)
    Batch { add_ids: adds }
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.780: Vec<u32> push fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_vec(body),
        "push into a Vec<u32> field must not emit i64; got:\n{body}"
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
fn vec_new_push_into_u32_field_tip_out_streaming_coordinator() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/world/streaming_coordinator_test.rs",
        ));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("streaming_coordinator_test.rs");
        if bad_i64_vec(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(
        saw,
        "P3.780: streaming_coordinator_test.rs product file missing"
    );
    assert!(
        bad_paths.is_empty(),
        "P3.780 RED: tip-out tile ids are Vec<i64>:\n  {}",
        bad_paths.join("\n  ")
    );
}

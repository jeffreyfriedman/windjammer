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

//! P3.779: `f32 * ((usize - 1) as f32)` must not compare the length to `1_i32`
//! or leave the cast on the product.
//!
//! `ui/layout.wj` writes `if child_count > 1 { self.gap * ((child_count - 1) as f32) }`
//! with `child_count` from `.len()` and `gap: f32`. Tip-out emits
//! `child_count > 1_i32` and `self.gap as f32 * (child_count - 1_usize) as f32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_gap(body: &str) -> bool {
    body.contains("> 1_i32") || body.contains("* (child_count - 1_usize) as f32")
}

#[test]
fn f32_times_len_minus_one_cast_must_parenthesize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Flex {
    pub gap: f32,
    pub children: Vec<i32>,
}

pub fn total_gap(container: Flex) -> f32 {
    let child_count = container.children.len()
    if child_count > 1 {
        container.gap * ((child_count - 1) as f32)
    } else {
        0.0
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.779: flex gap fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_gap(body),
        "f32 times (len - 1) as f32 must not use 1_i32 or an ungrouped cast; got:\n{body}"
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
fn f32_times_len_minus_one_cast_tip_out_layout() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ui/layout.rs"));
    }
    paths.push(std::path::PathBuf::from(
        "/Users/jeffreyfriedman/src/wj/windjammer-game/windjammer-game-core/gen/ui/layout.rs",
    ));
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("layout.rs");
        if text.contains("child_count > 1_i32")
            || text.contains("self.gap as f32 * (child_count - 1_usize) as f32")
        {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.779: layout.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.779 RED: tip-out flex gap mixes i32 and an ungrouped cast:\n  {}",
        bad_paths.join("\n  ")
    );
}

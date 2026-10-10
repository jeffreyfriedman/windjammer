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

//! P3.789: a `for`-in counter later cast to `f32` must not be `i64`.
//!
//! `visual_verification.wj` `non_black_percentage` writes `let mut count = 0`,
//! increments it inside `for p in self.pixels`, then returns
//! `(count as f32) / (self.pixels.len() as f32)`. Tip-out emits
//! `let mut count = 0_i64`. The counter is not a vec index.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i64_count(body: &str) -> bool {
    body.contains("count = 0_i64") || body.contains("count: i64")
}

#[test]
fn for_in_f32_ratio_count_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Shot {
    pub pixels: Vec<f32>,
}

pub fn ratio(self, threshold: f32) -> f32 {
    let mut count = 0
    for p in self.pixels {
        if p > threshold {
            count = count + 1
        }
    }
    let total = self.pixels.len()
    if total == 0 {
        return 0.0
    }
    (count as f32) / (total as f32)
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.789: for-in f32 ratio fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_count(body),
        "for-in counter cast to f32 must not be i64; got:\n{body}"
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
fn for_in_f32_ratio_count_tip_out_visual_verification() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(
            dir.join(
                "windjammer-game/windjammer-game-core/gen/rendering/visual_verification.rs",
            ),
        );
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("visual_verification.rs");
        if text.contains("let mut count = 0_i64") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(
        saw,
        "P3.789: visual_verification.rs product file missing"
    );
    assert!(
        bad_paths.is_empty(),
        "P3.789 RED: tip-out for-in ratio counter is 0_i64:\n  {}",
        bad_paths.join("\n  ")
    );
}

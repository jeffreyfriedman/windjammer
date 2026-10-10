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

//! P3.785: `while i + needle.len() <= haystack.len()` must not index with `i64`.
//!
//! `ecs_inspector_test.wj` `string_contains` writes `let mut i = 0` then
//! `haystack[i + j]` while `i + needle.len() <= haystack.len()`. Tip-out emits
//! `let mut i = 0_i64` and `haystack.as_bytes()[i + (j as i64)]`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i64_index(body: &str) -> bool {
    body.contains("i = 0_i64") || body.contains("j as i64") || body.contains("as i64)")
}

#[test]
fn string_index_plus_len_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub fn string_contains(haystack: string, needle: string) -> bool {
    let mut i = 0
    while i + needle.len() <= haystack.len() {
        let mut matched = true
        let mut j = 0
        while j < needle.len() {
            if haystack[i + j] != needle[j] {
                matched = false
            }
            j = j + 1
        }
        if matched {
            return true
        }
        i = i + 1
    }
    false
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.785: string index fixture must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i64_index(body),
        "string index counter compared to .len() must not be i64; got:\n{body}"
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
fn string_index_plus_len_tip_out_ecs_inspector() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/editor/ecs_inspector_test.rs"),
        );
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("ecs_inspector_test.rs");
        if text.contains("let mut i = 0_i64") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(
        saw,
        "P3.785: ecs_inspector_test.rs product file missing"
    );
    assert!(
        bad_paths.is_empty(),
        "P3.785 RED: tip-out string index counter is 0_i64:\n  {}",
        bad_paths.join("\n  ")
    );
}

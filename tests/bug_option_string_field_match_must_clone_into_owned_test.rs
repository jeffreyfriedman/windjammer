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

//! P3.738: matching `Option<string>` through `&self` must clone into owned `Option<string>`.
//!
//! `animation/controller.wj` `current_animation` matches `self.current_animation` and
//! returns `Some(s)`. Tip-out emits `match &self.current_animation { Some(s) => Some(s) }`,
//! which is `Option<&String>` (expected `String`, found `&String`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn borrowed_some_without_clone(body: &str) -> bool {
    body.contains("match &self.current_animation") && body.contains("Some(s) => Some(s)")
        && !body.contains("Some(s.clone())")
}

#[test]
fn option_string_field_match_must_clone_into_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Anim {
    pub current_animation: Option<string>,
}
impl Anim {
    pub fn current_animation(&self) -> Option<string> {
        match self.current_animation {
            Some(s) => Some(s),
            None => None,
        }
    }
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.738: Option<string> field match must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !borrowed_some_without_clone(body),
        "borrowed Option<String> match must clone into owned Option; got:\n{body}"
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
fn option_string_field_match_tip_out_animation_controller() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/animation/controller.rs",
        ));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("controller.rs");
        if borrowed_some_without_clone(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.738: animation controller.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.738 RED: tip-out returns &String from Option field match:\n  {}",
        bad_paths.join("\n  ")
    );
}

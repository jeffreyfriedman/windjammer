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

//! P3.792: `for i in 0..vec.len()` must stay `usize` beside an `i32` tally.
//!
//! `scripting/components.wj` `active_count` returns `i32` and writes
//! `let mut count: i32 = 0` then `for i in 0..self.components.len()` and
//! `self.components[i]`. Tip-out emits
//! `for i in 0_i32..(self.components.len() as i32)` and `components[(i as usize)]`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i32_index(body: &str) -> bool {
    body.contains("0_i32..") || body.contains("len() as i32") || body.contains("i as usize")
}

#[test]
fn index_loop_beside_i32_tally_must_stay_usize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Item {
    pub active: bool,
}

pub fn active_count(items: Vec<Item>) -> i32 {
    let mut count: i32 = 0
    for i in 0..items.len() {
        if items[i].active {
            count = count + 1
        }
    }
    count
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.792: index loop beside i32 tally must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i32_index(body),
        "for i in 0..len() used as an index must stay usize beside an i32 tally; got:\n{body}"
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
fn index_loop_beside_i32_tally_tip_out_components() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/scripting/components.rs"),
        );
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("components.rs");
        if text.contains("0_i32..(self.components.len() as i32)") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.792: components.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.792 RED: tip-out index loop is an i32 range:\n  {}",
        bad_paths.join("\n  ")
    );
}

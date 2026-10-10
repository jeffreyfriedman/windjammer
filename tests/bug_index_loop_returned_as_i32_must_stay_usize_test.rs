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

//! P3.795: `for i in 0..len()` must stay `usize` when the return is `i as i32`.
//!
//! `assets/pipeline.wj` `find_texture` writes `return i as i32` inside
//! `for i in 0..self.textures.len()` and indexes `self.textures[i]`.
//! Tip-out emits `for i in 0_i32..(self.textures.len() as i32)` and
//! `self.textures[(i as usize)]`. The source cast says `i` is not already `i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

fn bad_i32_range(body: &str) -> bool {
    body.contains("0_i32..") || body.contains("len() as i32") || body.contains("i as usize")
}

#[test]
fn index_loop_returned_as_i32_must_stay_usize() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub struct Tex {
    pub name: string,
}

pub fn find_texture(textures: Vec<Tex>, name: string) -> i32 {
    for i in 0..textures.len() {
        if textures[i].name == name {
            return i as i32
        }
    }
    -1
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.795: index loop returned as i32 must transpile");
    let body = map
        .get("mod.rs")
        .unwrap_or_else(|| panic!("mod.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        !bad_i32_range(body),
        "for i in 0..len() must stay usize when the source returns i as i32; got:\n{body}"
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
fn index_loop_returned_as_i32_tip_out_pipeline() {
    let mut paths = Vec::new();
    for dir in search_roots() {
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/assets/pipeline.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("pipeline.rs");
        if text.contains("0_i32..(self.textures.len() as i32)") {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.795: pipeline.rs product file missing");
    assert!(
        bad_paths.is_empty(),
        "P3.795 RED: tip-out find_texture index loop is an i32 range:\n  {}",
        bad_paths.join("\n  ")
    );
}

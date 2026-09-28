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
    feature = "codegen_tests",
))]

//! WDB-424: two `match`es on the same non-Copy field must borrow, not `.clone()`.
//!
//! Product `physics/jolt/world.rs`:
//!   `match body.shape.clone()` twice (ShapeType includes `ConvexHull(Vec<Vec3>)`).
//! WJ is `match body.shape` twice.
//! Distinct from WDB-417 (match-arm owned formal into fn), WDB-405 (unit enum
//! `.clone()`), and WDB-416 (`a.clone().as_float()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Shape {
    Box(f32),
    Hull(Vec<f32>),
}

pub struct Body {
    pub shape: Shape,
}

pub fn kind_and_x(body: Body) -> f32 {
    let kind = match body.shape {
        Shape::Box(_) => 0.0,
        Shape::Hull(_) => 1.0,
    }
    let x = match body.shape {
        Shape::Box(v) => v,
        Shape::Hull(pts) => pts[0],
    }
    kind + x
}
"#;

#[test]
fn wdb424_module_file_repeated_match_noncopy_field_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-424 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-424 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("shape.clone()");
    assert!(
        !cloned,
        "WDB-424 RED: repeated match cloned non-Copy field:\n{rs}"
    );
    test.cargo_check().expect("WDB-424 cargo-check");
}

fn wdb424_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
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
fn wdb424_tip_out_game_core_body_shape_match_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb424_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/world.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/physics/jolt/world.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/physics/jolt/world.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//") && t.contains("body.shape.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-424: jolt world product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-424 RED: tip/product cloned body.shape for repeated match in:\n  {}",
        bad_paths.join("\n  ")
    );
}

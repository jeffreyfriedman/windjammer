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

//! WDB-416: owned enum formal must not `a.clone().as_float()` when WJ is `a.as_float()`.
//!
//! Product `visual_scripting/runtime.rs`:
//!   `let fa = a.clone().as_float();`
//! WJ is `let fa = a.as_float()` — `as_float(self)` consumes once.
//! Distinct from WDB-358 (`self.clone().method()`), WDB-362 (`].clone().method()`),
//! and WDB-409 (`set(name: &str)`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Val {
    F(f32),
    I(i32),
}

impl Val {
    pub fn as_float(self) -> f32 {
        match self {
            Val::F(v) => v,
            Val::I(v) => v as f32,
        }
    }
}

pub fn add(a: Val, b: Val) -> f32 {
    a.as_float() + b.as_float()
}
"#;

#[test]
fn wdb416_module_file_owned_enum_must_not_clone_before_as_float() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-416 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-416 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone().as_float()") || rs.contains("a.clone()") || rs.contains("b.clone()");
    assert!(
        !cloned,
        "WDB-416 RED: owned enum cloned before as_float:\n{rs}"
    );
    test.cargo_check().expect("WDB-416 cargo-check");
}

fn wdb416_search_roots() -> Vec<PathBuf> {
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
fn wdb416_tip_out_game_core_value_must_not_clone_before_as_float() {
    let mut paths = Vec::new();
    for dir in wdb416_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/visual_scripting/runtime.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/visual_scripting/runtime.rs"));
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
            !t.starts_with("//") && t.contains(".clone().as_float()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-416: visual_scripting/runtime product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-416 RED: tip/product cloned Value before as_float in:\n  {}",
        bad_paths.join("\n  ")
    );
}

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

//! WDB-445: Copy `f32` formal into `let` must not `.clone()`.
//!
//! Product `terrain/vegetation.rs`:
//!   `let mut x = start_x.clone()`
//! WJ is `let mut x = start_x`.
//! Distinct from WDB-437 (i32 **formal** into let), WDB-442 (u32 **local** into let),
//! and WDB-440 (f32 into **struct lit**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn scatter_region(start_x: f32, start_z: f32, end_x: f32, end_z: f32) -> i32 {
    let mut count: i32 = 0
    let mut z = start_z
    while z < end_z {
        let mut x = start_x
        while x < end_x {
            count = count + 1
            x = x + 1.0
        }
        z = z + 1.0
    }
    count
}
"#;

#[test]
fn wdb445_module_file_copy_f32_formal_into_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-445 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-445 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("start_x.clone()") || rs.contains("start_z.clone()");
    assert!(
        !bad,
        "WDB-445 RED: Copy f32 formal cloned into let:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb445_search_roots() -> Vec<PathBuf> {
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
fn wdb445_tip_out_game_core_vegetation_start_x_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb445_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/terrain/vegetation.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/terrain/vegetation.rs"));
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
            !t.starts_with("//") && t.contains("start_x.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-445: vegetation product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-445 RED: tip/product Copy f32 formal into let clone:\n  {}",
        bad_paths.join("\n  ")
    );
}

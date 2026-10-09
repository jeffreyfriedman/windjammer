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

//! WDB-472: Copy `i32` for-range bindings must not `.clone()` at a by-value call.
//!
//! Product `scene/station_geometry.rs` `carve_room`:
//!   `set_if(grid, x.clone(), y.clone(), z.clone(), 0_i32)`
//! WJ is `set_if(grid, x, y, z, 0)` inside `for x` / `for z` / `for y`.
//! Distinct from WDB-438 (Copy i32 formals inside a tuple literal) and
//! WDB-456 (Copy i32 locals copied into `let`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
fn paint(x: i32, y: i32, z: i32) -> i32 {
    x + y + z
}

pub fn carve(x0: i32, z0: i32, x1: i32, z1: i32, floor_y: i32, ceil_y: i32) -> i32 {
    let mut n = 0
    for x in x0 + 1..x1 {
        for z in z0 + 1..z1 {
            for y in floor_y + 1..ceil_y {
                n = n + paint(x, y, z)
            }
        }
    }
    n
}
"#;

#[test]
fn wdb472_module_file_copy_i32_range_binding_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-472 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-472 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x.clone()") || rs.contains("y.clone()") || rs.contains("z.clone()");
    assert!(
        !bad,
        "WDB-472 RED: Copy i32 range binding cloned at the call:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb472_search_roots() -> Vec<PathBuf> {
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
fn wdb472_tip_out_station_geometry_range_i32_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb472_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/scene/station_geometry.rs"));
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/scene/station_geometry.rs",
        ));
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
            !t.starts_with("//")
                && (t.contains("x.clone()") || t.contains("y.clone()") || t.contains("z.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-472: station_geometry product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-472 RED: tip/product Copy i32 range binding cloned:\n  {}",
        bad_paths.join("\n  ")
    );
}

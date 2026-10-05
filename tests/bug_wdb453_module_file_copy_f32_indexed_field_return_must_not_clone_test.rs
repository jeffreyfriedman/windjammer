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

//! WDB-453: Copy `f32` **indexed field** on return must not `.clone()`.
//!
//! Product `rendering/gpu_profiler.rs` `timing_avg_ms_at`:
//!   `return self.pass_timings[index].avg_ms.clone()`
//! WJ uses bare `self.pass_timings[index].avg_ms`.
//! Distinct from WDB-439 (i32 indexed **tuple** field return), WDB-435 (u32 field return),
//! and WDB-346 (f32 field in expr, not indexed return).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct GpuPassTiming {
    pub avg_ms: f32,
}

pub struct GpuProfiler {
    pub pass_timings: Vec<GpuPassTiming>,
}

impl GpuProfiler {
    pub fn timing_avg_ms_at(self, index: int) -> f32 {
        if index >= 0 && index < self.pass_timings.len() {
            return self.pass_timings[index].avg_ms
        }
        0.0
    }
}
"#;

#[test]
fn wdb453_module_file_copy_f32_indexed_field_return_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-453 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-453 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("avg_ms.clone()");
    assert!(
        !bad,
        "WDB-453 RED: Copy f32 indexed field cloned on return:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb453_search_roots() -> Vec<PathBuf> {
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
fn wdb453_tip_out_game_core_gpu_profiler_f32_indexed_field_return_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb453_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/gpu_profiler.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/gpu_profiler.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/rendering/gpu_profiler.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/gpu_profiler.rs"));
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
            !t.starts_with("//") && t.contains("avg_ms.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-453: gpu_profiler product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-453 RED: tip/product Copy f32 indexed field return clone:\n  {}",
        bad_paths.join("\n  ")
    );
}

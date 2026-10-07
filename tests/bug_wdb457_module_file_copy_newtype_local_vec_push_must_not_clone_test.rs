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

//! WDB-457: Copy **newtype** local into `Vec::push` must not `.clone()`.
//!
//! Product `lighting2d/light_manager.rs` add_light:
//!   `self.light_ids.push(id.clone())`
//! WJ uses bare `id` (`let id = LightId::new(self.next_id)`).
//! MultiFile isolate is also RED (not tip-lag only).
//! Distinct from WDB-431 (Copy u64 **field** into insert/push), WDB-438 (Copy i32
//! formal into **tuple** lit for push), and WDB-455 (i32 local into **field**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct LightId {
    pub value: i32,
}

impl LightId {
    pub fn new(value: i32) -> LightId {
        LightId { value: value }
    }

    pub fn value(self) -> i32 {
        self.value
    }
}

pub struct LightManager {
    pub light_ids: Vec<LightId>,
    pub next_id: i32,
}

impl LightManager {
    pub fn new() -> LightManager {
        LightManager { light_ids: vec![], next_id: 0 }
    }

    pub fn add_light(self) -> LightId {
        let id = LightId::new(self.next_id)
        self.next_id = self.next_id + 1
        self.light_ids.push(id)
        LightId::new(id.value())
    }
}
"#;

#[test]
fn wdb457_module_file_copy_newtype_local_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-457 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-457 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("id.clone()");
    assert!(
        !bad,
        "WDB-457 RED: Copy newtype local cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb457_search_roots() -> Vec<PathBuf> {
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
fn wdb457_tip_out_game_core_light_manager_copy_newtype_vec_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb457_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/lighting2d/light_manager.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/lighting2d/light_manager.rs"),
        );
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
            !t.starts_with("//") && t.contains("light_ids.push(id.clone())")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-457: light_manager product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-457 RED: tip/product Copy newtype local into Vec::push clone:\n  {}",
        bad_paths.join("\n  ")
    );
}

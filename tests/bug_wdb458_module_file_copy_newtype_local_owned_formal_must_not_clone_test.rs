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

//! WDB-458: Copy **newtype** local into owned method formal must not `.clone()`.
//!
//! Product `ecs/component_storage.rs` list_component_ids_for_entity:
//!   `if self.has(entity, comp_id.clone()) { result.push(comp_id.clone()) }`
//! WJ uses bare `comp_id` for both `has` and `push`.
//! Distinct from WDB-457 (Copy newtype into **Vec::push** with field reuse),
//! WDB-431 (Copy u64 **field** into insert/push), and WDB-393 (i32 formal field).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct ComponentId {
    pub index: u32,
}

impl ComponentId {
    pub fn new(index: u32) -> ComponentId {
        ComponentId { index: index }
    }
}

pub struct Registry {
    pub arrays_len: u32,
}

impl Registry {
    pub fn has(self, entity: i64, component_id: ComponentId) -> bool {
        let _ = entity
        component_id.index < self.arrays_len
    }

    pub fn list_component_ids_for_entity(self, entity: i64) -> Vec<ComponentId> {
        let mut result: Vec<ComponentId> = Vec::new()
        let mut i: u32 = 0
        while i < self.arrays_len {
            let comp_id = ComponentId::new(i)
            if self.has(entity, comp_id) {
                result.push(comp_id)
            }
            i = i + 1
        }
        result
    }
}
"#;

#[test]
fn wdb458_module_file_copy_newtype_local_owned_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-458 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-458 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("comp_id.clone()");
    assert!(
        !bad,
        "WDB-458 RED: Copy newtype local cloned into owned method formal:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb458_search_roots() -> Vec<PathBuf> {
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
fn wdb458_tip_out_game_core_component_storage_copy_newtype_owned_formal_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb458_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ecs/component_storage.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ecs/component_storage.rs"));
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
            !t.starts_with("//") && t.contains("has(entity, comp_id.clone())")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-458: component_storage product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-458 RED: tip/product Copy newtype local into owned method formal clone:\n  {}",
        bad_paths.join("\n  ")
    );
}

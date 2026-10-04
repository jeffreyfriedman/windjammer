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

//! WDB-435: Copy `usize` field return / struct-literal must not `.clone()`.
//!
//! Product `object_pool/object_pool.rs`:
//!   `fn capacity() { self.capacity.clone() }`
//!   `fn in_use_count() { self.in_use.clone() }`
//!   `PoolStats { capacity: self.capacity.clone(), … }`
//! WJ returns bare `self.capacity` / `self.in_use`.
//! Distinct from WDB-433 (field `.clone()` before **cast/arith**), WDB-431
//! (field into insert/push), and WDB-427 (field into `let`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct PoolStats {
    pub capacity: usize,
    pub in_use: usize,
    pub peak_usage: usize,
}

pub struct ObjectPool<T> {
    available: Vec<T>,
    capacity: usize,
    in_use: usize,
    peak_usage: usize,
}

impl ObjectPool<T> {
    pub fn capacity(self) -> usize {
        self.capacity
    }

    pub fn in_use_count(self) -> usize {
        self.in_use
    }

    pub fn stats(self) -> PoolStats {
        PoolStats {
            capacity: self.capacity,
            in_use: self.in_use,
            peak_usage: self.peak_usage,
        }
    }
}
"#;

#[test]
fn wdb435_module_file_copy_usize_field_return_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-435 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-435 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("self.capacity.clone()")
        || rs.contains("self.in_use.clone()")
        || rs.contains("self.peak_usage.clone()");
    assert!(
        !bad,
        "WDB-435 RED: Copy usize field return/struct-lit cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-435 cargo-check");
}

fn wdb435_search_roots() -> Vec<PathBuf> {
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
fn wdb435_tip_out_game_core_object_pool_copy_field_return_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb435_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/object_pool/object_pool.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/object_pool/object_pool.rs"),
        );
        paths.push(
            dir.join(
                "windjammer-game/windjammer-game-core/gen/object_pool/object_pool/object_pool.rs",
            ),
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
            !t.starts_with("//")
                && (t.contains("self.capacity.clone()")
                    || t.contains("self.in_use.clone()")
                    || t.contains("self.peak_usage.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-435: object_pool product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-435 RED: tip/product Copy usize field return clone:\n  {}",
        bad_paths.join("\n  ")
    );
}

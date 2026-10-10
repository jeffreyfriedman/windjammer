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

//! WDB-485: a `string` passed into a call and also moved in a later `match`
//! arm must not `.clone().clone()`. One clone into the call is enough.
//!
//! Product `assets/loader.rs` `load_batch`:
//!   `self.load(name.clone().clone(), path, size)`
//!   then `failures.push((name, error))` on `Err`
//! WJ is `self.load(name, path, size)` and the `Err` arm uses `name`.
//! P3.590b allows one `name.clone()`. This gate rejects the second clone.
//! Distinct from WDB-482 (exclusive arms, where no clone is required).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Asset {
    pub name: string,
}

pub struct Loader {
    pub error_count: int,
}

impl Loader {
    pub fn load(self, name: string, path: string, size: int) -> Result<Asset, string> {
        if size == 0 {
            return Err("empty")
        }
        Ok(Asset { name: name })
    }

    pub fn load_batch(self, assets: Vec<(string, string, int)>) -> int {
        let mut failures = Vec::new()
        for entry in assets {
            let (name, path, size) = entry
            match self.load(name, path, size) {
                Ok(_asset) => {}
                Err(error) => {
                    self.error_count = self.error_count + 1
                    failures.push(name)
                    let _kept = error
                }
            }
        }
        failures.len()
    }
}
"#;

#[test]
fn wdb485_module_file_match_reused_string_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-485 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-485 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "WDB-485 RED: match-reused string double-cloned into load:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb485_search_roots() -> Vec<PathBuf> {
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
fn wdb485_tip_out_loader_name_must_not_double_clone() {
    let mut paths = Vec::new();
    for dir in wdb485_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/assets/loader.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/assets/loader.rs"));
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
            !t.starts_with("//") && t.contains("name.clone().clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-485: loader product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-485 RED: tip/product match-reused name double-cloned into load:\n  {}",
        bad_paths.join("\n  ")
    );
}

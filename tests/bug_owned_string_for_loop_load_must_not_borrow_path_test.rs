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

//! P3.590b: product `load_batch` — Err arm reuses `name` → `name.clone()`, but
//! `path` must still move into owned `String` formal (not `&path`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Asset {
    pub name: string,
    pub path: string,
}

pub struct Loader {
    pub error_count: int,
}

impl Loader {
    pub fn new() -> Loader {
        Loader { error_count: 0 }
    }

    pub fn detect_format(path: string) -> int {
        if path.ends_with(".png") {
            return 1
        }
        0
    }

    pub fn load(self, name: string, path: string, size: usize) -> Result<Asset, string> {
        let _fmt = Loader::detect_format(path)
        if size == 0 {
            return Err("empty")
        }
        Ok(Asset { name: name, path: path })
    }

    pub fn load_batch(self, assets: Vec<(string, string, usize)>) -> int {
        let mut n = 0
        let mut failures: Vec<(string, string)> = Vec::new()
        for entry in assets {
            let (name, path, size) = entry
            match self.load(name, path, size) {
                Ok(_asset) => {
                    n = n + 1
                },
                Err(error) => {
                    self.error_count = self.error_count + 1
                    failures.push((name, error))
                },
            }
        }
        let _ = failures
        n
    }
}
"#;

#[test]
fn owned_string_for_loop_load_must_not_borrow_path() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    eprintln!("P3.590b MultiFile emit:\n{rs}");
    let load_sig = rs
        .lines()
        .find(|l| l.contains("pub fn load(") && l.contains("path:"))
        .unwrap_or("")
        .to_string();
    assert!(
        load_sig.contains("path: String"),
        "load formal must stay owned String:\n{load_sig}\n{rs}"
    );
    let bad: Vec<_> = rs
        .lines()
        .filter(|l| l.contains(".load(") && (l.contains("&path") || l.contains("&_temp")))
        .collect();
    assert!(
        bad.is_empty(),
        "P3.590b RED MultiFile: owned path borrowed at load() sites:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn tip_out_loader_load_batch_must_not_borrow_path() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/assets/loader.rs");
    let rs = std::fs::read_to_string(&game).unwrap_or_default();
    assert!(!rs.is_empty(), "missing {}", game.display());
    let bad: Vec<_> = rs
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains(".load(") && (l.contains("&path") || l.contains("&_temp")))
        .map(|(i, l)| format!("{}:{}", i + 1, l.trim()))
        .collect();
    assert!(
        bad.is_empty(),
        "P3.590b RED tip-out: owned path borrowed at load() sites:\n  {}",
        bad.join("\n  ")
    );
}

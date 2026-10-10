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

//! Regression: module file result ok must not to owned borrow break.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Asset {
    pub name: string,
}

pub fn load(name: string) -> Result<Asset, string> {
    Ok(Asset { name: name })
}

pub fn load_owned(name: string) -> Result<Asset, string> {
    load(name)
}
"#;

/// Shape: match self.load(...) Ok/Err + push.
const SRC_SELF_LOAD: &str = r#"
pub struct LoadedAsset {
    pub name: string,
}

pub struct AssetLoader {
    pub count: i32,
}

impl AssetLoader {
    pub fn load(self, name: string, path: string, size: usize) -> Result<LoadedAsset, string> {
        self.count = self.count + 1
        Ok(LoadedAsset { name: name })
    }

    pub fn load_batch(self, name: string, path: string, size: usize) {
        let mut successes = Vec::new()
        match self.load(name, path, size) {
            Ok(asset) => successes.push(asset),
            Err(_) => {},
        }
        let _ = successes
    }
}
"#;

#[test]
fn module_file_result_ok_must_not_to_owned_borrow_break() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".to_owned()") || rs.contains("to_owned()");
    assert!(
        !bad,
        "RED: Result Ok emitted to_owned borrow-break:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_self_load_result_must_not_to_owned_borrow_break() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_SELF_LOAD);
    let map = test.compile().expect("self-load compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("self-load lib.rs:\n{rs}");
    let bad = rs.contains(".to_owned()") || rs.contains("to_owned()");
    assert!(
        !bad,
        "RED: self.load Result match used to_owned:\n{rs}"
    );
    test.cargo_check().expect("self-load cargo-check");
}

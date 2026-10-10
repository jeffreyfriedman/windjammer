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

//! DB-348: owned `String` field init must not emit `path.clone().to_string()`.
//!
//! Tip output for a game project `assets/loader.rs`:
//!   `LoadedAsset { …, path: path.clone().to_string(), … }` with `path: String`.
//! Prefer move (`path`) or a single clone (`path.clone()`), never clone+to_string.
//! Related to DB-340 (double to_string) but distinct pattern.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct LoadedAsset {
    pub name: string,
    pub path: string,
}

pub fn load(name: string, path: string) -> LoadedAsset {
    LoadedAsset {
        name: name,
        path: path,
    }
}
"#;

#[test]
fn module_file_owned_string_must_not_clone_then_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone().to_string()");
    assert!(
        !bad,
        "owned String emitted clone().to_string():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

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

//! DB-340: owned String store must not emit `.to_string().to_string()`.
//!
//! Tip output for a game project cluster:
//!   `ai/bt_serialization.rs`: `rec.params = value.to_string().to_string()`
//!   `scripting/components.rs`, `editor/prefab_system.rs`, `editor/asset_browser.rs`,
//!   `rendering/sprite.rs` — same double coerce.
//! Prefer a single `.to_string()` (from `&str`) or a move (from `String`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Rec {
    pub params: string,
}

pub fn update_params(value: string) -> Rec {
    Rec {
        params: value,
    }
}
"#;

/// Product shape (editor/asset_browser): demoted `&str` formal reused in `.contains`
/// then assigned into owned `string` field — tip emitted `query.to_string().to_string()`.
const SRC_DEMOTED_ASSIGN: &str = r#"
pub struct AssetBrowser {
    pub search_query: string,
}

impl AssetBrowser {
    pub fn search(self, query: string) {
        self.search_query = query
        let _ = "name".contains(query)
    }

    pub fn list_assets(self, root_path: string) {
        self.search_query = root_path
        let _ = root_path.len()
    }
}
"#;

#[test]
fn module_file_owned_string_must_not_double_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".to_string().to_string()");
    assert!(
        !bad,
        "owned String store emitted double to_string:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_demoted_str_assign_must_not_double_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_DEMOTED_ASSIGN);
    let map = test.compile().expect("demoted assign compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("demoted-assign lib.rs:\n{rs}");
    let bad = rs.contains(".to_string().to_string()");
    assert!(
        !bad,
        "demoted &str → String field double to_string:\n{rs}"
    );
    assert!(
        rs.contains(".to_string()") || rs.contains(".into()") || rs.contains("search_query ="),
        "expected owned coerce into search_query. Got:\n{rs}"
    );
    test.cargo_check().expect("demoted assign cargo-check");
}

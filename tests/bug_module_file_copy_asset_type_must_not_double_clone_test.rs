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

//! Regression: module file copy asset type must not double clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum AssetType {
    Mesh,
    Texture,
}

pub fn type_name(t: AssetType) -> string {
    match t {
        AssetType::Mesh => "mesh",
        AssetType::Texture => "texture",
    }
}

pub struct AssetEntry {
    pub asset_type: AssetType,
}

pub struct Browser {
    pub assets: Vec<AssetEntry>,
}

impl Browser {
    pub fn name_at(self, i: usize) -> string {
        type_name(self.assets[i].asset_type)
    }
}
"#;

#[test]
fn module_file_copy_asset_type_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().asset_type") || rs.contains(".asset_type.clone()");
    assert!(
        !bad,
        "RED: indexed Copy AssetType double-cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

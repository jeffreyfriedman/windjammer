#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_if_let_void_block_adds_semicolon() {
    let input = r#"
use std::collections::HashMap

struct Asset {
    id: i64,
    name: string,
    status: i32,
}

struct Manager {
    assets: HashMap<i64, Asset>,
}

impl Manager {
    pub fn update_status(self, asset_id: i64) -> bool {
        if let Some(asset) = self.assets.get(asset_id) {
            let mut copy = asset.clone()
            copy.status = 1
            self.assets.insert(asset_id, copy)
        }
        true
    }
}
"#;

    let rust_code = test_utils::compile_single(input);
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("self.assets.insert(asset_id, copy);"),
        "Expected semicolon after insert in if-let block.\nGenerated:\n{}",
        rust_code
    );
}

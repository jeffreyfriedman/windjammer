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

//!
//!   `self.tiles[i] = tile_id.clone()`
//! WJ uses bare `tile_id`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct TileMap {
    pub tiles: Vec<u32>,
    pub width: u32,
    pub height: u32,
}

impl TileMap {
    pub fn clear(self, tile_id: u32) {
        let mut i: u32 = 0
        let n = self.width * self.height
        while i < n {
            self.tiles[i as int] = tile_id
            i = i + 1
        }
    }
}
"#;

#[test]
fn module_file_copy_u32_formal_index_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("tile_id.clone()");
    assert!(
        !bad,
        "RED: Copy u32 formal cloned into index assign:\n{rs}"
    );
    let _ = test.cargo_check();
}


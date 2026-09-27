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

//! WDB-406: i32 field/local compares must not emit `as usize` on one peer.
//!
//! Product leftover after WDB-395 (signed *zero* sentinels stay signed):
//!   `rendering/spatial_index.rs`:
//!     `if clen > (self.max_leaf_objects as usize) && d < (self.max_depth as usize)`
//!   while `clen`/`d`/`max_leaf_objects`/`max_depth` are all i32.
//! The same function indexes with `as usize` (`.len() as i32`, `nodes[cix as usize]`),
//! which still poisons the comparison peers.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct SpatialOctree {
    pub depths: Vec<i32>,
    pub max_leaf_objects: i32,
    pub max_depth: i32,
}

impl SpatialOctree {
    pub fn should_split(self, node_idx: i32, count: i32) -> bool {
        if (node_idx as usize) >= self.depths.len() {
            return false
        }
        let d = self.depths[node_idx as usize]
        let clen = count
        clen > self.max_leaf_objects && d < self.max_depth
    }
}
"#;

#[test]
fn wdb406_module_file_i32_field_compare_must_not_emit_usize() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-406 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-406 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("max_leaf_objects as usize")
            && !rs.contains("max_depth as usize"),
        "WDB-406 RED: i32 field compare emitted as usize:\n{rs}"
    );
    test.cargo_check()
        .expect("WDB-406 i32 field compare must cargo-check");
}

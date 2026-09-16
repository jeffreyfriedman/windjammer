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
))]

//! P3.316: `while i < self.component_size` (usize field) must not emit `i += 1 as i32` / bare `0` + `as usize`.
//!
//! Product: `windjammer-game-core` `ecs/component_storage.wj` — inferred i32 locals vs usize bounds.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod storage
"#;

const STORAGE: &str = r#"
pub struct Pool {
    pub component_size: usize,
    pub dense: Vec<u8>,
}

pub fn copy_bytes(self, data: Vec<u8>) {
    let mut i = 0
    while i < self.component_size {
        self.dense.push(data[i])
        i = i + 1
    }
}
"#;

fn bad_usize_loop_increment(rs: &str) -> bool {
    rs.contains("+= 1 as i32")
        || rs.contains("+ 1 as i32")
        || (rs.contains("let mut i = 0;") && rs.contains("+= 1 as usize"))
}

#[test]
fn usize_field_loop_counter_increment_width() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("storage.wj", STORAGE);
    let map = test
        .compile()
        .expect("P3.316 multipass compile should succeed");
    let rs = map.get("storage.rs").expect("storage.rs");
    if bad_usize_loop_increment(rs) {
        eprintln!("P3.316 RED emit:\n{rs}");
    }
    assert!(
        !bad_usize_loop_increment(rs),
        "usize field loop counter must use usize binding + plain increment, not i32 casts"
    );
    test.cargo_check().expect("P3.316 storage.rs must cargo-check");
}

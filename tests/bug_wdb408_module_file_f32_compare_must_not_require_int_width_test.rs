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

//! WDB-408: f32 slab compares must not fail numeric inference as Isize vs I64.
//!
//! Product: `physics/collision.wj` `raycast` —
//!   `if tmax >= tmin && tmax >= 0.0`
//! Engine library transpile (`wj game build`) dies with
//! `Type mismatch comparison operands must share integer width: Isize vs I64`.
//! Float compares are not integer-width constraints.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct AABB {
    pub min_x: f32,
    pub max_x: f32,
}

impl AABB {
    pub fn raycast(self, origin_x: f32, dir_x: f32) -> (bool, f32) {
        let mut tmin = 0.0
        let mut tmax = 1000000.0
        if dir_x.abs() > 0.0001 {
            let tx1 = (self.min_x - origin_x) / dir_x
            let tx2 = (self.max_x - origin_x) / dir_x
            tmin = tmin.max(tx1.min(tx2))
            tmax = tmax.min(tx1.max(tx2))
        }
        if tmax >= tmin && tmax >= 0.0 {
            (true, tmin)
        } else {
            (false, 0.0)
        }
    }
}
"#;

#[test]
fn wdb408_module_file_f32_compare_must_not_require_int_width() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test
        .compile()
        .expect("WDB-408: f32 tmax >= tmin must not fail numeric inference");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-408 MultiFile lib.rs:\n{rs}");
    assert!(
        rs.contains("tmax >= tmin") || rs.contains("tmax>="),
        "expected f32 compare to emit:\n{rs}"
    );
    test.cargo_check()
        .expect("WDB-408 f32 compare must cargo-check");
}

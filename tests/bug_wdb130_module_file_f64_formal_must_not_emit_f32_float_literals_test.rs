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

//! WDB-130: float literal `0.0` / `0.85` into an `f64` formal or field is emitted as
//! `0.0_f32` / `0.85_f32` → E0308 (`expected f64, found f32`).
//!
//! WindjammerDB CQ-C5 (~8×):
//!   `heap.push_or_decrease(0.0_f32, …)` while key is `f64`
//!   `graph_vertex_f64_set(..., 0.0_f32)`
//!   `session.pagerank(0.85_f32, 20)`
//!   struct field `distance: 0.1_f32` while field is `f64`
//!
//! Expected: `0.0_f64` / `0.85` inferred as f64 from formal/field type.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod math
pub mod use_math
"#;

const MATH: &str = r#"
pub fn set_f64(seed: f64, value: f64) -> f64 {
    seed + value
}

pub struct Hit {
    pub distance: f64,
}
"#;

const USE_MATH: &str = r#"
use crate::math::set_f64
use crate::math::Hit

pub fn seed_zero() -> f64 {
    set_f64(0.0, 0.0)
}

pub fn hit_near() -> Hit {
    Hit { distance: 0.1 }
}

pub fn cap() -> f64 {
    seed_zero() + hit_near().distance
}
"#;

fn wdb130_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("math.wj", MATH);
    test.add_file("use_math.wj", USE_MATH);
    test
}

#[test]
fn wdb130_module_file_f64_formal_must_not_emit_f32_float_literals() {
    let test = wdb130_fixture();
    let map = test
        .compile()
        .expect("WDB-130 multipass compile should succeed (codegen may still be wrong)");
    let use_rs = map.get("use_math.rs").expect("use_math.rs");

    let bad_f32 = use_rs.contains("0.0_f32")
        || use_rs.contains("0.1_f32")
        || use_rs.contains("_f32");
    let good_f64 = use_rs.contains("0.0_f64")
        || use_rs.contains("0.1_f64")
        || (use_rs.contains("0.0") && !use_rs.contains("_f32"));

    eprintln!("WDB-130 use_math.rs:\n{use_rs}");
    eprintln!("bad_f32={bad_f32} good_f64={good_f64}");

    if bad_f32 {
        panic!(
            "WDB-130 RED: f64 formals/fields must not emit _f32 literals. \
             Product: graph_sssp / graph_batch / pagerank / VectorTopKHit (~8 wdb-layers E0308)."
        );
    }

    test.cargo_check().expect(
        "WDB-130 RED: f64-typed float literals must cargo-check as f64 (not f32).",
    );

    assert!(
        good_f64 || !bad_f32,
        "WDB-130: expected f64-typed literals in emit"
    );
}

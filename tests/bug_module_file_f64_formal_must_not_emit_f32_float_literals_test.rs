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

//! Regression: module file f64 formal must not emit f32 float literals.
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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("math.wj", MATH);
    test.add_file("use_math.wj", USE_MATH);
    test
}

#[test]
fn module_file_f64_formal_must_not_emit_f32_float_literals() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let use_rs = map.get("use_math.rs").expect("use_math.rs");

    let bad_f32 = use_rs.contains("0.0_f32")
        || use_rs.contains("0.1_f32")
        || use_rs.contains("_f32");
    let good_f64 = use_rs.contains("0.0_f64")
        || use_rs.contains("0.1_f64")
        || (use_rs.contains("0.0") && !use_rs.contains("_f32"));

    eprintln!("use_math.rs:\n{use_rs}");
    eprintln!("bad_f32={bad_f32} good_f64={good_f64}");

    if bad_f32 {
        panic!(
            "RED: f64 formals/fields must not emit _f32 literals. \
            "
        );
    }

    test.cargo_check().expect(
        "RED: f64-typed float literals must cargo-check as f64 (not f32).",
    );

    assert!(
        good_f64 || !bad_f32,
        "expected f64-typed literals in emit"
    );
}

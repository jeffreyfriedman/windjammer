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

//!
//!   `wave1_opt_hardware_build_session(…, &empty_bakeoff_run(), …)`
//! while formals are owned `Wave1OptBakeoffRun` → E0308 expected T, found &T.
//!
//! **owned** and the arg is already an owned temporary — the compiler must not invent `&`.
//! Signature-driven (no hardcoded fn names).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod bakeoff
pub mod hardware
"#;

const BAKEOFF: &str = r#"
pub struct BakeoffRun {
    pub nanos: int,
    pub contended: bool,
}

pub fn empty_bakeoff_run() -> BakeoffRun {
    BakeoffRun { nanos: 0, contended: false }
}
"#;

const HARDWARE: &str = r#"
use crate::bakeoff::BakeoffRun
use crate::bakeoff::empty_bakeoff_run

pub fn build_session(
    point: BakeoffRun,
    secondary: BakeoffRun,
    update: BakeoffRun,
    q6: BakeoffRun,
) -> bool {
    point.nanos >= 0 && !secondary.contended && update.nanos >= 0 && q6.nanos >= 0
}

pub fn cap_empty_session() -> bool {
    build_session(
        empty_bakeoff_run(),
        empty_bakeoff_run(),
        empty_bakeoff_run(),
        empty_bakeoff_run(),
    )
}

pub fn cap_with_run(run: BakeoffRun) -> bool {
    build_session(run, empty_bakeoff_run(), empty_bakeoff_run(), empty_bakeoff_run())
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("bakeoff.wj", BAKEOFF);
    test.add_file("hardware.wj", HARDWARE);
    test
}

#[test]
fn module_file_owned_helper_into_owned_formal_must_not_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let bake_rs = map.get("bakeoff.rs").expect("bakeoff.rs");
    let hw_rs = map.get("hardware.rs").expect("hardware.rs");

    eprintln!("bakeoff.rs:\n{bake_rs}\nhardware.rs:\n{hw_rs}");

    let owned_formals = hw_rs.contains("point: BakeoffRun")
        || hw_rs.contains("point:BakeoffRun");
    let bad_borrow = hw_rs.contains("build_session(&empty_bakeoff_run()")
        || hw_rs.contains("&empty_bakeoff_run()");

    assert!(
        owned_formals,
        "expected owned BakeoffRun formals. Got:\n{hw_rs}"
    );
    assert!(
        !bad_borrow,
        "RED: owned BakeoffRun formal received &empty_bakeoff_run(). \
         Got:\n{hw_rs}"
    );
}


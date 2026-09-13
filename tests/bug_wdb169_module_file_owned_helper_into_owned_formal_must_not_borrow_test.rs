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

//! WDB-169: owned helper return into owned Custom formal must not emit `&`.
//!
//! Product after tip-cluster of `wave1_opt_hardware_port` alone (~82×):
//!   `wave1_opt_hardware_build_session(…, &empty_bakeoff_run(), …)`
//! while formals are owned `Wave1OptBakeoffRun` → E0308 expected T, found &T.
//!
//! Opposite of WDB-167 (owned into demoted `&T` needs borrow). Here the formal is
//! **owned** and the arg is already an owned temporary — tip must not invent `&`.
//! Signature-driven (no hardcoded fn names).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

pub fn build_session(point: BakeoffRun, secondary: BakeoffRun) -> bool {
    point.nanos >= 0 && !secondary.contended
}

pub fn cap_empty_session() -> bool {
    // Product: wave1_opt_hardware_build_session(…, empty_bakeoff_run(), …)
    build_session(empty_bakeoff_run(), empty_bakeoff_run())
}
"#;

fn wdb169_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("bakeoff.wj", BAKEOFF);
    test.add_file("hardware.wj", HARDWARE);
    test
}

#[test]
fn wdb169_module_file_owned_helper_into_owned_formal_must_not_borrow() {
    let test = wdb169_fixture();
    let map = test
        .compile()
        .expect("WDB-169 multipass compile should succeed");
    let bake_rs = map.get("bakeoff.rs").expect("bakeoff.rs");
    let hw_rs = map.get("hardware.rs").expect("hardware.rs");

    eprintln!("WDB-169 bakeoff.rs:\n{bake_rs}\nhardware.rs:\n{hw_rs}");

    let owned_formals = hw_rs.contains("point: BakeoffRun")
        || hw_rs.contains("point:BakeoffRun");
    let bad_borrow = hw_rs.contains("build_session(&empty_bakeoff_run()")
        || hw_rs.contains("&empty_bakeoff_run()");

    assert!(
        owned_formals,
        "WDB-169: expected owned BakeoffRun formals. Got:\n{hw_rs}"
    );
    assert!(
        !bad_borrow,
        "WDB-169 RED: owned BakeoffRun formal received &empty_bakeoff_run(). \
         Product: wave1_opt_hardware_build_session(…, &empty_bakeoff_run(), …). Got:\n{hw_rs}"
    );
}

#[test]
fn wdb169_product_wave1_opt_must_not_borrow_empty_bakeoff_into_owned() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational/wave1_opt_hardware_port.rs");
    if !path.exists() {
        eprintln!("WDB-169: skip product gate — {} missing", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path).expect("read opt");
    let owned_build = text.contains("point: Wave1OptBakeoffRun");
    let bad = owned_build && text.contains("&empty_bakeoff_run()");
    eprintln!(
        "WDB-169 product owned_point_formal={} empty_borrow={}",
        owned_build,
        text.contains("&empty_bakeoff_run()")
    );
    assert!(
        !bad,
        "WDB-169 RED: product wave1_opt still passes &empty_bakeoff_run() into owned BakeoffRun formals."
    );
}

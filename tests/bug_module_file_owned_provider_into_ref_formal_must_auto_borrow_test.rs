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

//! owned Provider / tuple field into demoted `&Provider` formal must auto-borrow.
//!
//! covers bare owned local `req`; this gate covers **tuple-field** owned args
//! (`triple.0`) matching semantic DF Cap call sites. Signature-driven — no hardcoded names.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod ffi
pub mod cap
"#;

const FFI: &str = r#"
pub struct DfProvider {
    pub name: string,
    pub ok: bool,
}

/// Non-Copy (`string` field) so demotion matches RelationalDfProvider.
pub fn sql_via_provider(provider: DfProvider, sql: string) -> bool {
    provider.ok && provider.name.len() > 0 && sql.len() > 0
}
"#;

const CAP: &str = r#"
use crate::ffi::DfProvider
use crate::ffi::sql_via_provider

pub fn make_provider() -> (DfProvider, bool) {
    (DfProvider { name: "cap", ok: true }, true)
}

pub fn cap_run() -> bool {
    let triple = make_provider()
    sql_via_provider(triple.0, "SELECT 1")
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ffi.wj", FFI);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn module_file_owned_provider_tuple_into_ref_formal_must_auto_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let ffi_rs = map.get("ffi.rs").expect("ffi.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    eprintln!("ffi.rs:\n{ffi_rs}\ncap.rs:\n{cap_rs}");

    let demoted = ffi_rs.contains("provider: &DfProvider")
        || ffi_rs.contains("provider:&DfProvider");
    let owned_pass = cap_rs.contains("sql_via_provider(triple.0")
        && !cap_rs.contains("sql_via_provider(&triple.0");
    let borrows = cap_rs.contains("sql_via_provider(&triple.0");

    if demoted && owned_pass && !borrows {
        panic!(
            "RED demoted &DfProvider formal receives owned triple.0. \
Got:\n{cap_rs}\n{ffi_rs}"
        );
    }
    // GREEN if formal stays owned, or demoted + auto-borrow.
    assert!(
        !demoted || borrows || !owned_pass,
        "expected owned formal or &triple.0 borrow. Got:\n{cap_rs}\n{ffi_rs}"
    );
}

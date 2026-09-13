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

//! WDB-167: owned Provider / tuple field into demoted `&Provider` formal must auto-borrow.
//!
//! Product census (~65×): tip demotes
//!   `relational_df_sql_via_table_provider(…, provider: &RelationalDfProvider, …)`
//! while Caps pass owned `triple.0` / `provider` → E0308 expected `&RelationalDfProvider`,
//! found `RelationalDfProvider`.
//!
//! WDB-149 covers bare owned local `req`; this gate covers **tuple-field** owned args
//! (`triple.0`) matching semantic DF Cap call sites. Signature-driven — no hardcoded names.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod ffi
pub mod cap
"#;

const FFI: &str = r#"
pub struct DfProvider {
    pub ok: bool,
}

/// Read-only provider consumer — tip demotes to `&DfProvider` (product df_ffi).
pub fn sql_via_provider(provider: DfProvider, sql: string) -> bool {
    provider.ok && sql.len() > 0
}
"#;

const CAP: &str = r#"
use crate::ffi::DfProvider
use crate::ffi::sql_via_provider

pub fn make_provider() -> (DfProvider, bool) {
    (DfProvider { ok: true }, true)
}

pub fn cap_run() -> bool {
    let triple = make_provider()
    // Product Caps: relational_df_sql_via_table_provider(…, triple.0, …)
    sql_via_provider(triple.0, "SELECT 1")
}
"#;

fn wdb167_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ffi.wj", FFI);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb167_module_file_owned_provider_tuple_into_ref_formal_must_auto_borrow() {
    let test = wdb167_fixture();
    let map = test
        .compile()
        .expect("WDB-167 multipass compile should succeed");
    let ffi_rs = map.get("ffi.rs").expect("ffi.rs");
    let cap_rs = map.get("cap.rs").expect("cap.rs");

    eprintln!("WDB-167 ffi.rs:\n{ffi_rs}\ncap.rs:\n{cap_rs}");

    let demoted = ffi_rs.contains("provider: &DfProvider")
        || ffi_rs.contains("provider:&DfProvider");
    let owned_pass = cap_rs.contains("sql_via_provider(triple.0")
        && !cap_rs.contains("sql_via_provider(&triple.0");
    let borrows = cap_rs.contains("sql_via_provider(&triple.0");

    if demoted && owned_pass && !borrows {
        panic!(
            "WDB-167 RED: demoted &DfProvider formal receives owned triple.0. \
             Product: semantic Caps pass triple.0 into provider: &RelationalDfProvider. Got:\n{cap_rs}\n{ffi_rs}"
        );
    }
    // GREEN if formal stays owned, or demoted + auto-borrow.
    assert!(
        !demoted || borrows || !owned_pass,
        "WDB-167: expected owned formal or &triple.0 borrow. Got:\n{cap_rs}\n{ffi_rs}"
    );
}

#[test]
fn wdb167_product_df_ffi_demoted_provider_must_auto_borrow_at_caps() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let ffi = root.join("relational_module_file/relational_df_ffi_port.rs");
    if !ffi.exists() {
        eprintln!("WDB-167: skip product gate — {} missing", ffi.display());
        return;
    }
    let ffi_text = std::fs::read_to_string(&ffi).expect("ffi");
    let demoted = ffi_text.contains("provider: &RelationalDfProvider");
    if !demoted {
        eprintln!("WDB-167: product ffi provider not demoted — skip Cap scan");
        return;
    }
    // Caps that call with bare owned provider / triple.0 (no leading &).
    let mut bad = Vec::new();
    for entry in std::fs::read_dir(root.join("semantic")).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.contains("df_ffi_table_provider") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        for line in text.lines() {
            if !line.contains("relational_df_sql_via_table_provider(") {
                continue;
            }
            // Owned provider arg patterns seen in census: `, triple.0,` or `, provider,`
            // without a preceding `&`.
            if line.contains(", triple.0,") && !line.contains(", &triple.0,") {
                bad.push(format!("{}: {}", name, line.trim()));
            }
            if line.contains(", provider,") && !line.contains(", &provider,") {
                bad.push(format!("{}: {}", name, line.trim()));
            }
        }
    }
    eprintln!("WDB-167 product demoted_provider Cap bad={}", bad.len());
    assert!(
        bad.is_empty(),
        "WDB-167 RED: demoted &RelationalDfProvider still receives owned Cap args:\n{}",
        bad.join("\n")
    );
}

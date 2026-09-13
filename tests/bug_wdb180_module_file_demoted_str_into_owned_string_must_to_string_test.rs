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

//! WDB-180: demoted `&str` formals into owned `String` callee must `.to_string()`.
//!
//! Product residual (~8× String←&str), tip-out bakeoff:
//!   `bakeoff_run_to_fill(hardware: &str, …)` calls
//!   `opt_operator_fill_record(matrix_row: String, hardware: String, …)`
//!   with bare `matrix_row, hardware, scale, dated_label` → E0308.
//!
//! Distinct from WDB-170 (`.clone()` on demoted &str). Here args are already `&str`
//! and need ownership into String formals. Signature-driven.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod fill
pub mod bakeoff
"#;

const FILL: &str = r#"
pub struct FillRow {
    pub hardware: string,
    pub scale: string,
    pub label: string,
    pub row: string,
}

/// Owns all string fields (product opt_operator_fill_record).
pub fn fill_record(matrix_row: string, hardware: string, scale: string, dated_label: string) -> FillRow {
    FillRow { hardware: hardware, scale: scale, label: dated_label, row: matrix_row }
}
"#;

const BAKEOFF: &str = r#"
use crate::fill::FillRow
use crate::fill::fill_record

/// Multi-use read-only probes demote toward `&str` (product bakeoff_run_to_fill).
pub fn label_len(s: string) -> int {
    s.len()
}

pub fn bakeoff_run_to_fill(hardware: string, scale: string, dated_label: string, matrix_row: string) -> FillRow {
    let _h = label_len(hardware)
    let _s = label_len(scale)
    let _d = label_len(dated_label)
    let _m = label_len(matrix_row)
    // Product: fill_record(matrix_row, hardware, …) while demoted — must to_string.
    fill_record(matrix_row, hardware, scale, dated_label)
}
"#;

fn wdb180_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("fill.wj", FILL);
    test.add_file("bakeoff.wj", BAKEOFF);
    test
}

#[test]
fn wdb180_module_file_demoted_str_into_owned_string_must_to_string() {
    let test = wdb180_fixture();
    let map = test
        .compile()
        .expect("WDB-180 multipass compile should succeed");
    let fill_rs = map.get("fill.rs").expect("fill.rs");
    let bake_rs = map.get("bakeoff.rs").expect("bakeoff.rs");

    eprintln!("WDB-180 fill.rs:\n{fill_rs}\nbakeoff.rs:\n{bake_rs}");

    let callee_owned = {
        let i = fill_rs.find("fn fill_record").unwrap_or(0);
        let sl = &fill_rs[i..fill_rs.len().min(i + 200)];
        (sl.contains("hardware: String") || sl.contains("hardware:String"))
            && !(sl.contains("hardware: &str") || sl.contains("hardware:&str"))
    };
    let caller_demoted = {
        let i = bake_rs.find("fn bakeoff_run_to_fill").unwrap_or(0);
        let sl = &bake_rs[i..bake_rs.len().min(i + 200)];
        sl.contains("hardware: &str") || sl.contains("hardware:&str")
    };
    let call_ok = bake_rs.contains("fill_record(matrix_row.to_string()")
        || bake_rs.contains("fill_record(matrix_row.to_owned()")
        || bake_rs.contains("hardware.to_string()")
        || bake_rs.contains("String::from(hardware)");
    let call_bad = bake_rs.contains("fill_record(matrix_row, hardware, scale, dated_label)")
        && !bake_rs.contains(".to_string()");

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "WDB-180 RED: demoted &str into owned String fill_record without to_string. \
             Product: opt_operator_fill_record(matrix_row, hardware, …). Got:\n{bake_rs}\n{fill_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok || !call_bad,
            "WDB-180: demoted &str into owned String must to_string. Got:\n{bake_rs}"
        );
    }
}

#[test]
fn wdb180_tip_out_bakeoff_must_to_string_demoted_str_into_owned_fill() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let bakeoff = if tip.join("wave1_opt_bakeoff_port.rs").exists() {
        tip.join("wave1_opt_bakeoff_port.rs")
    } else {
        gen.join("relational/wave1_opt_bakeoff_port.rs")
    };
    let fill = gen.join("stats/opt_operator_fill_port.rs");
    if !bakeoff.exists() || !fill.exists() {
        eprintln!("WDB-180: skip tip-out — bakeoff/fill missing");
        return;
    }
    let bake_text = std::fs::read_to_string(&bakeoff).expect("bakeoff");
    let fill_text = std::fs::read_to_string(&fill).expect("fill");
    let fill_owned = fill_text.contains("hardware: String")
        && fill_text.contains("fn opt_operator_fill_record");
    let caller_demoted = bake_text.contains("fn bakeoff_run_to_fill(hardware: &str");
    let bare = bake_text
        .contains("opt_operator_fill_record(matrix_row, hardware, scale, dated_label,")
        && !bake_text.contains("matrix_row.to_string()")
        && !bake_text.contains("hardware.to_string()");
    eprintln!(
        "WDB-180 tip-out fill_owned={} demoted={} bare={} path={}",
        fill_owned,
        caller_demoted,
        bare,
        bakeoff.display()
    );
    assert!(
        !(fill_owned && caller_demoted && bare),
        "WDB-180 RED: tip-out bakeoff passes &str into owned fill_record. {}",
        bakeoff.display()
    );
}

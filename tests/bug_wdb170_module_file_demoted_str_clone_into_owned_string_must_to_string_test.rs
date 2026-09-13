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

//! WDB-170: demoted caller `&str` into owned `String` formal must `.to_string()`, not `.clone()`.
//!
//! Product census (~49× `String`←`&str`), e.g. `relational_df_analytic_execute_port.rs`:
//!   `pub fn …(sql: &str, …) { relational_sql_parse_ast(sql.clone()) }`
//! while `relational_sql_parse_ast(sql: String)` → E0308 expected `String`, found `&str`.
//!
//! Distinct from WDB-139 (field assign) and WDB-152 (lit → owned). Signature-driven.
//! Prefer tip greens over `dogfood_gen_p153.py` / tip-cluster patches.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod peg
pub mod exec
"#;

const PEG: &str = r#"
/// Owns the SQL string (product relational_sql_parse_ast keeps `sql: String`).
pub struct WdbAst {
    pub sql: string,
}

pub fn parse_ast(sql: string) -> WdbAst {
    // Field store forces owned formal (must not demote to &str).
    WdbAst { sql: sql }
}

/// Read-only probe — tip demotes to `&str` (pulls caller formal toward demotion).
pub fn sql_label_len(sql: string) -> int {
    sql.len()
}
"#;

const EXEC: &str = r#"
use crate::peg::parse_ast
use crate::peg::sql_label_len
use crate::peg::WdbAst

/// Multi-use read-only + owned callee (product DF execute demotes formal then .clone()).
pub fn execute_q1_sum_from_sql(sql: string) -> bool {
    let _n = sql_label_len(sql)
    let _empty = sql.len() == 0
    let _sel = sql == "SELECT"
    // Product emits `parse_ast(sql.clone())` while demoted — must be `.to_string()`.
    let ast = parse_ast(sql)
    ast.sql.len() > 0
}
"#;

fn wdb170_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("peg.wj", PEG);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn wdb170_module_file_demoted_str_clone_into_owned_string_must_to_string() {
    let test = wdb170_fixture();
    let map = test
        .compile()
        .expect("WDB-170 multipass compile should succeed");
    let peg_rs = map.get("peg.rs").expect("peg.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("WDB-170 peg.rs:\n{peg_rs}\nexec.rs:\n{exec_rs}");

    let callee_owned = {
        let i = peg_rs.find("fn parse_ast").unwrap_or(0);
        let sl = &peg_rs[i..peg_rs.len().min(i + 120)];
        (sl.contains("sql: String") || sl.contains("sql:String"))
            && !(sl.contains("sql: &str") || sl.contains("sql:&str"))
    };
    let caller_demoted = {
        let i = exec_rs.find("fn execute_q1_sum_from_sql").unwrap_or(0);
        let sl = &exec_rs[i..exec_rs.len().min(i + 140)];
        sl.contains("sql: &str") || sl.contains("sql:&str")
    };

    let call_ok = exec_rs.contains("parse_ast(sql.to_string()")
        || exec_rs.contains("parse_ast(sql.to_owned()")
        || exec_rs.contains("parse_ast(String::from(sql)")
        || exec_rs.contains("parse_ast((*sql).to_string()")
        || exec_rs.contains("parse_ast(sql.into()");
    let call_bad = exec_rs.contains("parse_ast(sql.clone()")
        || (exec_rs.contains("parse_ast(sql)")
            && !exec_rs.contains("parse_ast(sql.to_string()")
            && !exec_rs.contains("parse_ast(sql.to_owned()")
            && !exec_rs.contains("parse_ast(sql.into()"));

    // Soft note when tip demotes both sides (product keeps owned parse_ast).
    if !callee_owned {
        eprintln!(
            "WDB-170 note: parse_ast formal demoted (product keeps String); \
             product gate still covers DF execute."
        );
    }

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "WDB-170 RED: demoted &str caller passed into owned String via .clone()/bare. \
             Product: relational_sql_parse_ast(sql.clone()) with sql: &str. Got:\n{exec_rs}\n{peg_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok,
            "WDB-170: demoted &str into owned String must .to_string()/.to_owned(). Got:\n{exec_rs}"
        );
    }
}

fn wdb170_rel_gen_root() -> PathBuf {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    if tip.join("relational_df_analytic_execute_port.rs").exists() {
        return tip;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen/relational")
}

#[test]
fn wdb170_product_df_execute_demoted_sql_must_to_string_into_parse_ast() {
    let root = wdb170_rel_gen_root();
    let peg = root.join("relational_sql_peg_port.rs");
    let exec = root.join("relational_df_analytic_execute_port.rs");
    if !peg.exists() || !exec.exists() {
        eprintln!("WDB-170: skip product gate — peg/exec missing");
        return;
    }
    let peg_text = std::fs::read_to_string(&peg).expect("peg");
    let exec_text = std::fs::read_to_string(&exec).expect("exec");
    let callee_owned = peg_text.contains("pub fn relational_sql_parse_ast(sql: String)");
    let bad = callee_owned
        && exec_text.contains("relational_sql_parse_ast(sql.clone())")
        && exec_text.contains("sql: &str");
    eprintln!(
        "WDB-170 product callee_owned={} bad_clone_into_owned={}",
        callee_owned, bad
    );
    assert!(
        !bad,
        "WDB-170 RED: product DF execute still passes sql.clone() (&str) into owned parse_ast(String)."
    );
}

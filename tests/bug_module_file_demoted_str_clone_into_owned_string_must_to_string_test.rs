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

//! Regression: module file demoted str clone into owned string must to string.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod peg
pub mod exec
"#;

const PEG: &str = r#"
/// Owns the SQL string.
pub struct SqlAst {
    pub sql: string,
}

pub fn parse_ast(sql: string) -> SqlAst {
    // Field store forces owned formal (must not demote to &str).
    SqlAst { sql: sql }
}

/// Read-only probe — codegen demotes to `&str` (pulls caller formal toward demotion).
pub fn sql_label_len(sql: string) -> int {
    sql.len()
}
"#;

const EXEC: &str = r#"
use crate::peg::parse_ast
use crate::peg::sql_label_len
use crate::peg::SqlAst

/// Multi-use read-only + owned callee).
pub fn execute_q1_sum_from_sql(sql: string) -> bool {
    let _n = sql_label_len(sql)
    let _empty = sql.len() == 0
    let _sel = sql == "SELECT"
    // Codegen emits `parse_ast(sql.clone())` while demoted — must be `.to_string()`.
    let ast = parse_ast(sql)
    ast.sql.len() > 0
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("peg.wj", PEG);
    test.add_file("exec.wj", EXEC);
    test
}

#[test]
fn module_file_demoted_str_clone_into_owned_string_must_to_string() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let peg_rs = map.get("peg.rs").expect("peg.rs");
    let exec_rs = map.get("exec.rs").expect("exec.rs");

    eprintln!("peg.rs:\n{peg_rs}\nexec.rs:\n{exec_rs}");

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

    // Soft note when codegen demotes both sides.
    if !callee_owned {
        eprintln!(
            "note: parse_ast formal demoted (keeps String); \
             covers DF execute."
        );
    }

    if callee_owned && caller_demoted && call_bad && !call_ok {
        panic!(
            "RED: demoted &str caller passed into owned String via .clone()/bare. \
             Got:\n{exec_rs}\n{peg_rs}"
        );
    }

    if callee_owned && caller_demoted {
        assert!(
            call_ok,
            "demoted &str into owned String must .to_string()/.to_owned(). Got:\n{exec_rs}"
        );
    }
}

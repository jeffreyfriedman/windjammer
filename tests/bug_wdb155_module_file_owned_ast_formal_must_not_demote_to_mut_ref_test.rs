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

//! WDB-155: tip `--module-file` demotes owned AST formals to `&mut T` when a
//! forwarder passes the AST into a sibling binder (product DF SQL lower path).
//!
//! Product (2026-09-11 tip recheck after fixture false-GREEN):
//! - `.wj`: `relational_sql_ast_to_datafusion_sql(ast: WdbAst)` + `bind_ast(ast, …)`
//! - tip emit: both formals become `ast: &mut WdbAst`; callers keep `let ast`
//!   → E0596 on `relational_df_analytic_execute_port` (+ cascading E0308).
//!
//! Narrow Option/match emit-only fixture false-GREENs. Gate needs binder sibling.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod ast_parse
pub mod ast_bind
pub mod ast_emit
pub mod ast_exec
"#;

const AST_PARSE: &str = r#"
pub struct SqlAst {
    pub kind: i64
    pub table: string
    pub projection: string
}

pub fn parse_ast(sql: string) -> Option<SqlAst> {
    if sql.len() == 0 {
        return None
    }
    Some(SqlAst { kind: 1, table: "t", projection: "c" })
}
"#;

const AST_BIND: &str = r#"
use crate::ast_parse::SqlAst

pub struct SqlResolved {
    pub table: string
}

pub fn bind_ast(ast: SqlAst) -> Option<SqlResolved> {
    Some(SqlResolved { table: ast.table })
}
"#;

const AST_EMIT: &str = r#"
use crate::ast_parse::SqlAst
use crate::ast_bind::bind_ast
use crate::ast_bind::SqlResolved

pub struct SqlEmit {
    pub sql: string
    pub table: string
}

pub fn emit_sql(ast: SqlAst) -> Option<SqlEmit> {
    match bind_ast(ast) {
        Some(resolved) => Some(SqlEmit { sql: "select 1", table: resolved.table }),
        None => None,
    }
}
"#;

const AST_EXEC: &str = r#"
use crate::ast_parse::parse_ast
use crate::ast_emit::emit_sql

pub fn parse_then_emit(sql: string) -> bool {
    let ast = match parse_ast(sql) {
        Some(a) => a,
        None => {
            return false
        }
    }
    let emit = match emit_sql(ast) {
        Some(e) => e,
        None => {
            return false
        }
    }
    emit.sql.len() > 0
}
"#;

fn wdb155_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ast_parse.wj", AST_PARSE);
    test.add_file("ast_bind.wj", AST_BIND);
    test.add_file("ast_emit.wj", AST_EMIT);
    test.add_file("ast_exec.wj", AST_EXEC);
    test
}

#[test]
fn wdb155_module_file_option_match_ast_pipeline_must_keep_owned_formal() {
    let mut test = wdb155_fixture();
    let map = test
        .compile()
        .expect("WDB-155 multipass compile should succeed (codegen may still be wrong)");
    let bind_rs = map.get("ast_bind.rs").expect("ast_bind.rs must be generated");
    let emit_rs = map.get("ast_emit.rs").expect("ast_emit.rs must be generated");
    let exec_rs = map.get("ast_exec.rs").expect("ast_exec.rs must be generated");

    let demoted_bind = bind_rs.contains("ast: &mut SqlAst") || bind_rs.contains("ast:&mut SqlAst");
    let demoted_emit = emit_rs.contains("ast: &mut SqlAst") || emit_rs.contains("ast:&mut SqlAst");
    let mut_borrow_call = exec_rs.contains("emit_sql(&mut ast)") || exec_rs.contains("&mut ast");

    if demoted_bind || demoted_emit || mut_borrow_call {
        eprintln!("WDB-155 RED emit ast_bind.rs:\n{bind_rs}");
        eprintln!("WDB-155 RED emit ast_emit.rs:\n{emit_rs}");
        eprintln!("WDB-155 RED emit ast_exec.rs:\n{exec_rs}");
    }

    assert!(
        !demoted_bind,
        "WDB-155 RED: bind_ast(ast: SqlAst) must stay owned (not &mut). Product tip demotes relational_sql_bind_ast."
    );
    assert!(
        !demoted_emit,
        "WDB-155 RED: emit_sql forwarder that calls bind_ast must keep owned formal (not &mut SqlAst). Product tip demotes relational_sql_ast_to_datafusion_sql."
    );
    assert!(
        !mut_borrow_call,
        "WDB-155 RED: parse_then_emit must call emit_sql(ast) not emit_sql(&mut ast) after `let ast = match parse…`."
    );
}

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
    feature = "codegen_tests",
))]

//!
//! Same-crate free-fn / MultiFile isolate: owned formals must stay owned at the call site.
//! `&emit.table` is correct there; the gate skips when formals are demoted.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Emit {
    pub table: string,
    pub sql: string,
}

pub struct Host {
    pub ok: bool,
}

pub fn sql_exec(host: Host, left_table: string, right_table: string, sql: string) -> Emit {
    Emit {
        table: left_table + "_" + right_table,
        sql: sql,
    }
}

pub fn run(host: Host, emit: Emit, right_name: string) -> Emit {
    let _ = host.ok
    sql_exec(host, emit.table, right_name, emit.sql)
}
"#;

#[test]
fn module_file_owned_string_sql_exec_must_not_receive_refs() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned_left = rs.contains("fn sql_exec(host: Host, left_table: String")
        || rs.contains("fn sql_exec(host: Host, mut left_table: String");
    assert!(
        owned_left,
        "expected owned left_table String on sql_exec:\n{rs}"
    );
    // The compiler may demote concat-only `right_table` to `&str`; `&right_name` is then correct.
    let right_demoted = rs.contains("right_table: &str");
    let bad_owned = rs.contains("&emit.table")
        || rs.contains("&emit.sql")
        || (!right_demoted
            && (rs.contains("&right_name") || rs.contains("sql_exec(host, &")));
    assert!(
        !bad_owned,
        "RED: owned sql_exec slots received &emit.*/&right_name:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}


#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! FAILING REPRO (dogfood):
//!
//! `std::db` / `windjammer_runtime::db::Connection::query` takes `params: Vec<String>`
//! by value. Codegen emits `&vec![...]` at call sites →
//! `expected Vec<String>, found &Vec<String>` (22× on platform `make test`).
//!
//! WJ `std/db.wj` declares owned `Vec<string>`; runtime Rust keeps owned `Vec<String>`.
//! Call sites must pass owned vec literals, not auto-borrowed `&vec![...]`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_db_query_vec_params_must_not_auto_borrow() {
    use std::fs;
    use tempfile::TempDir;
    use windjammer::compiler::build_project;
    use windjammer::CompilationTarget;

    let source = r#"
use std::db

pub fn list_for_tenant(tenant_slug: string) -> int {
    let conn = match db.connect("sqlite::memory:" + "") {
        Ok(c) => c,
        Err(_) => return 0,
    }
    let rows = match conn.query("select 1 where $1 = $1" + "", vec![tenant_slug + ""]) {
        Ok(r) => r,
        Err(_) => return 0,
    }
    rows.len()
}

fn main() {
    let _ = list_for_tenant("demo" + "")
}
"#;

    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("db_vec.wj");
    fs::write(&wj, source).unwrap();
    let out = tmp.path().join("build");
    build_project(&wj, &out, CompilationTarget::Rust, false).expect("compile");
    let rs = fs::read_to_string(out.join("db_vec.rs")).unwrap_or_default();

    assert!(
        !rs.contains("&vec!["),
        "Connection::query params are Vec<String> by value (runtime); must not emit &vec![...] (dogfood). Got:\n{rs}"
    );
}

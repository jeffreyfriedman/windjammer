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
//! Platform pattern:
//! `conn.query(insert_sql(), vec![slug + "", seq + "", event + "", ...])`
//! Codegen emits:
//! `conn.query(&insert_sql(), &vec![format!(...), format!(...), ...])`
//! while `windjammer_runtime::db::Connection::query` expects `params: Vec<String>`.
//!
//! Simple `vec![tenant_slug]` often stays owned; multi-element `string + ""` temps
//! still get auto-borrowed as `&vec![...]`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn multi_format_vec_params_must_not_auto_borrow() {
    use std::fs;
    use tempfile::TempDir;
    use windjammer::compiler::build_project;
    use windjammer::CompilationTarget;

    let source = r#"
use std::db

fn insert_sql() -> string {
    "insert into outbox values ($1,$2,$3,$4,$5)" + ""
}

pub fn append_row(
    slug: string,
    seq: string,
    event_type: string,
    schema: string,
    payload: string,
) -> int {
    let conn = match db.connect("sqlite::memory:" + "") {
        Ok(c) => c,
        Err(_) => return 0,
    }
    let rows = match conn.query(
        insert_sql(),
        vec![slug + "", seq + "", event_type + "", schema + "", payload + ""],
    ) {
        Ok(r) => r,
        Err(_) => return 0,
    }
    rows.len()
}

fn main() {
    let _ = append_row("demo" + "", "1" + "", "journal.posted" + "", "1" + "", "{}" + "")
}
"#;

    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("db_multi_vec.wj");
    fs::write(&wj, source).unwrap();
    let out = tmp.path().join("build");
    build_project(&wj, &out, CompilationTarget::Rust, false).expect("compile");
    let rs = fs::read_to_string(out.join("db_multi_vec.rs")).unwrap_or_default();

    assert!(
        !rs.contains("&vec!["),
        "multi-element query params must stay owned Vec (dogfood). Got:\n{rs}"
    );
}

//! `std::db` connect + execute must be usable for migrations and `cargo check`.

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
    feature = "integration_tests",
))]

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_db_connect_execute_codegen_resolves() {
    let source = r#"
use std::db

pub fn apply(url: string, sql: string) -> Result<int, string> {
    match db.connect(url) {
        Ok(conn) => {
            let mut params = Vec::new()
            conn.execute(sql, params)
        },
        Err(e) => Err(e),
    }
}
"#;
    let generated = test_utils::assert_stdlib_runtime_links(source, &["db::connect", "execute("]);
    assert!(
        generated.contains("execute("),
        "db.execute must appear in generated Rust, got:\n{generated}"
    );
}

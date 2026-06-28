#[test]
fn db_query_vec_param_not_borrowed() {
    use std::fs;
    use tempfile::TempDir;
    use windjammer::compiler::build_project;
    use windjammer::CompilationTarget;

    let source = r#"
use std::db
pub fn f(tenant_slug: string) -> Vec<Row> {
    let conn = match db.connect("u") { Ok(c) => c, Err(_) => return vec![] }
    match conn.query("sql", vec![tenant_slug]) { Ok(r) => r, Err(_) => vec![] }
}
"#;
    let tmp = TempDir::new().unwrap();
    let wj = tmp.path().join("t.wj");
    fs::write(&wj, source).unwrap();
    let out = tmp.path().join("build");
    build_project(&wj, &out, CompilationTarget::Rust, false).unwrap();
    let rs = fs::read_to_string(out.join("t.rs")).unwrap();
    eprintln!("GENERATED:\n{rs}");
    assert!(
        !rs.contains("&vec![tenant_slug]"),
        "must not borrow vec! params:\n{rs}"
    );
}

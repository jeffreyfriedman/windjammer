//! LSP database IDE analysis integration tests.

use tower_lsp::lsp_types::Url;
use windjammer_lsp::database::WindjammerDatabase;

#[test]
fn test_salsa_db_ide_analysis_infers_return_type() {
    let source = r#"
pub fn identity(x: u32) -> u32 {
    x
}
"#;
    let mut db = WindjammerDatabase::new();
    let uri = Url::from_file_path("/tmp/id.wj").unwrap();
    let file = db.set_source_text(uri, source.to_string());
    let analysis = db.get_ide_analysis(file);
    assert!(analysis.success);
    assert_eq!(
        analysis.inferred_types().get("identity::return"),
        Some(&"u32".to_string())
    );
}

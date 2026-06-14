//! Integration tests for `windjammer::ide_analysis` and LSP wiring.

use std::path::PathBuf;
use windjammer::ide_analysis::{analyze_source, IdeAnalysisOptions};

#[test]
fn test_ide_analysis_infers_function_return_type() {
    let source = r#"
pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}
"#;
    let result = analyze_source(
        source,
        IdeAnalysisOptions {
            enable_lint: false,
            file_path: PathBuf::from("math.wj"),
        },
    );
    assert!(result.success, "{:?}", result.diagnostics);
    assert_eq!(
        result.inferred_types.get("multiply::return"),
        Some(&"i32".to_string())
    );
}

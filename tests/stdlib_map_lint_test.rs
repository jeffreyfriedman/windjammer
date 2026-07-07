//! TDD: std/map.wj must pass rust-leakage lint (W0001).

use std::fs;
use std::path::PathBuf;

use windjammer::lexer::Lexer;
use windjammer::linter::rust_leakage::RustLeakageLinter;
use windjammer::parser::Parser;

fn lint_file(path: &PathBuf) -> Vec<windjammer::linter::LintDiagnostic> {
    let source = fs::read_to_string(path).expect("read stdlib source");
    let file_name = path.to_string_lossy().to_string();
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, file_name.clone(), source.clone());
    let program = parser.parse().expect("std/map.wj should parse");
    let mut linter = RustLeakageLinter::new(&file_name);
    linter.lint_program(&program);
    linter.into_diagnostics()
}

#[test]
fn test_std_map_wj_has_no_w0001_rust_leakage() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("std/map.wj");
    let warnings = lint_file(&path);
    let w0001: Vec<_> = warnings
        .iter()
        .filter(|w| w.lint_name == "W0001")
        .collect();
    assert!(
        w0001.is_empty(),
        "std/map.wj must use inferred ownership (no W0001). Found: {:?}",
        w0001
    );
}

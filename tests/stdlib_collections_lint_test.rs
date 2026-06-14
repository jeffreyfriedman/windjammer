//! TDD: std/collections.wj must pass rust-leakage lint (W0001).
//!
//! Engine library builds run the linter on every module including pulled-in stdlib.
//! Explicit `&self` / `&K` in collections.wj blocked full engine transpile.

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
    let program = parser.parse().expect("std/collections.wj should parse");
    let mut linter = RustLeakageLinter::new(&file_name);
    linter.lint_program(&program);
    linter.into_diagnostics()
}

#[test]
fn test_std_collections_wj_has_no_w0001_rust_leakage() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("std/collections.wj");
    let warnings = lint_file(&path);
    let w0001: Vec<_> = warnings
        .iter()
        .filter(|w| w.lint_name == "W0001")
        .collect();
    assert!(
        w0001.is_empty(),
        "std/collections.wj must use inferred ownership (no W0001). Found: {:?}",
        w0001
    );
}

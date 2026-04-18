use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_to_rust(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_fns, registry, _) = analyzer.analyze_program(&program).unwrap();
    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed_fns)
}

#[test]
fn test_std_fs_qualified_path_compiles() {
    // Qualified std::fs::read_to_string should pass through as valid Rust
    let output = compile_to_rust(r#"
fn read_file(path: String) -> String {
    let content = std::fs::read_to_string(path)
    content
}
"#);
    assert!(
        output.contains("std::fs::read_to_string"),
        "Qualified std::fs::read_to_string should pass through. Got:\n{}",
        output
    );
}

#[test]
fn test_use_std_fs_generates_import() {
    // `use std::fs` should generate a Rust import so unqualified fs::read_to_string works
    let output = compile_to_rust(r#"
use std::fs

fn read_file(path: String) -> String {
    let content = fs::read_to_string(path)
    content
}
"#);
    // Should generate `use std::fs;` or similar
    assert!(
        output.contains("use std::fs"),
        "use std::fs should generate an import. Got:\n{}",
        output
    );
}

#[test]
fn test_std_fs_write_qualified() {
    let output = compile_to_rust(r#"
fn write_file(path: String, data: String) {
    std::fs::write(path, data)
}
"#);
    assert!(
        output.contains("std::fs::write"),
        "std::fs::write should pass through. Got:\n{}",
        output
    );
}

#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use std::fs;

use tempfile::TempDir;
use windjammer::analyzer::{Analyzer, OwnershipMode};
use windjammer::build_project;
use windjammer::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg;
use windjammer::CompilationTarget;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Type};

fn compile_wj(source: &str) -> String {
    let tmp = TempDir::new().expect("tempdir");
    let wj_file = tmp.path().join("test.wj");
    fs::write(&wj_file, source).unwrap();
    let out_dir = tmp.path().join("build");
    build_project(&wj_file, &out_dir, CompilationTarget::Rust, false)
        .expect("build_project");
    fs::read_to_string(out_dir.join("test.rs")).expect("read test.rs")
}

#[test]
fn circular_dep_codegen_passes_owned_string_at_call_sites() {
    let source = r#"
fn foo(x: string) -> bool {
    if x == "stop" { true } else { bar(x) }
}

fn bar(y: string) -> bool {
    if y == "stop" { false } else { foo(y) }
}
"#;
    let generated = compile_wj(source);
    assert!(
        generated.contains("bar(x)") && generated.contains("foo(y)"),
        "mutual recursion must pass owned String at call sites.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("bar(&x)") && !generated.contains("foo(&y)"),
        "stale borrow metadata must not emit & on plain string formals.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn circular_dep_bar_call_site_uses_owned_string() {
    let source = r#"
fn foo(x: string) -> bool {
    if x == "stop" { true } else { bar(x) }
}

fn bar(y: string) -> bool {
    if y == "stop" { false } else { foo(y) }
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse");
    let mut analyzer = Analyzer::new();
    let (_, registry, _) = analyzer.analyze_program(&program).expect("analyze");

    let bar = registry.get_signature("bar").expect("bar signature");
    assert_eq!(bar.formal_param_type(0), Some(&Type::String));
    assert_eq!(
        effective_param_ownership_for_arg(bar, 0),
        OwnershipMode::Owned,
        "bar(y: string) call sites must pass owned String (formal={:?}, param_types={:?}, ownership={:?})",
        bar.formal_param_types,
        bar.param_types,
        bar.param_ownership,
    );
}

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: .clone() in WJ source should produce a W0005 warning
//!
//! .clone() is Rust leakage -- Windjammer handles cloning automatically.
//! The RustLeakageLinter should emit a warning when it detects .clone() in user code.

use windjammer::lexer::Lexer;
use windjammer::linter::rust_leakage::RustLeakageLinter;
use windjammer::parser::Parser;

fn lint_source(source: &str) -> Vec<String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut linter = RustLeakageLinter::new("test.wj");
    linter.lint_program(&program);
    linter
        .diagnostics()
        .iter()
        .map(|d| format!("{}: {}", d.lint_name, d.message))
        .collect()
}

#[test]
fn test_clone_in_wj_produces_warning() {
    let source = r#"
pub struct Foo { pub name: String }
pub fn do_stuff(items: Vec<Foo>) -> Vec<Foo> {
    let first = items[0].clone()
    let mut result: Vec<Foo> = Vec::new()
    result.push(first)
    result
}
"#;

    let diagnostics = lint_source(source);
    let clone_warnings: Vec<_> = diagnostics.iter().filter(|d| d.contains("W0005")).collect();

    assert!(
        !clone_warnings.is_empty(),
        "Expected W0005 warning for .clone() usage, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_no_clone_no_warning() {
    let source = r#"
pub struct Foo { pub value: i32 }
pub fn get_value(foo: Foo) -> i32 {
    foo.value
}
"#;

    let diagnostics = lint_source(source);
    let clone_warnings: Vec<_> = diagnostics.iter().filter(|d| d.contains("W0005")).collect();

    assert!(
        clone_warnings.is_empty(),
        "No .clone() used, should have no W0005 warning, got: {:?}",
        clone_warnings
    );
}

#[test]
fn test_clone_in_method_produces_warning() {
    let source = r#"
pub struct Data { pub items: Vec<String> }
impl Data {
    pub fn get_first(self) -> String {
        self.items[0].clone()
    }
}
"#;

    let diagnostics = lint_source(source);
    let clone_warnings: Vec<_> = diagnostics.iter().filter(|d| d.contains("W0005")).collect();

    assert!(
        !clone_warnings.is_empty(),
        "Expected W0005 warning for .clone() in method, got: {:?}",
        diagnostics
    );
}

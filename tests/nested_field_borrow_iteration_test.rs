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
fn test_single_field_access_borrows_for_iteration() {
    // Baseline: for x in self.items should borrow
    let output = compile_to_rust(
        r#"
struct Container {
    items: Vec<i32>,
}

impl Container {
    fn sum(self) -> i32 {
        let mut total = 0
        for item in self.items {
            total = total + item
        }
        total
    }
}
"#,
    );
    assert!(
        output.contains("&self.items") || output.contains("& self.items"),
        "for x in self.items should borrow (&self.items). Got:\n{}",
        output
    );
}

#[test]
fn test_nested_field_access_borrows_for_iteration() {
    // Bug: for x in self.renderer.items should also borrow, but doesn't
    // because should_borrow_for_iteration only checks one level of FieldAccess
    let output = compile_to_rust(
        r#"
struct Renderer {
    items: Vec<i32>,
}

struct Engine {
    renderer: Renderer,
}

impl Engine {
    fn total(self) -> i32 {
        let mut sum = 0
        for item in self.renderer.items {
            sum = sum + item
        }
        sum
    }
}
"#,
    );
    assert!(
        output.contains("&self.renderer.items") || output.contains("& self.renderer.items"),
        "for x in self.renderer.items should borrow. Got:\n{}",
        output
    );
}

#[test]
fn test_triple_nested_field_access_borrows() {
    let output = compile_to_rust(
        r#"
struct Inner {
    values: Vec<i32>,
}

struct Middle {
    inner: Inner,
}

struct Outer {
    middle: Middle,
}

impl Outer {
    fn count(self) -> i32 {
        let mut n = 0
        for val in self.middle.inner.values {
            n = n + 1
        }
        n
    }
}
"#,
    );
    assert!(
        output.contains("&self.middle.inner.values"),
        "for x in self.middle.inner.values should borrow. Got:\n{}",
        output
    );
}

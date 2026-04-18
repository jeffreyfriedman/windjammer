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
fn test_vec_remove_zero_no_wrong_suffix() {
    let output = compile_to_rust(r#"
fn remove_first(items: Vec<i32>) -> i32 {
    let first = items.remove(0)
    first
}
"#);
    // Should NOT contain i32 or u32 suffix for index arg
    assert!(
        !output.contains("remove(0_u32") && !output.contains("remove(0_i32"),
        "Vec::remove should not generate u32/i32 suffix. Got:\n{}",
        output
    );
}

#[test]
fn test_vec_remove_explicit_usize_cast() {
    // When using an explicit usize variable, it should pass through correctly
    let output = compile_to_rust(r#"
fn remove_at_index(items: Vec<i32>) -> i32 {
    let idx: usize = 0
    items.remove(idx)
}
"#);
    assert!(
        output.contains("remove(idx"),
        "Should use the usize variable directly. Got:\n{}",
        output
    );
}

#[test]
fn test_vec_insert_zero_no_wrong_suffix() {
    let output = compile_to_rust(r#"
fn insert_first(items: Vec<i32>, value: i32) {
    items.insert(0, value)
}
"#);
    assert!(
        !output.contains("insert(0_u32") && !output.contains("insert(0_i32"),
        "Vec::insert should not generate u32/i32 suffix for index. Got:\n{}",
        output
    );
}

#[test]
fn test_codegen_rewrites_all_integer_suffixes_to_usize() {
    // Verify the rewrite logic handles all suffixes by checking the codegen
    // doesn't produce any non-usize integer suffix for index methods.
    // This covers the case where int inference picks u32 in complex contexts.
    let output = compile_to_rust(r#"
struct Queue {
    items: Vec<i32>,
}

impl Queue {
    fn pop_front(self) -> i32 {
        self.items.remove(0)
    }
}
"#);
    assert!(
        !output.contains("remove(0_u32") && !output.contains("remove(0_i32")
            && !output.contains("remove(0_i64") && !output.contains("remove(0_u64"),
        "No non-usize integer suffix should appear for remove. Got:\n{}",
        output
    );
}

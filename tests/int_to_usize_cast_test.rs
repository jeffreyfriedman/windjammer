use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
/// TDD Test: i64 to usize casting
///
/// Dogfooding Issue: `error[E0606]: casting &i64 as usize is invalid`
///
/// Windjammer code that tries to cast an `&i64` to `usize` should either:
/// 1. Dereference first: `(*value) as usize`
/// 2. Or better: automatically handle the dereference when the type requires it
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_code(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_ref_int_to_usize_cast() {
    let code = r#"
    fn get_index(value: int) -> usize {
        return value as usize
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should generate: (value as usize) or (*value as usize) if value is &i64
    // But NOT: (value as usize) where value is &i64
    assert!(
        !output.contains("&i64 as usize"),
        "Should not try to cast &i64 to usize directly"
    );
}

#[test]
fn test_int_field_to_usize_cast() {
    let code = r#"
    struct Counter { count: int }
    
    fn get_index(counter: Counter) -> usize {
        return counter.count as usize
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should handle field access correctly in casts
    assert!(
        output.contains("counter.count as usize") || output.contains("counter.count) as usize"),
        "Should generate valid cast from field access"
    );
}

#[test]
fn test_method_result_to_usize_cast() {
    let code = r#"
    struct Value { data: int }
    
    impl Value {
        fn get(self) -> int {
            return self.data
        }
    }
    
    fn process(v: Value) -> usize {
        return v.get() as usize
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should handle method call results in casts
    assert!(
        output.contains("v.get() as usize") || output.contains("v.get()) as usize"),
        "Should generate valid cast from method call"
    );
}


//! Test decorator syntax for testing framework
//!
//! Tests that decorators like @timeout, @bench, @requires, @ensures, etc.
//! generate correct Rust code that wraps function bodies appropriately.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_timeout_decorator() {
    let source = r#"
    @timeout(1000)
    @test
    fn my_test() {
        assert_eq(1 + 1, 2);
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should use with_timeout from runtime
    assert!(output.contains("with_timeout"));
    assert!(output.contains("Duration::from_millis(1000)"));
    assert!(output.contains("assert_eq"));  // Expression may be simplified
    assert!(output.contains(".unwrap()"));  // Timeout returns Result
}

#[test]
fn test_bench_decorator() {
    let source = r#"
    @bench
    fn benchmark_add() {
        let x = 1 + 1;
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should wrap with bench() call
    assert!(output.contains("bench(||"));
    assert!(output.contains("let x ="));  // Expression may be simplified
    assert!(output.contains("println!(\"Benchmark:"));
}

#[test]
fn test_requires_decorator() {
    let source = r#"
    @requires(x > 0)
    @requires(y > 0)
    fn add(x: int, y: int) -> int {
        x + y
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should have requires() calls at start of function
    assert!(output.contains("requires(x > 0"));
    assert!(output.contains("requires(y > 0"));
}

#[test]
fn test_ensures_decorator() {
    let source = r#"
    @ensures(result > 0)
    fn abs(x: int) -> int {
        if x < 0 { -x } else { x }
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should have result capture and ensures() check
    assert!(output.contains("let __result ="));
    assert!(output.contains("ensures(__result > 0"));
}

#[test]
fn test_invariant_decorator() {
    let source = r#"
    @invariant(count >= 0)
    fn increment(count: int) -> int {
        count + 1
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should have invariant check at end
    assert!(output.contains("invariant("));
}

#[test]
fn test_property_test_decorator() {
    let source = r#"
    @property_test(100)
    fn test_commutative(a: int, b: int) {
        assert_eq(a + b, b + a);
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should wrap with property_test_with_gen2
    assert!(output.contains("property_test_with_gen2"));
    assert!(output.contains("100"));
}

#[test]
fn test_test_with_setup_teardown() {
    let source = r#"
    @test(setup = setup_db, teardown = teardown_db)
    fn test_database(db: Database) {
        assert(db.is_connected());
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should use with_setup_teardown
    assert!(output.contains("with_setup_teardown"));
    assert!(output.contains("setup_db"));
    assert!(output.contains("teardown_db"));
}

#[test]
fn test_combined_decorators() {
    let source = r#"
    @timeout(5000)
    @bench
    @test
    fn my_test() {
        let x = 1 + 1;
    }
    "#;
    
    let output = parse_and_generate(source);
    println!("Generated Rust:\n{}", output);
    
    // Should have both timeout and bench
    assert!(output.contains("with_timeout"));
    assert!(output.contains("Duration::from_millis(5000)"));
    assert!(output.contains("bench(||"));
}


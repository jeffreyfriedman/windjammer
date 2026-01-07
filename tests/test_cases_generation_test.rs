// Test that @test_cases decorator generates multiple test functions

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
fn test_test_cases_basic() {
    let source = r#"
@test_cases([
    (5, 3, 8),
    (10, -5, 5),
    (0, 0, 0),
])
fn add_numbers(a: int, b: int, expected: int) {
    assert_eq(a + b, expected);
}
"#;

    let rust_code = parse_and_generate(source);

    println!("Generated Rust:\n{}", rust_code);

    // Should generate 3 test functions
    assert!(
        rust_code.contains("fn add_numbers_case_0()"),
        "Missing case 0"
    );
    assert!(
        rust_code.contains("fn add_numbers_case_1()"),
        "Missing case 1"
    );
    assert!(
        rust_code.contains("fn add_numbers_case_2()"),
        "Missing case 2"
    );

    // Should generate the implementation function
    assert!(
        rust_code.contains("fn add_numbers_impl("),
        "Missing impl function"
    );

    // Should have #[test] on each case
    assert_eq!(
        rust_code.matches("#[test]").count(),
        3,
        "Should have 3 #[test] attributes"
    );

    // Should call impl with correct arguments
    assert!(
        rust_code.contains("add_numbers_impl(5, 3, 8)"),
        "Missing case 0 call"
    );
    assert!(
        rust_code.contains("add_numbers_impl(10, -5, 5)")
            || rust_code.contains("add_numbers_impl(10, (-5), 5)"),
        "Missing case 1 call"
    );
    assert!(
        rust_code.contains("add_numbers_impl(0, 0, 0)"),
        "Missing case 2 call"
    );
}

#[test]
fn test_test_cases_string() {
    let source = r#"
@test_cases([
    ("hello", 5),
    ("world", 5),
    ("", 0),
])
fn string_length(input: string, expected: int) {
    let len = input.len() as int;
    assert_eq(len, expected);
}
"#;

    let rust_code = parse_and_generate(source);

    // Should generate 3 test functions
    assert!(
        rust_code.contains("fn string_length_case_0()"),
        "Missing case 0"
    );
    assert!(
        rust_code.contains("fn string_length_case_1()"),
        "Missing case 1"
    );
    assert!(
        rust_code.contains("fn string_length_case_2()"),
        "Missing case 2"
    );

    // Should generate the implementation function
    assert!(
        rust_code.contains("fn string_length_impl("),
        "Missing impl function"
    );

    // Should have #[test] on each case
    assert_eq!(
        rust_code.matches("#[test]").count(),
        3,
        "Should have 3 #[test] attributes"
    );
}

#[test]
fn test_test_cases_bool() {
    let source = r#"
@test_cases([
    (true, false),
    (false, true),
])
fn bool_negation(input: bool, expected: bool) {
    assert_eq(!input, expected);
}
"#;

    let rust_code = parse_and_generate(source);

    // Should generate 2 test functions
    assert!(
        rust_code.contains("fn bool_negation_case_0()"),
        "Missing case 0"
    );
    assert!(
        rust_code.contains("fn bool_negation_case_1()"),
        "Missing case 1"
    );

    // Should generate the implementation function
    assert!(
        rust_code.contains("fn bool_negation_impl("),
        "Missing impl function"
    );

    // Should have #[test] on each case
    assert_eq!(
        rust_code.matches("#[test]").count(),
        2,
        "Should have 2 #[test] attributes"
    );

    // Should call impl with correct arguments
    assert!(
        rust_code.contains("bool_negation_impl(true, false)"),
        "Missing case 0 call"
    );
    assert!(
        rust_code.contains("bool_negation_impl(false, true)"),
        "Missing case 1 call"
    );
}

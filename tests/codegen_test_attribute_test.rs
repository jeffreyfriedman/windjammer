// Test that Windjammer compiler generates #[test] attributes for test functions
//
// This is a TDD test for the compiler itself

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str, filename: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.set_source_file(filename);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_generates_test_attribute_for_test_functions() {
    let input = r#"
pub fn test_example() {
    assert_eq(1, 1)
}

pub fn test_another() {
    assert_eq(2, 2)
}

pub fn not_a_test() {
    // Regular function, should NOT get #[test]
}
"#;
    
    let generated_rust = parse_and_generate(input, "test_file_test.wj");
    
    // Should generate #[test] before test functions
    assert!(generated_rust.contains("#[test]"), 
            "Expected #[test] attribute for test_example");
    
    // Count occurrences - should have 2 #[test] attributes (one per test)
    let test_attr_count = generated_rust.matches("#[test]").count();
    assert_eq!(test_attr_count, 2, 
               "Expected 2 #[test] attributes, found {}", test_attr_count);
    
    // Should generate test functions
    assert!(generated_rust.contains("pub fn test_example()"),
            "Expected test_example function");
    assert!(generated_rust.contains("pub fn test_another()"),
            "Expected test_another function");
    
    // Regular function should NOT have #[test]
    let lines: Vec<&str> = generated_rust.lines().collect();
    let mut found_not_a_test = false;
    let mut prev_line_was_test_attr = false;
    
    for line in lines {
        if line.contains("pub fn not_a_test") {
            found_not_a_test = true;
            assert!(!prev_line_was_test_attr, 
                    "not_a_test should NOT have #[test] attribute");
        }
        prev_line_was_test_attr = line.trim() == "#[test]";
    }
    
    assert!(found_not_a_test, "Expected to find not_a_test function");
}

#[test]
fn test_generates_runtime_import_for_test_files() {
    let input = r#"
pub fn test_with_assertion() {
    assert_eq(1, 1)
    assert_gt(2, 1)
    assert_approx_f32(1.0, 1.0, 0.01)
}
"#;
    
    let generated_rust = parse_and_generate(input, "module_test.wj");
    
    // Should auto-import windjammer_runtime::test::*
    assert!(generated_rust.contains("use windjammer_runtime::test::*") ||
            generated_rust.contains("use crate::test::*"),
            "Expected runtime test import for test file");
}

#[test]
fn test_test_functions_must_start_with_test_prefix() {
    let input = r#"
pub fn test_valid() {
    assert_eq(1, 1)
}

pub fn my_test() {
    // Does NOT start with "test_", should NOT get #[test]
    assert_eq(2, 2)
}
"#;
    
    let generated_rust = parse_and_generate(input, "something_test.wj");
    
    // Should have exactly 1 #[test] attribute
    let test_attr_count = generated_rust.matches("#[test]").count();
    assert_eq!(test_attr_count, 1, 
               "Only test_valid should get #[test], found {} attributes", test_attr_count);
}

#[test]
fn test_test_files_detected_by_suffix() {
    // Test that *_test.wj files are recognized as test files
    let input = r#"
pub fn test_something() {
    assert_eq(1, 1)
}
"#;
    
    let result_test_file = parse_and_generate(input, "my_module_test.wj");
    let result_regular_file = parse_and_generate(input, "my_module.wj");
    
    // Test files should get test attributes
    assert!(result_test_file.contains("#[test]"),
            "Expected #[test] in *_test.wj file");
    
    // Regular files should NOT get test attributes for test_ functions
    // (unless they're in a tests/ directory)
    assert!(!result_regular_file.contains("#[test]"),
            "Regular .wj file should not auto-add #[test] attributes");
}

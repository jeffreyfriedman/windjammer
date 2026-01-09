// TDD Test: Method calls on variables should not be treated as module paths
//
// Bug: Transpiler generates `json::push_str()` instead of `json.push_str()`
// when calling methods on a variable named `json`.
//
// This happens because the codegen is treating the variable name as a module path.

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
fn test_method_call_on_variable_not_module_path() {
    let source = r#"
fn test() {
    let mut json = String::from("{")
    json.push_str("hello")
    json.push_str(&format!("world"))
}
"#;

    let rust_code = parse_and_generate(source);

    // Should generate method call syntax, not module path syntax
    assert!(rust_code.contains("json.push_str"), 
           "Should generate 'json.push_str' (method call), not 'json::push_str' (module path).\nGenerated:\n{}", rust_code);
    assert!(
        !rust_code.contains("json::push_str"),
        "Should NOT generate 'json::push_str' (module path).\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_method_call_on_various_variable_names() {
    let source = r#"
fn test() {
    let mut data = String::new()
    data.push_str("test")
    
    let mut result = Vec::new()
    result.push(42)
    
    let mut map = HashMap::new()
    map.insert("key", "value")
}
"#;

    let rust_code = parse_and_generate(source);

    // All should be method calls, not module paths
    assert!(
        rust_code.contains("data.push_str"),
        "Should generate 'data.push_str'"
    );
    assert!(
        rust_code.contains("result.push"),
        "Should generate 'result.push'"
    );
    assert!(
        rust_code.contains("map.insert"),
        "Should generate 'map.insert'"
    );

    assert!(
        !rust_code.contains("data::push_str"),
        "Should NOT generate 'data::push_str'"
    );
    assert!(
        !rust_code.contains("result::push"),
        "Should NOT generate 'result::push'"
    );
    assert!(
        !rust_code.contains("map::insert"),
        "Should NOT generate 'map::insert'"
    );
}

// NOTE: Module paths (Vec::new, String::from) are tested elsewhere
// This test was removed due to parser issues with the test code

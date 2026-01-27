use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
/// TDD Test: Auto-ref with cast expressions
///
/// Dogfooding Issue: `error[E0606]: casting &i64 as usize is invalid`
///
/// When auto-ref adds `&` to an argument that is a cast expression,
/// it should add `&` to the WHOLE cast, not the operand:
/// WRONG: `vec.remove(&index as usize)`  // casts &i64 to usize
/// RIGHT: `vec.remove(&(index as usize))` // casts i64 to usize, then borrows
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
fn test_vec_remove_with_cast() {
    let code = r#"
    fn remove_at(items: Vec<string>, index: int) -> Vec<string> {
        let mut items = items
        items.remove(index as usize)
        return items
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Vec::remove expects usize by value, not &usize
    // So we should NOT add & at all for cast expressions to usize
    // OR if we do add &, it should be &(index as usize), not &index as usize
    assert!(
        !output.contains("&index as usize"),
        "Should not generate `&index as usize` (invalid cast)"
    );

    // Should either be:
    // - items.remove(index as usize) - no & because usize is Copy
    // - items.remove(&(index as usize)) - & around the whole cast
    let has_valid_syntax = output.contains("items.remove(index as usize)")
        || output.contains("items.remove(&(index as usize))");

    assert!(
        has_valid_syntax,
        "Should generate valid syntax for Vec::remove with cast"
    );
}

#[test]
fn test_hashmap_remove_with_cast() {
    let code = r#"
    use std::collections::HashMap
    
    fn remove_entry(map: HashMap<usize, string>, key: int) -> HashMap<usize, string> {
        let mut map = map
        map.remove(key as usize)
        return map
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // HashMap::remove expects &K, but K=usize is Copy
    // So we should NOT add & at all
    assert!(
        !output.contains("&key as usize"),
        "Should not generate `&key as usize` (invalid cast)"
    );

    // For Copy types like usize, we don't need & for HashMap::remove either
    assert!(
        output.contains("map.remove(key as usize)")
            || output.contains("map.remove(&(key as usize))"),
        "Should generate valid syntax for HashMap::remove with cast"
    );
}

#[test]
fn test_method_call_with_multiple_casts() {
    let code = r#"
    struct Values { data: Vec<int> }
    
    impl Values {
        fn get_range(self, start: int, end: int) -> Vec<int> {
            let start_idx = start as usize
            let end_idx = end as usize
            return self.data[start_idx..end_idx].to_vec()
        }
    }
    "#;

    let output = compile_code(code);
    println!("Generated:\n{}", output);

    // Should handle casts in variable assignments correctly
    assert!(
        output.contains("let start_idx = start as usize"),
        "Should handle cast in let binding"
    );
    assert!(
        output.contains("let end_idx = end as usize"),
        "Should handle cast in let binding"
    );
}

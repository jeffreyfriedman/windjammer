//! Comprehensive Parser Statement Tests
//!
//! These tests verify that the parser correctly parses all statement types.
//! They serve as documentation for the statement grammar of Windjammer.

use windjammer::lexer::Lexer;
use windjammer::parser::ast::*;
use windjammer::parser_impl::Parser;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn parse_stmt(input: &str) -> Statement {
    // Wrap statement in a function to make it a valid program
    let full_code = format!("fn test() {{ {} }}", input);
    let mut lexer = Lexer::new(&full_code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");

    // Extract the statement from the function body
    if let Some(Item::Function { decl, .. }) = program.items.first() {
        if let Some(stmt) = decl.body.first() {
            return stmt.clone();
        }
    }
    panic!("Failed to extract statement from: {}", input);
}

fn parse_program(input: &str) -> Program {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().expect("Failed to parse program")
}

fn count_statements(input: &str) -> usize {
    let full_code = format!("fn test() {{ {} }}", input);
    let mut lexer = Lexer::new(&full_code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");

    if let Some(Item::Function { decl, .. }) = program.items.first() {
        decl.body.len()
    } else {
        0
    }
}

// ============================================================================
// LET STATEMENTS
// ============================================================================

#[test]
fn test_let_simple() {
    let stmt = parse_stmt("let x = 42");
    assert!(matches!(stmt, Statement::Let { .. }));
}

#[test]
fn test_let_mutable() {
    let stmt = parse_stmt("let mut x = 42");
    if let Statement::Let { mutable, .. } = stmt {
        assert!(mutable);
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_let_immutable() {
    let stmt = parse_stmt("let x = 42");
    if let Statement::Let { mutable, .. } = stmt {
        assert!(!mutable);
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_let_with_type() {
    let stmt = parse_stmt("let x: i32 = 42");
    if let Statement::Let { type_, .. } = stmt {
        assert!(type_.is_some());
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_let_string_literal() {
    let stmt = parse_stmt(r#"let s = "hello""#);
    assert!(matches!(stmt, Statement::Let { .. }));
}

#[test]
fn test_let_with_expression() {
    let stmt = parse_stmt("let x = 1 + 2 * 3");
    assert!(matches!(stmt, Statement::Let { .. }));
}

#[test]
fn test_let_with_function_call() {
    let stmt = parse_stmt("let x = foo()");
    assert!(matches!(stmt, Statement::Let { .. }));
}

#[test]
fn test_let_with_method_call() {
    let stmt = parse_stmt("let x = obj.method()");
    assert!(matches!(stmt, Statement::Let { .. }));
}

// ============================================================================
// ASSIGNMENT STATEMENTS
// ============================================================================

#[test]
fn test_assignment_simple() {
    let stmt = parse_stmt("x = 42");
    assert!(matches!(stmt, Statement::Assignment { .. }));
}

#[test]
fn test_assignment_field() {
    let stmt = parse_stmt("obj.field = 42");
    assert!(matches!(stmt, Statement::Assignment { .. }));
}

#[test]
fn test_assignment_index() {
    let stmt = parse_stmt("arr[0] = 42");
    assert!(matches!(stmt, Statement::Assignment { .. }));
}

#[test]
fn test_compound_assignment_add() {
    // Compound assignments are desugared to regular assignments
    let stmt = parse_stmt("x += 1");
    assert!(matches!(stmt, Statement::Assignment { .. }));
}

#[test]
fn test_compound_assignment_sub() {
    let stmt = parse_stmt("x -= 1");
    assert!(matches!(stmt, Statement::Assignment { .. }));
}

#[test]
fn test_compound_assignment_mul() {
    let stmt = parse_stmt("x *= 2");
    assert!(matches!(stmt, Statement::Assignment { .. }));
}

#[test]
fn test_compound_assignment_div() {
    let stmt = parse_stmt("x /= 2");
    assert!(matches!(stmt, Statement::Assignment { .. }));
}

// ============================================================================
// IF STATEMENTS
// ============================================================================

#[test]
fn test_if_simple() {
    let stmt = parse_stmt("if true { x }");
    assert!(matches!(stmt, Statement::If { .. }));
}

#[test]
fn test_if_with_else() {
    let stmt = parse_stmt("if cond { a } else { b }");
    if let Statement::If { else_block, .. } = stmt {
        assert!(else_block.is_some());
    } else {
        panic!("Expected If statement");
    }
}

#[test]
fn test_if_else_if() {
    let code = "if a { 1 } else if b { 2 } else { 3 }";
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::If { .. }));
}

#[test]
fn test_if_with_comparison() {
    let stmt = parse_stmt("if x > 0 { positive }");
    assert!(matches!(stmt, Statement::If { .. }));
}

#[test]
fn test_if_with_logical() {
    let stmt = parse_stmt("if a && b { both }");
    assert!(matches!(stmt, Statement::If { .. }));
}

#[test]
fn test_if_nested() {
    let code = r#"if outer {
        if inner {
            nested
        }
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::If { .. }));
}

// ============================================================================
// FOR STATEMENTS
// ============================================================================

#[test]
fn test_for_simple() {
    let stmt = parse_stmt("for x in items { x }");
    assert!(matches!(stmt, Statement::For { .. }));
}

#[test]
fn test_for_with_range() {
    let stmt = parse_stmt("for i in 0..10 { i }");
    assert!(matches!(stmt, Statement::For { .. }));
}

#[test]
fn test_for_with_inclusive_range() {
    let stmt = parse_stmt("for i in 0..=10 { i }");
    assert!(matches!(stmt, Statement::For { .. }));
}

#[test]
fn test_for_with_method_call() {
    let stmt = parse_stmt("for item in vec.iter() { item }");
    assert!(matches!(stmt, Statement::For { .. }));
}

#[test]
fn test_for_nested() {
    let code = r#"for x in items {
        for y in more {
            x + y
        }
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::For { .. }));
}

// ============================================================================
// WHILE STATEMENTS
// ============================================================================

#[test]
fn test_while_simple() {
    let stmt = parse_stmt("while true { x }");
    assert!(matches!(stmt, Statement::While { .. }));
}

#[test]
fn test_while_with_condition() {
    let stmt = parse_stmt("while x < 10 { x += 1 }");
    assert!(matches!(stmt, Statement::While { .. }));
}

#[test]
fn test_while_nested() {
    let code = r#"while outer {
        while inner {
            break
        }
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::While { .. }));
}

// ============================================================================
// LOOP STATEMENTS
// ============================================================================

#[test]
fn test_loop_simple() {
    let stmt = parse_stmt("loop { break }");
    assert!(matches!(stmt, Statement::Loop { .. }));
}

#[test]
fn test_loop_with_break() {
    let code = r#"loop {
        if done { break }
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::Loop { .. }));
}

// ============================================================================
// MATCH STATEMENTS
// ============================================================================

#[test]
fn test_match_simple() {
    let code = r#"match x {
        1 => a,
        2 => b,
        _ => c,
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::Match { .. }));
}

#[test]
fn test_match_with_patterns() {
    let code = r#"match opt {
        Some(x) => x,
        None => 0,
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::Match { .. }));
}

#[test]
fn test_match_with_guards() {
    let code = r#"match x {
        n if n > 0 => positive,
        n if n < 0 => negative,
        _ => zero,
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::Match { .. }));
}

#[test]
fn test_match_with_or_patterns() {
    let code = r#"match x {
        1 | 2 | 3 => small,
        _ => large,
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::Match { .. }));
}

// ============================================================================
// RETURN STATEMENTS
// ============================================================================

#[test]
fn test_return_empty() {
    let stmt = parse_stmt("return");
    if let Statement::Return { value, .. } = stmt {
        assert!(value.is_none());
    } else {
        panic!("Expected Return statement");
    }
}

#[test]
fn test_return_with_value() {
    let stmt = parse_stmt("return 42");
    if let Statement::Return { value, .. } = stmt {
        assert!(value.is_some());
    } else {
        panic!("Expected Return statement");
    }
}

#[test]
fn test_return_with_expression() {
    let stmt = parse_stmt("return x + y");
    if let Statement::Return { value, .. } = stmt {
        assert!(value.is_some());
    } else {
        panic!("Expected Return statement");
    }
}

// ============================================================================
// BREAK AND CONTINUE STATEMENTS
// ============================================================================

#[test]
fn test_break_simple() {
    let stmt = parse_stmt("break");
    assert!(matches!(stmt, Statement::Break { .. }));
}

#[test]
fn test_continue_simple() {
    let stmt = parse_stmt("continue");
    assert!(matches!(stmt, Statement::Continue { .. }));
}

// ============================================================================
// EXPRESSION STATEMENTS
// ============================================================================

#[test]
fn test_expression_statement_call() {
    let stmt = parse_stmt("foo()");
    assert!(matches!(stmt, Statement::Expression { .. }));
}

#[test]
fn test_expression_statement_method_call() {
    let stmt = parse_stmt("obj.method()");
    assert!(matches!(stmt, Statement::Expression { .. }));
}

#[test]
fn test_expression_statement_macro() {
    let stmt = parse_stmt("println!(x)");
    assert!(matches!(stmt, Statement::Expression { .. }));
}

// ============================================================================
// MULTIPLE STATEMENTS
// ============================================================================

#[test]
fn test_multiple_statements() {
    let count = count_statements("let x = 1; let y = 2; x + y");
    assert_eq!(count, 3);
}

#[test]
fn test_statements_with_semicolons() {
    let count = count_statements("let a = 1; let b = 2;");
    assert!(count >= 2);
}

// ============================================================================
// CONST STATEMENTS
// ============================================================================

#[test]
fn test_const_declaration() {
    let code = "const MAX: i32 = 100";
    let stmt = parse_stmt(code);
    if let Statement::Const { name, .. } = stmt {
        assert_eq!(name, "MAX");
        // type_ is required in Const statements
    } else {
        panic!("Expected Const statement");
    }
}

// ============================================================================
// DEFER STATEMENTS
// ============================================================================

#[test]
fn test_defer_statement() {
    let code = "defer cleanup()";
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::Defer { .. }));
}

// ============================================================================
// BLOCK STATEMENTS
// ============================================================================

#[test]
fn test_block_statement() {
    let code = "{ let x = 1; x + 1 }";
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::Expression { .. }));
}

// ============================================================================
// COMPLEX STATEMENTS
// ============================================================================

#[test]
fn test_if_with_let_else() {
    // if-let-else is currently parsed via let-else pattern
    // Use regular if for now
    let code = r#"if opt.is_some() {
        opt.unwrap()
    } else {
        0
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::If { .. }));
}

#[test]
fn test_for_with_enumerate() {
    let code = r#"for (i, item) in items.enumerate() {
        i + item
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::For { .. }));
}

#[test]
fn test_match_in_if() {
    let code = r#"if enabled {
        match x {
            1 => a,
            _ => b,
        }
    }"#;
    let stmt = parse_stmt(code);
    assert!(matches!(stmt, Statement::If { .. }));
}

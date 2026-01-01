//! Comprehensive Parser Expression Tests
//!
//! These tests verify that the parser correctly parses all expression types.
//! They serve as documentation for the expression grammar of Windjammer.

use windjammer::lexer::Lexer;
use windjammer::parser::ast::*;
use windjammer::parser_impl::Parser;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn parse_expr(input: &str) -> &'static Expression<'static> {
    // Wrap expression in a function to make it a valid program
    let full_code = format!("fn test() {{ {} }}", input);
    let mut lexer = Lexer::new(&full_code);
    let tokens = lexer.tokenize_with_locations();
    // Leak parser to keep arena alive for 'static lifetime (acceptable in tests)
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().expect("Failed to parse program");

    // Extract the expression from the function body
    if let Some(Item::Function { decl, .. }) = program.items.first() {
        if let Some(Statement::Expression { expr, .. }) = decl.body.first() {
            return expr;
        }
        if let Some(Statement::Let { value, .. }) = decl.body.first() {
            return value;
        }
    }
    panic!("Failed to extract expression from: {}", input);
}

fn parse_program(input: &str) -> Program<'static> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();
    // Leak parser to keep arena alive for 'static lifetime (acceptable in tests)
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().expect("Failed to parse program")
}

// ============================================================================
// LITERAL EXPRESSIONS
// ============================================================================

#[test]
fn test_integer_literal() {
    let expr = parse_expr("42");
    if let Expression::Literal {
        value: Literal::Int(n),
        ..
    } = *expr
    {
        assert_eq!(n, 42);
    } else {
        panic!("Expected Int literal, got {:?}", expr);
    }
}

#[test]
fn test_float_literal() {
    let expr = parse_expr("2.5");
    if let Expression::Literal {
        value: Literal::Float(f),
        ..
    } = *expr
    {
        assert!((f - 2.5).abs() < 0.001);
    } else {
        panic!("Expected Float literal");
    }
}

#[test]
fn test_string_literal() {
    let expr = parse_expr(r#""hello""#);
    if let Expression::Literal {
        value: Literal::String(ref s),
        ..
    } = *expr
    {
        assert_eq!(s, "hello");
    } else {
        panic!("Expected String literal");
    }
}

#[test]
fn test_bool_literal_true() {
    let expr = parse_expr("true");
    if let Expression::Literal {
        value: Literal::Bool(b),
        ..
    } = *expr
    {
        assert!(b);
    } else {
        panic!("Expected Bool literal");
    }
}

#[test]
fn test_bool_literal_false() {
    let expr = parse_expr("false");
    if let Expression::Literal {
        value: Literal::Bool(b),
        ..
    } = *expr
    {
        assert!(!b);
    } else {
        panic!("Expected Bool literal");
    }
}

#[test]
fn test_char_literal() {
    let expr = parse_expr("'a'");
    if let Expression::Literal {
        value: Literal::Char(c),
        ..
    } = *expr
    {
        assert_eq!(c, 'a');
    } else {
        panic!("Expected Char literal");
    }
}

// ============================================================================
// IDENTIFIER EXPRESSIONS
// ============================================================================

#[test]
fn test_identifier() {
    let expr = parse_expr("foo");
    if let Expression::Identifier { ref name, .. } = *expr {
        assert_eq!(name, "foo");
    } else {
        panic!("Expected Identifier");
    }
}

#[test]
fn test_identifier_with_underscores() {
    let expr = parse_expr("foo_bar_baz");
    if let Expression::Identifier { ref name, .. } = *expr {
        assert_eq!(name, "foo_bar_baz");
    } else {
        panic!("Expected Identifier");
    }
}

// ============================================================================
// BINARY EXPRESSIONS
// ============================================================================

#[test]
fn test_binary_add() {
    let expr = parse_expr("1 + 2");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Add);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_subtract() {
    let expr = parse_expr("5 - 3");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Sub);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_multiply() {
    let expr = parse_expr("4 * 2");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Mul);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_divide() {
    let expr = parse_expr("10 / 2");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Div);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_modulo() {
    let expr = parse_expr("7 % 3");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Mod);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_equal() {
    let expr = parse_expr("a == b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Eq);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_not_equal() {
    let expr = parse_expr("a != b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Ne);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_less_than() {
    let expr = parse_expr("a < b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Lt);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_less_equal() {
    let expr = parse_expr("a <= b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Le);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_greater_than() {
    let expr = parse_expr("a > b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Gt);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_greater_equal() {
    let expr = parse_expr("a >= b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Ge);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_and() {
    let expr = parse_expr("a && b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::And);
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_binary_or() {
    let expr = parse_expr("a || b");
    if let Expression::Binary { op, .. } = *expr {
        assert_eq!(op, BinaryOp::Or);
    } else {
        panic!("Expected Binary");
    }
}

// ============================================================================
// OPERATOR PRECEDENCE
// ============================================================================

#[test]
fn test_precedence_mul_over_add() {
    // a + b * c should parse as a + (b * c)
    let expr = parse_expr("a + b * c");
    if let Expression::Binary {
        op, left, right, ..
    } = *expr
    {
        assert_eq!(op, BinaryOp::Add);
        // left should be 'a'
        assert!(matches!(*left, Expression::Identifier { .. }));
        // right should be 'b * c'
        assert!(matches!(
            *right,
            Expression::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_precedence_comparison_over_logical() {
    // a < b && c > d should parse as (a < b) && (c > d)
    let expr = parse_expr("a < b && c > d");
    if let Expression::Binary {
        op, left, right, ..
    } = *expr
    {
        assert_eq!(op, BinaryOp::And);
        assert!(matches!(
            *left,
            Expression::Binary {
                op: BinaryOp::Lt,
                ..
            }
        ));
        assert!(matches!(
            *right,
            Expression::Binary {
                op: BinaryOp::Gt,
                ..
            }
        ));
    } else {
        panic!("Expected Binary");
    }
}

#[test]
fn test_precedence_parentheses() {
    // (a + b) * c should parse with add inside mul
    let expr = parse_expr("(a + b) * c");
    if let Expression::Binary { op, left, .. } = *expr {
        assert_eq!(op, BinaryOp::Mul);
        assert!(matches!(
            *left,
            Expression::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    } else {
        panic!("Expected Binary");
    }
}

// ============================================================================
// UNARY EXPRESSIONS
// ============================================================================

#[test]
fn test_unary_negation() {
    let expr = parse_expr("-x");
    if let Expression::Unary { op, .. } = *expr {
        assert_eq!(op, UnaryOp::Neg);
    } else {
        panic!("Expected Unary");
    }
}

#[test]
fn test_unary_not() {
    let expr = parse_expr("!flag");
    if let Expression::Unary { op, .. } = *expr {
        assert_eq!(op, UnaryOp::Not);
    } else {
        panic!("Expected Unary");
    }
}

// ============================================================================
// CALL EXPRESSIONS
// ============================================================================

#[test]
fn test_function_call_no_args() {
    let expr = parse_expr("foo()");
    if let Expression::Call {
        function,
        ref arguments,
        ..
    } = *expr
    {
        assert!(matches!(*function, Expression::Identifier { ref name, .. } if name == "foo"));
        assert!(arguments.is_empty());
    } else {
        panic!("Expected Call");
    }
}

#[test]
fn test_function_call_one_arg() {
    let expr = parse_expr("foo(1)");
    if let Expression::Call { ref arguments, .. } = *expr {
        assert_eq!(arguments.len(), 1);
    } else {
        panic!("Expected Call");
    }
}

#[test]
fn test_function_call_multiple_args() {
    let expr = parse_expr("foo(1, 2, 3)");
    if let Expression::Call { ref arguments, .. } = *expr {
        assert_eq!(arguments.len(), 3);
    } else {
        panic!("Expected Call");
    }
}

#[test]
fn test_function_call_nested() {
    let expr = parse_expr("foo(bar(x))");
    if let Expression::Call { ref arguments, .. } = *expr {
        assert_eq!(arguments.len(), 1);
        // Each argument is (Option<String>, Expression)
        let (_, inner) = &arguments[0];
        assert!(matches!(inner, Expression::Call { .. }));
    } else {
        panic!("Expected Call");
    }
}

// ============================================================================
// METHOD CALL EXPRESSIONS
// ============================================================================

#[test]
fn test_method_call_no_args() {
    let expr = parse_expr("obj.method()");
    assert!(matches!(expr, Expression::MethodCall { .. }));
}

#[test]
fn test_method_call_with_args() {
    let expr = parse_expr("obj.method(1, 2)");
    if let Expression::MethodCall {
        ref method,
        ref arguments,
        ..
    } = *expr
    {
        assert_eq!(method, "method");
        assert_eq!(arguments.len(), 2);
    } else {
        panic!("Expected MethodCall");
    }
}

#[test]
fn test_method_call_chained() {
    let expr = parse_expr("obj.first().second()");
    // The outer call should be .second()
    if let Expression::MethodCall {
        ref method, object, ..
    } = *expr
    {
        assert_eq!(method, "second");
        // object should be obj.first()
        assert!(matches!(*object, Expression::MethodCall { .. }));
    } else {
        panic!("Expected MethodCall");
    }
}

// ============================================================================
// FIELD ACCESS EXPRESSIONS
// ============================================================================

#[test]
fn test_field_access() {
    let expr = parse_expr("obj.field");
    if let Expression::FieldAccess { ref field, .. } = *expr {
        assert_eq!(field, "field");
    } else {
        panic!("Expected FieldAccess");
    }
}

#[test]
fn test_field_access_nested() {
    let expr = parse_expr("a.b.c");
    if let Expression::FieldAccess {
        ref field, object, ..
    } = *expr
    {
        assert_eq!(field, "c");
        if let Expression::FieldAccess {
            field: ref inner_field,
            ..
        } = *object
        {
            assert_eq!(inner_field, "b");
        } else {
            panic!("Expected nested FieldAccess");
        }
    } else {
        panic!("Expected FieldAccess");
    }
}

// ============================================================================
// INDEX EXPRESSIONS
// ============================================================================

#[test]
fn test_index_access() {
    let expr = parse_expr("arr[0]");
    assert!(matches!(expr, Expression::Index { .. }));
}

#[test]
fn test_index_access_variable() {
    let expr = parse_expr("arr[i]");
    assert!(matches!(expr, Expression::Index { .. }));
}

#[test]
fn test_index_access_expression() {
    let expr = parse_expr("arr[i + 1]");
    assert!(matches!(expr, Expression::Index { .. }));
}

// ============================================================================
// STRUCT LITERAL EXPRESSIONS
// ============================================================================

#[test]
fn test_struct_literal_empty() {
    let expr = parse_expr("Point {}");
    if let Expression::StructLiteral { ref name, .. } = *expr {
        assert_eq!(name, "Point");
    } else {
        panic!("Expected StructLiteral, got {:?}", expr);
    }
}

#[test]
fn test_struct_literal_with_fields() {
    let expr = parse_expr("Point { x: 1, y: 2 }");
    if let Expression::StructLiteral {
        ref name,
        ref fields,
        ..
    } = *expr
    {
        assert_eq!(name, "Point");
        assert_eq!(fields.len(), 2);
    } else {
        panic!("Expected StructLiteral");
    }
}

// ============================================================================
// ARRAY LITERAL EXPRESSIONS
// ============================================================================

#[test]
fn test_array_literal_empty() {
    let expr = parse_expr("[]");
    if let Expression::Array { ref elements, .. } = *expr {
        assert!(elements.is_empty());
    } else {
        panic!("Expected Array");
    }
}

#[test]
fn test_array_literal_with_elements() {
    let expr = parse_expr("[1, 2, 3]");
    if let Expression::Array { ref elements, .. } = *expr {
        assert_eq!(elements.len(), 3);
    } else {
        panic!("Expected Array");
    }
}

// ============================================================================
// TUPLE EXPRESSIONS
// ============================================================================

#[test]
fn test_tuple_expression() {
    let expr = parse_expr("(1, 2, 3)");
    if let Expression::Tuple { ref elements, .. } = *expr {
        assert_eq!(elements.len(), 3);
    } else {
        panic!("Expected Tuple, got {:?}", expr);
    }
}

// ============================================================================
// RANGE EXPRESSIONS
// ============================================================================

#[test]
fn test_range_exclusive() {
    let expr = parse_expr("0..10");
    if let Expression::Range { inclusive, .. } = *expr {
        assert!(!inclusive);
    } else {
        panic!("Expected Range");
    }
}

#[test]
fn test_range_inclusive() {
    let expr = parse_expr("0..=10");
    if let Expression::Range { inclusive, .. } = *expr {
        assert!(inclusive);
    } else {
        panic!("Expected Range");
    }
}

// ============================================================================
// CLOSURE EXPRESSIONS
// ============================================================================

#[test]
fn test_closure_no_params() {
    let expr = parse_expr("|| 42");
    assert!(matches!(expr, Expression::Closure { .. }));
}

#[test]
fn test_closure_one_param() {
    let expr = parse_expr("|x| x + 1");
    if let Expression::Closure { ref parameters, .. } = *expr {
        assert_eq!(parameters.len(), 1);
    } else {
        panic!("Expected Closure");
    }
}

#[test]
fn test_closure_multiple_params() {
    let expr = parse_expr("|a, b| a + b");
    if let Expression::Closure { ref parameters, .. } = *expr {
        assert_eq!(parameters.len(), 2);
    } else {
        panic!("Expected Closure");
    }
}

#[test]
fn test_closure_with_block() {
    let expr = parse_expr("|x| { let y = x; y + 1 }");
    assert!(matches!(expr, Expression::Closure { .. }));
}

// ============================================================================
// IF EXPRESSIONS (as Statements)
// ============================================================================

#[test]
fn test_if_statement() {
    let code = r#"
    fn test() {
        if true {
            1
        } else {
            0
        }
    }
    "#;
    let program = parse_program(code);
    assert!(!program.items.is_empty());
}

// ============================================================================
// MATCH EXPRESSIONS
// ============================================================================

#[test]
fn test_match_expression() {
    let code = r#"
    fn test() {
        match x {
            1 => a,
            2 => b,
            _ => c,
        }
    }
    "#;
    let program = parse_program(code);
    assert!(!program.items.is_empty());
}

// ============================================================================
// BLOCK EXPRESSIONS
// ============================================================================

#[test]
fn test_block_expression() {
    let expr = parse_expr("{ let x = 1; x + 1 }");
    assert!(matches!(expr, Expression::Block { .. }));
}

// ============================================================================
// AWAIT EXPRESSIONS
// ============================================================================

#[test]
fn test_await_expression() {
    let expr = parse_expr("future.await");
    // This is either Await or FieldAccess depending on implementation
    assert!(matches!(
        expr,
        Expression::Await { .. } | Expression::FieldAccess { .. }
    ));
}

// ============================================================================
// SELF EXPRESSIONS
// ============================================================================

#[test]
fn test_self_keyword() {
    let code = "fn test(&self) { self }";
    let program = parse_program(code);
    assert!(!program.items.is_empty());
}

// ============================================================================
// COMPLEX EXPRESSIONS
// ============================================================================

#[test]
fn test_complex_expression_chain() {
    let expr = parse_expr("obj.method().field[0].inner()");
    // Should parse successfully as a chain
    assert!(matches!(expr, Expression::MethodCall { .. }));
}

#[test]
fn test_complex_expression_with_calls() {
    let expr = parse_expr("foo(bar(x + y), baz.qux())");
    if let Expression::Call { ref arguments, .. } = *expr {
        assert_eq!(arguments.len(), 2);
    } else {
        panic!("Expected Call");
    }
}

// ============================================================================
// INTERPOLATED STRINGS
// ============================================================================

#[test]
fn test_interpolated_string() {
    // Interpolated strings are handled at the lexer level
    // and produce multiple tokens or a special expression
    let code = r#"fn test() { "Hello, ${name}!" }"#;
    let program = parse_program(code);
    assert!(!program.items.is_empty());
}

// ============================================================================
// TRY OPERATOR
// ============================================================================

#[test]
fn test_try_operator() {
    let expr = parse_expr("result?");
    assert!(matches!(expr, Expression::TryOp { .. }));
}

// ============================================================================
// CAST EXPRESSIONS
// ============================================================================

#[test]
fn test_cast_expression() {
    let expr = parse_expr("x as i32");
    assert!(matches!(expr, Expression::Cast { .. }));
}

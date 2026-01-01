// TDD Tests for AST Utility Functions
// Tests written FIRST before extraction!
//
// UPDATED: Now using AST builder functions!

use windjammer::codegen::rust::ast_utilities::*;
use windjammer::parser::ast::builders::*;
use windjammer::parser::{Expression, Pattern, Statement};
use windjammer::test_utils::*;

// Arena-allocating wrappers
fn alloc_int(n: i64) -> &'static Expression<'static> {
    test_alloc_expr(expr_int(n))
}

fn alloc_var(name: impl Into<String>) -> &'static Expression<'static> {
    test_alloc_expr(expr_var(name))
}

fn alloc_stmt_let(
    name: &str,
    type_: Option<windjammer::parser::Type>,
    value: &'static Expression<'static>,
) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_let(name, type_, value))
}

fn alloc_stmt_return(value: Option<&'static Expression<'static>>) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_return(value))
}

fn alloc_stmt_if(
    cond: &'static Expression<'static>,
    then_block: Vec<&'static Statement<'static>>,
    else_block: Option<Vec<&'static Statement<'static>>>,
) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_if(cond, then_block, else_block))
}

fn alloc_stmt_expr(expr: &'static Expression<'static>) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_expr(expr))
}

fn alloc_field(
    object: &'static Expression<'static>,
    field: impl Into<String>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_field(object, field))
}

fn alloc_stmt_match(
    expr: &'static Expression<'static>,
    arms: Vec<windjammer::parser::MatchArm<'static>>,
) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_match(expr, arms))
}

fn alloc_stmt_assign(
    target: &'static Expression<'static>,
    value: &'static Expression<'static>,
) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_assign(target, value))
}

fn alloc_stmt_while(
    cond: &'static Expression<'static>,
    body: Vec<&'static Statement<'static>>,
) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_while(cond, body))
}

fn alloc_stmt_loop(body: Vec<&'static Statement<'static>>) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_loop(body))
}

fn alloc_stmt_for(
    pattern: &'static windjammer::parser::Pattern<'static>,
    iterable: &'static Expression<'static>,
    body: Vec<&'static Statement<'static>>,
) -> &'static Statement<'static> {
    test_alloc_stmt(stmt_for(pattern.clone(), iterable, body))
}

fn alloc_bool(b: bool) -> &'static Expression<'static> {
    test_alloc_expr(expr_bool(b))
}

fn alloc_add(
    left: &'static Expression<'static>,
    right: &'static Expression<'static>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_add(left, right))
}

// ============================================================================
// count_statements TESTS
// ============================================================================

#[test]
fn test_count_statements_empty() {
    let statements: Vec<&Statement<'static>> = vec![];
    assert_eq!(count_statements(&statements), 0);
}

#[test]
fn test_count_statements_simple_let() {
    let statements = vec![alloc_stmt_let("x", None, alloc_int(42))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_return() {
    let statements = vec![alloc_stmt_return(Some(alloc_var("x")))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_expression() {
    let statements = vec![alloc_stmt_expr(alloc_int(42))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_if_weighted() {
    // If statements should count as 3 (weighted more heavily)
    let statements = vec![alloc_stmt_if(alloc_bool(true), vec![], None)];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_while_weighted() {
    let statements = vec![alloc_stmt_while(alloc_bool(true), vec![])];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_loop_weighted() {
    let statements = vec![alloc_stmt_loop(vec![])];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_for_weighted() {
    let statements = vec![alloc_stmt_for(
        test_alloc_pattern(Pattern::Identifier("i".to_string())),
        alloc_var("items"),
        vec![],
    )];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_match_weighted() {
    // Match statements should count as 5 (most complex)
    let statements = vec![alloc_stmt_match(alloc_var("x"), vec![])];
    assert_eq!(count_statements(&statements), 5);
}

#[test]
fn test_count_statements_assignment() {
    let statements = vec![alloc_stmt_assign(alloc_var("x"), alloc_int(10))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_mixed() {
    let statements: Vec<&'static Statement<'static>> = vec![
        test_alloc_stmt(Statement::Let {
            pattern: Pattern::Identifier("x".to_string()),
            mutable: false,
            type_: None,
            value: test_alloc_expr(Expression::Literal {
                value: windjammer::parser::Literal::Int(1),
                location: None,
            }),
            else_block: None,
            location: None,
        }),
        test_alloc_stmt(Statement::If {
            condition: test_alloc_expr(Expression::Literal {
                value: windjammer::parser::Literal::Bool(true),
                location: None,
            }),
            then_block: vec![],
            else_block: None,
            location: None,
        }),
        test_alloc_stmt(Statement::Return {
            value: Some(test_alloc_expr(Expression::Identifier {
                name: "x".to_string(),
                location: None,
            })),
            location: None,
        }),
    ];
    // 1 (let) + 3 (if) + 1 (return) = 5
    assert_eq!(count_statements(&statements), 5);
}

// ============================================================================
// extract_function_name TESTS
// ============================================================================

#[test]
fn test_extract_function_name_identifier() {
    let expr = alloc_var("my_function");
    assert_eq!(extract_function_name(expr), "my_function");
}

#[test]
fn test_extract_function_name_field_access() {
    let expr = alloc_field(alloc_var("module"), "function");
    assert_eq!(extract_function_name(expr), "function");
}

#[test]
fn test_extract_function_name_literal_returns_empty() {
    let expr = alloc_int(42);
    assert_eq!(extract_function_name(expr), "");
}

#[test]
fn test_extract_function_name_binary_returns_empty() {
    let expr = alloc_add(alloc_var("a"), alloc_var("b"));
    assert_eq!(extract_function_name(expr), "");
}

// ============================================================================
// extract_field_access_path TESTS
// ============================================================================

#[test]
fn test_extract_field_access_path_identifier() {
    let expr = expr_var("variable");
    assert_eq!(
        extract_field_access_path(&expr),
        Some("variable".to_string())
    );
}

#[test]
fn test_extract_field_access_path_simple_field() {
    let expr = alloc_field(alloc_var("config"), "name");
    assert_eq!(
        extract_field_access_path(expr),
        Some("config.name".to_string())
    );
}

#[test]
fn test_extract_field_access_path_nested() {
    let expr = test_alloc_expr(Expression::FieldAccess {
        object: test_alloc_expr(Expression::FieldAccess {
            object: test_alloc_expr(Expression::Identifier {
                name: "app".to_string(),
                location: None,
            }),
            field: "config".to_string(),
            location: None,
        }),
        field: "paths".to_string(),
        location: None,
    });
    assert_eq!(
        extract_field_access_path(expr),
        Some("app.config.paths".to_string())
    );
}

#[test]
fn test_extract_field_access_path_method_call() {
    let expr = test_alloc_expr(Expression::MethodCall {
        object: test_alloc_expr(Expression::Identifier {
            name: "source".to_string(),
            location: None,
        }),
        method: "get_items".to_string(),
        arguments: vec![],
        type_args: None,
        location: None,
    });
    assert_eq!(
        extract_field_access_path(expr),
        Some("source.get_items()".to_string())
    );
}

#[test]
fn test_extract_field_access_path_index() {
    let expr = test_alloc_expr(Expression::Index {
        object: test_alloc_expr(Expression::Identifier {
            name: "items".to_string(),
            location: None,
        }),
        index: test_alloc_expr(Expression::Literal {
            value: windjammer::parser::Literal::Int(0),
            location: None,
        }),
        location: None,
    });
    assert_eq!(
        extract_field_access_path(expr),
        Some("items[Int(0)]".to_string())
    );
}

#[test]
fn test_extract_field_access_path_complex_method_on_field() {
    let expr = test_alloc_expr(Expression::MethodCall {
        object: test_alloc_expr(Expression::FieldAccess {
            object: test_alloc_expr(Expression::Identifier {
                name: "config".to_string(),
                location: None,
            }),
            field: "items".to_string(),
            location: None,
        }),
        method: "len".to_string(),
        arguments: vec![],
        type_args: None,
        location: None,
    });
    assert_eq!(
        extract_field_access_path(expr),
        Some("config.items.len()".to_string())
    );
}

#[test]
fn test_extract_field_access_path_literal_returns_none() {
    let expr = test_alloc_expr(Expression::Literal {
        value: windjammer::parser::Literal::Int(42),
        location: None,
    });
    assert_eq!(extract_field_access_path(expr), None);
}

#[test]
fn test_extract_field_access_path_binary_returns_none() {
    let expr = test_alloc_expr(Expression::Binary {
        left: test_alloc_expr(Expression::Identifier {
            name: "a".to_string(),
            location: None,
        }),
        op: windjammer::parser::BinaryOp::Add,
        right: test_alloc_expr(Expression::Identifier {
            name: "b".to_string(),
            location: None,
        }),
        location: None,
    });
    assert_eq!(extract_field_access_path(expr), None);
}

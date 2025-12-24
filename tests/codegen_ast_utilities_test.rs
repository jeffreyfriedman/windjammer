// TDD Tests for AST Utility Functions
// Tests written FIRST before extraction!
//
// UPDATED: Now using AST builder functions!

use windjammer::codegen::rust::ast_utilities::*;
use windjammer::parser::ast::builders::*;
use windjammer::parser::{Expression, Pattern, Statement};

// ============================================================================
// count_statements TESTS
// ============================================================================

#[test]
fn test_count_statements_empty() {
    let statements: Vec<Statement> = vec![];
    assert_eq!(count_statements(&statements), 0);
}

#[test]
fn test_count_statements_simple_let() {
    let statements = vec![stmt_let("x", None, expr_int(42))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_return() {
    let statements = vec![stmt_return(Some(expr_var("x")))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_expression() {
    let statements = vec![stmt_expr(expr_int(42))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_if_weighted() {
    // If statements should count as 3 (weighted more heavily)
    let statements = vec![stmt_if(expr_bool(true), vec![], None)];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_while_weighted() {
    let statements = vec![stmt_while(expr_bool(true), vec![])];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_loop_weighted() {
    let statements = vec![stmt_loop(vec![])];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_for_weighted() {
    let statements = vec![stmt_for(
        Pattern::Identifier("i".to_string()),
        expr_var("items"),
        vec![],
    )];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_match_weighted() {
    // Match statements should count as 5 (most complex)
    let statements = vec![stmt_match(expr_var("x"), vec![])];
    assert_eq!(count_statements(&statements), 5);
}

#[test]
fn test_count_statements_assignment() {
    let statements = vec![stmt_assign(expr_var("x"), expr_int(10))];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_mixed() {
    let statements = vec![
        Statement::Let {
            pattern: Pattern::Identifier("x".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Literal {
                value: windjammer::parser::Literal::Int(1),
                location: None,
            },
            else_block: None,
            location: None,
        },
        Statement::If {
            condition: Expression::Literal {
                value: windjammer::parser::Literal::Bool(true),
                location: None,
            },
            then_block: vec![],
            else_block: None,
            location: None,
        },
        Statement::Return {
            value: Some(Expression::Identifier {
                name: "x".to_string(),
                location: None,
            }),
            location: None,
        },
    ];
    // 1 (let) + 3 (if) + 1 (return) = 5
    assert_eq!(count_statements(&statements), 5);
}

// ============================================================================
// extract_function_name TESTS
// ============================================================================

#[test]
fn test_extract_function_name_identifier() {
    let expr = expr_var("my_function");
    assert_eq!(extract_function_name(&expr), "my_function");
}

#[test]
fn test_extract_function_name_field_access() {
    let expr = expr_field(expr_var("module"), "function");
    assert_eq!(extract_function_name(&expr), "function");
}

#[test]
fn test_extract_function_name_literal_returns_empty() {
    let expr = expr_int(42);
    assert_eq!(extract_function_name(&expr), "");
}

#[test]
fn test_extract_function_name_binary_returns_empty() {
    let expr = expr_add(expr_var("a"), expr_var("b"));
    assert_eq!(extract_function_name(&expr), "");
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
    let expr = expr_field(expr_var("config"), "name");
    assert_eq!(
        extract_field_access_path(&expr),
        Some("config.name".to_string())
    );
}

#[test]
fn test_extract_field_access_path_nested() {
    let expr = Expression::FieldAccess {
        object: Box::new(Expression::FieldAccess {
            object: Box::new(Expression::Identifier {
                name: "app".to_string(),
                location: None,
            }),
            field: "config".to_string(),
            location: None,
        }),
        field: "paths".to_string(),
        location: None,
    };
    assert_eq!(
        extract_field_access_path(&expr),
        Some("app.config.paths".to_string())
    );
}

#[test]
fn test_extract_field_access_path_method_call() {
    let expr = Expression::MethodCall {
        object: Box::new(Expression::Identifier {
            name: "source".to_string(),
            location: None,
        }),
        method: "get_items".to_string(),
        arguments: vec![],
        type_args: None,
        location: None,
    };
    assert_eq!(
        extract_field_access_path(&expr),
        Some("source.get_items()".to_string())
    );
}

#[test]
fn test_extract_field_access_path_index() {
    let expr = Expression::Index {
        object: Box::new(Expression::Identifier {
            name: "items".to_string(),
            location: None,
        }),
        index: Box::new(Expression::Literal {
            value: windjammer::parser::Literal::Int(0),
            location: None,
        }),
        location: None,
    };
    assert_eq!(
        extract_field_access_path(&expr),
        Some("items[Int(0)]".to_string())
    );
}

#[test]
fn test_extract_field_access_path_complex_method_on_field() {
    let expr = Expression::MethodCall {
        object: Box::new(Expression::FieldAccess {
            object: Box::new(Expression::Identifier {
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
    };
    assert_eq!(
        extract_field_access_path(&expr),
        Some("config.items.len()".to_string())
    );
}

#[test]
fn test_extract_field_access_path_literal_returns_none() {
    let expr = Expression::Literal {
        value: windjammer::parser::Literal::Int(42),
        location: None,
    };
    assert_eq!(extract_field_access_path(&expr), None);
}

#[test]
fn test_extract_field_access_path_binary_returns_none() {
    let expr = Expression::Binary {
        left: Box::new(Expression::Identifier {
            name: "a".to_string(),
            location: None,
        }),
        op: windjammer::parser::BinaryOp::Add,
        right: Box::new(Expression::Identifier {
            name: "b".to_string(),
            location: None,
        }),
        location: None,
    };
    assert_eq!(extract_field_access_path(&expr), None);
}

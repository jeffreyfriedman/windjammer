// TDD Tests for AST Utility Functions
// Tests written FIRST before extraction!

use windjammer::codegen::rust::ast_utilities::*;
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
    let statements = vec![Statement::Let {
        pattern: Pattern::Identifier("x".to_string()),
        mutable: false,
        type_: None,
        value: Expression::Literal {
            value: windjammer::parser::Literal::Int(42),
            location: None,
        },
        else_block: None,
        location: None,
    }];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_return() {
    let statements = vec![Statement::Return {
        value: Some(Expression::Identifier {
            name: "x".to_string(),
            location: None,
        }),
        location: None,
    }];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_expression() {
    let statements = vec![Statement::Expression {
        expr: Expression::Literal {
            value: windjammer::parser::Literal::Int(42),
            location: None,
        },
        location: None,
    }];
    assert_eq!(count_statements(&statements), 1);
}

#[test]
fn test_count_statements_if_weighted() {
    // If statements should count as 3 (weighted more heavily)
    let statements = vec![Statement::If {
        condition: Expression::Literal {
            value: windjammer::parser::Literal::Bool(true),
            location: None,
        },
        then_block: vec![],
        else_block: None,
        location: None,
    }];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_while_weighted() {
    let statements = vec![Statement::While {
        condition: Expression::Literal {
            value: windjammer::parser::Literal::Bool(true),
            location: None,
        },
        body: vec![],
        location: None,
    }];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_loop_weighted() {
    let statements = vec![Statement::Loop {
        body: vec![],
        location: None,
    }];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_for_weighted() {
    let statements = vec![Statement::For {
        pattern: Pattern::Identifier("i".to_string()),
        iterable: Expression::Identifier {
            name: "items".to_string(),
            location: None,
        },
        body: vec![],
        location: None,
    }];
    assert_eq!(count_statements(&statements), 3);
}

#[test]
fn test_count_statements_match_weighted() {
    // Match statements should count as 5 (most complex)
    let statements = vec![Statement::Match {
        value: Expression::Identifier {
            name: "x".to_string(),
            location: None,
        },
        arms: vec![],
        location: None,
    }];
    assert_eq!(count_statements(&statements), 5);
}

#[test]
fn test_count_statements_assignment() {
    let statements = vec![Statement::Assignment {
        target: Expression::Identifier {
            name: "x".to_string(),
            location: None,
        },
        value: Expression::Literal {
            value: windjammer::parser::Literal::Int(10),
            location: None,
        },
        compound_op: None,
        location: None,
    }];
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
    let expr = Expression::Identifier {
        name: "my_function".to_string(),
        location: None,
    };
    assert_eq!(extract_function_name(&expr), "my_function");
}

#[test]
fn test_extract_function_name_field_access() {
    let expr = Expression::FieldAccess {
        object: Box::new(Expression::Identifier {
            name: "module".to_string(),
            location: None,
        }),
        field: "function".to_string(),
        location: None,
    };
    assert_eq!(extract_function_name(&expr), "function");
}

#[test]
fn test_extract_function_name_literal_returns_empty() {
    let expr = Expression::Literal {
        value: windjammer::parser::Literal::Int(42),
        location: None,
    };
    assert_eq!(extract_function_name(&expr), "");
}

#[test]
fn test_extract_function_name_binary_returns_empty() {
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
    assert_eq!(extract_function_name(&expr), "");
}

// ============================================================================
// extract_field_access_path TESTS
// ============================================================================

#[test]
fn test_extract_field_access_path_identifier() {
    let expr = Expression::Identifier {
        name: "variable".to_string(),
        location: None,
    };
    assert_eq!(
        extract_field_access_path(&expr),
        Some("variable".to_string())
    );
}

#[test]
fn test_extract_field_access_path_simple_field() {
    let expr = Expression::FieldAccess {
        object: Box::new(Expression::Identifier {
            name: "config".to_string(),
            location: None,
        }),
        field: "name".to_string(),
        location: None,
    };
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


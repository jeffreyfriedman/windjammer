// TDD Tests for Codegen Helper Functions
// Tests written FIRST before extraction!

use windjammer::codegen::rust::codegen_helpers::*;
use windjammer::parser::{Expression, Item, Pattern, Statement, Type};

// ============================================================================
// get_expression_location TESTS
// ============================================================================

#[test]
fn test_get_expression_location_with_location() {
    let location = Some(windjammer::source_map::Location {
        file: "test.wj".to_string().into(),
        line: 42,
        column: 10,
    });

    let expr = Expression::Identifier {
        name: "foo".to_string(),
        location: location.clone(),
    };

    assert_eq!(get_expression_location(&expr), location);
}

#[test]
fn test_get_expression_location_none() {
    let expr = Expression::Identifier {
        name: "foo".to_string(),
        location: None,
    };

    assert_eq!(get_expression_location(&expr), None);
}

#[test]
fn test_get_expression_location_literal() {
    let location = Some(windjammer::source_map::Location {
        file: "main.wj".to_string().into(),
        line: 1,
        column: 5,
    });

    let expr = Expression::Literal {
        value: windjammer::parser::Literal::Int(123),
        location: location.clone(),
    };

    assert_eq!(get_expression_location(&expr), location);
}

// ============================================================================
// get_statement_location TESTS
// ============================================================================

#[test]
fn test_get_statement_location_with_location() {
    let location = Some(windjammer::source_map::Location {
        file: "test.wj".to_string().into(),
        line: 10,
        column: 4,
    });

    let stmt = Statement::Return {
        value: None,
        location: location.clone(),
    };

    assert_eq!(get_statement_location(&stmt), location);
}

#[test]
fn test_get_statement_location_none() {
    let stmt = Statement::Return {
        value: None,
        location: None,
    };

    assert_eq!(get_statement_location(&stmt), None);
}

#[test]
fn test_get_statement_location_let() {
    let location = Some(windjammer::source_map::Location {
        file: "module.wj".to_string().into(),
        line: 25,
        column: 8,
    });

    let stmt = Statement::Let {
        pattern: Pattern::Identifier("x".to_string()),
        mutable: false,
        type_: None,
        value: windjammer::test_utils::test_alloc_expr(Expression::Literal {
            value: windjammer::parser::Literal::Int(0),
            location: None,
        }),
        else_block: None,
        location: location.clone(),
    };

    assert_eq!(get_statement_location(&stmt), location);
}

// ============================================================================
// get_item_location TESTS
// ============================================================================

#[test]
fn test_get_item_location_function() {
    let location = Some(windjammer::source_map::Location {
        file: "lib.wj".to_string().into(),
        line: 50,
        column: 1,
    });

    let item = Item::Function {
        decl: windjammer::parser::FunctionDecl {
            name: "test".to_string(),
            parameters: vec![],
            return_type: Some(Type::Int),
            body: vec![],
            type_params: vec![],
            decorators: vec![],
            where_clause: vec![],
            is_pub: false,
            is_async: false,
            is_extern: false,
            parent_type: None,
            doc_comment: None,
        },
        location: location.clone(),
    };

    assert_eq!(get_item_location(&item), location);
}

#[test]
fn test_get_item_location_struct() {
    let location = Some(windjammer::source_map::Location {
        file: "types.wj".to_string().into(),
        line: 100,
        column: 1,
    });

    let item = Item::Struct {
        decl: windjammer::parser::StructDecl {
            name: "Point".to_string(),
            fields: vec![],
            type_params: vec![],
            decorators: vec![],
            where_clause: vec![],
            is_pub: false,
            doc_comment: None,
        },
        location: location.clone(),
    };

    assert_eq!(get_item_location(&item), location);
}

#[test]
fn test_get_item_location_none() {
    let item = Item::Function {
        decl: windjammer::parser::FunctionDecl {
            name: "test".to_string(),
            parameters: vec![],
            return_type: Some(Type::Int),
            body: vec![],
            type_params: vec![],
            decorators: vec![],
            where_clause: vec![],
            is_pub: false,
            is_async: false,
            is_extern: false,
            parent_type: None,
            doc_comment: None,
        },
        location: None,
    };

    assert_eq!(get_item_location(&item), None);
}

// ============================================================================
// format_where_clause TESTS
// ============================================================================

#[test]
fn test_format_where_clause_empty() {
    let where_clause: Vec<(String, Vec<String>)> = vec![];
    assert_eq!(format_where_clause(&where_clause), "");
}

#[test]
fn test_format_where_clause_single_bound() {
    let where_clause = vec![("T".to_string(), vec!["Display".to_string()])];
    assert_eq!(
        format_where_clause(&where_clause),
        "\nwhere\n    T: Display"
    );
}

#[test]
fn test_format_where_clause_multiple_bounds_single_param() {
    let where_clause = vec![(
        "T".to_string(),
        vec![
            "Display".to_string(),
            "Clone".to_string(),
            "Debug".to_string(),
        ],
    )];
    assert_eq!(
        format_where_clause(&where_clause),
        "\nwhere\n    T: Display + Clone + Debug"
    );
}

#[test]
fn test_format_where_clause_multiple_params() {
    let where_clause = vec![
        ("T".to_string(), vec!["Display".to_string()]),
        (
            "U".to_string(),
            vec!["Debug".to_string(), "Clone".to_string()],
        ),
    ];
    assert_eq!(
        format_where_clause(&where_clause),
        "\nwhere\n    T: Display,\n    U: Debug + Clone"
    );
}

#[test]
fn test_format_where_clause_complex() {
    let where_clause = vec![
        (
            "T".to_string(),
            vec!["Display".to_string(), "PartialEq".to_string()],
        ),
        ("U".to_string(), vec!["Debug".to_string()]),
        (
            "V".to_string(),
            vec![
                "Clone".to_string(),
                "Default".to_string(),
                "Send".to_string(),
            ],
        ),
    ];
    assert_eq!(
        format_where_clause(&where_clause),
        "\nwhere\n    T: Display + PartialEq,\n    U: Debug,\n    V: Clone + Default + Send"
    );
}

#[test]
fn test_format_where_clause_formatting() {
    // Test that indentation is correct (4 spaces before each clause)
    let where_clause = vec![("T".to_string(), vec!["Copy".to_string()])];
    let result = format_where_clause(&where_clause);

    assert!(result.starts_with("\nwhere\n"));
    assert!(result.contains("    T: Copy"));
}

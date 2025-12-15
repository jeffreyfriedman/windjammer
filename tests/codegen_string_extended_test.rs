// TDD Tests for extended string expression analysis functions (Phase 6)

use std::path::PathBuf;
use windjammer::codegen::rust::string_analysis::{
    block_has_as_str, expression_has_as_str, expression_produces_string, statement_has_as_str,
};
use windjammer::parser::{Expression, Literal, MacroDelimiter, Pattern, Statement};
use windjammer::source_map::Location;

fn test_loc() -> Location {
    Location {
        file: PathBuf::from(""),
        line: 0,
        column: 0,
    }
}

#[cfg(test)]
mod expression_produces_string_tests {
    use super::*;

    #[test]
    fn test_format_macro() {
        let expr = Expression::MacroInvocation {
            name: "format".to_string(),
            args: vec![],
            delimiter: MacroDelimiter::Parens,
            location: Some(test_loc()),
        };
        assert!(expression_produces_string(&expr));
    }

    #[test]
    fn test_to_string_method() {
        let expr = Expression::MethodCall {
            object: Box::new(Expression::Identifier {
                name: "obj".to_string(),
                location: Some(test_loc()),
            }),
            method: "to_string".to_string(),
            type_args: None,
            arguments: vec![],
            location: Some(test_loc()),
        };
        assert!(expression_produces_string(&expr));
    }

    #[test]
    fn test_len_method_not_string() {
        let expr = Expression::MethodCall {
            object: Box::new(Expression::Identifier {
                name: "arr".to_string(),
                location: Some(test_loc()),
            }),
            method: "len".to_string(),
            type_args: None,
            arguments: vec![],
            location: Some(test_loc()),
        };
        assert!(!expression_produces_string(&expr));
    }
}

#[cfg(test)]
mod as_str_detection_tests {
    use super::*;

    #[test]
    fn test_expression_has_as_str() {
        let expr = Expression::MethodCall {
            object: Box::new(Expression::Identifier {
                name: "s".to_string(),
                location: Some(test_loc()),
            }),
            method: "as_str".to_string(),
            type_args: None,
            arguments: vec![],
            location: Some(test_loc()),
        };
        assert!(expression_has_as_str(&expr));
    }

    #[test]
    fn test_expression_no_as_str() {
        let expr = Expression::MethodCall {
            object: Box::new(Expression::Identifier {
                name: "s".to_string(),
                location: Some(test_loc()),
            }),
            method: "to_string".to_string(),
            type_args: None,
            arguments: vec![],
            location: Some(test_loc()),
        };
        assert!(!expression_has_as_str(&expr));
    }

    #[test]
    fn test_statement_expression_has_as_str() {
        let stmt = Statement::Expression {
            expr: Expression::MethodCall {
                object: Box::new(Expression::Identifier {
                    name: "s".to_string(),
                    location: Some(test_loc()),
                }),
                method: "as_str".to_string(),
                type_args: None,
                arguments: vec![],
                location: Some(test_loc()),
            },
            location: Some(test_loc()),
        };
        assert!(statement_has_as_str(&stmt));
    }

    #[test]
    fn test_statement_no_as_str() {
        let stmt = Statement::Let {
            pattern: Pattern::Identifier("x".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Literal {
                value: Literal::Int(5),
                location: Some(test_loc()),
            },
            else_block: None,
            location: Some(test_loc()),
        };
        assert!(!statement_has_as_str(&stmt));
    }

    #[test]
    fn test_block_has_as_str() {
        let block = vec![Statement::Expression {
            expr: Expression::MethodCall {
                object: Box::new(Expression::Identifier {
                    name: "s".to_string(),
                    location: Some(test_loc()),
                }),
                method: "as_str".to_string(),
                type_args: None,
                arguments: vec![],
                location: Some(test_loc()),
            },
            location: Some(test_loc()),
        }];
        assert!(block_has_as_str(&block));
    }

    #[test]
    fn test_empty_block_no_as_str() {
        let block = vec![];
        assert!(!block_has_as_str(&block));
    }
}

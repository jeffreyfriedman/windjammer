// TDD Tests for extended string expression analysis functions (Phase 6)
//
// UPDATED: Now using AST builder functions!

use windjammer::codegen::rust::string_analysis::{
    block_has_as_str, block_has_explicit_ref, expression_has_as_str, expression_is_explicit_ref,
    expression_produces_string, statement_has_as_str,
};
use windjammer::parser::ast::builders::*;
use windjammer::parser::Pattern;

#[cfg(test)]
mod expression_produces_string_tests {
    use super::*;

    #[test]
    fn test_format_macro() {
        let expr = expr_macro("format", vec![]);
        assert!(expression_produces_string(&expr));
    }

    #[test]
    fn test_to_string_method() {
        let expr = expr_method(expr_var("obj"), "to_string", vec![]);
        assert!(expression_produces_string(&expr));
    }

    #[test]
    fn test_len_method_not_string() {
        let expr = expr_method(expr_var("arr"), "len", vec![]);
        assert!(!expression_produces_string(&expr));
    }
}

#[cfg(test)]
mod as_str_detection_tests {
    use super::*;

    #[test]
    fn test_expression_has_as_str() {
        let expr = expr_method(expr_var("s"), "as_str", vec![]);
        assert!(expression_has_as_str(&expr));
    }

    #[test]
    fn test_expression_no_as_str() {
        let expr = expr_method(expr_var("s"), "to_string", vec![]);
        assert!(!expression_has_as_str(&expr));
    }

    #[test]
    fn test_statement_expression_has_as_str() {
        let stmt = stmt_expr(expr_method(expr_var("s"), "as_str", vec![]));
        assert!(statement_has_as_str(&stmt));
    }

    #[test]
    fn test_statement_no_as_str() {
        let stmt = stmt_let("x", None, expr_int(5));
        assert!(!statement_has_as_str(&stmt));
    }

    #[test]
    fn test_block_has_as_str() {
        let block = vec![stmt_expr(expr_method(expr_var("s"), "as_str", vec![]))];
        assert!(block_has_as_str(&block));
    }

    #[test]
    fn test_empty_block_no_as_str() {
        let block = vec![];
        assert!(!block_has_as_str(&block));
    }
}

// =============================================================================
// Explicit Reference Detection Tests (Phase 8 - Retroactive TDD)
// =============================================================================

#[cfg(test)]
mod explicit_ref_tests {
    use super::*;
    use windjammer::parser::UnaryOp;

    #[test]
    fn test_expression_is_explicit_ref_with_ref() {
        // &x → true
        let expr = expr_unary(UnaryOp::Ref, expr_var("x"));
        assert!(expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_expression_is_explicit_ref_without_ref() {
        // x → false
        let expr = expr_var("x");
        assert!(!expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_expression_is_explicit_ref_with_block() {
        // { &x } → true (recursive check)
        let expr = expr_block(vec![stmt_expr(expr_unary(UnaryOp::Ref, expr_var("x")))]);
        assert!(expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_expression_is_explicit_ref_with_block_no_ref() {
        // { x } → false
        let expr = expr_block(vec![stmt_expr(expr_var("x"))]);
        assert!(!expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_block_has_explicit_ref_last_statement_ref() {
        // { let y = 1; &x } → true (only last statement matters)
        let block = vec![
            stmt_let("y", None, expr_int(1)),
            stmt_expr(expr_unary(UnaryOp::Ref, expr_var("x"))),
        ];
        assert!(block_has_explicit_ref(&block));
    }

    #[test]
    fn test_block_has_explicit_ref_return_statement() {
        // { return &x; } → true
        let block = vec![stmt_return(Some(expr_unary(UnaryOp::Ref, expr_var("x"))))];
        assert!(block_has_explicit_ref(&block));
    }

    #[test]
    fn test_block_has_explicit_ref_no_ref() {
        // { x } → false
        let block = vec![stmt_expr(expr_var("x"))];
        assert!(!block_has_explicit_ref(&block));
    }

    #[test]
    fn test_block_has_explicit_ref_empty() {
        // {} → false
        let block = vec![];
        assert!(!block_has_explicit_ref(&block));
    }

    #[test]
    fn test_block_has_explicit_ref_non_expression_last() {
        // { let x = 1; } → false (last statement is not Expression or Return)
        let block = vec![stmt_let("x", None, expr_int(1))];
        assert!(!block_has_explicit_ref(&block));
    }
}

// TDD Tests for extended string expression analysis functions (Phase 6)
//
// UPDATED: Now using AST builder functions!

use windjammer::codegen::rust::string_analysis::{
    block_has_as_str, block_has_explicit_ref, expression_has_as_str, expression_is_explicit_ref,
    expression_produces_string, statement_has_as_str,
};
use windjammer::parser::ast::builders::*;
use windjammer::test_utils::*;

// Arena-allocating wrappers
fn alloc_var(name: &str) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_var(name))
}

fn alloc_int(n: i64) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_int(n))
}

fn alloc_method(obj: &'static windjammer::parser::Expression<'static>, method: &str, args: Vec<&'static windjammer::parser::Expression<'static>>) -> &'static windjammer::parser::Expression<'static> {
    let args_with_names: Vec<(Option<String>, &'static windjammer::parser::Expression<'static>)> = args.into_iter().map(|e| (None, e)).collect();
    test_alloc_expr(expr_method(obj, method, args_with_names))
}

fn alloc_macro(name: &str, args: Vec<&'static windjammer::parser::Expression<'static>>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_macro(name, args))
}

fn alloc_unary(op: windjammer::parser::UnaryOp, operand: &'static windjammer::parser::Expression<'static>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_unary(op, operand))
}

fn alloc_block(stmts: Vec<&'static windjammer::parser::Statement<'static>>) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(expr_block(stmts))
}

fn alloc_stmt_expr(expr: &'static windjammer::parser::Expression<'static>) -> &'static windjammer::parser::Statement<'static> {
    test_alloc_stmt(stmt_expr(expr))
}

fn alloc_stmt_let(name: &str, type_: Option<windjammer::parser::Type>, value: &'static windjammer::parser::Expression<'static>) -> &'static windjammer::parser::Statement<'static> {
    test_alloc_stmt(stmt_let(name, type_, value))
}

fn alloc_stmt_return(value: Option<&'static windjammer::parser::Expression<'static>>) -> &'static windjammer::parser::Statement<'static> {
    test_alloc_stmt(stmt_return(value))
}

#[cfg(test)]
mod expression_produces_string_tests {
    use super::*;

    #[test]
    fn test_format_macro() {
        let expr = alloc_macro("format", vec![]);
        assert!(expression_produces_string(&expr));
    }

    #[test]
    fn test_to_string_method() {
        let expr = alloc_method(alloc_var("obj"), "to_string", vec![]);
        assert!(expression_produces_string(&expr));
    }

    #[test]
    fn test_len_method_not_string() {
        let expr = alloc_method(alloc_var("arr"), "len", vec![]);
        assert!(!expression_produces_string(&expr));
    }
}

#[cfg(test)]
mod as_str_detection_tests {
    use super::*;

    #[test]
    fn test_expression_has_as_str() {
        let expr = alloc_method(alloc_var("s"), "as_str", vec![]);
        assert!(expression_has_as_str(&expr));
    }

    #[test]
    fn test_expression_no_as_str() {
        let expr = alloc_method(alloc_var("s"), "to_string", vec![]);
        assert!(!expression_has_as_str(&expr));
    }

    #[test]
    fn test_statement_expression_has_as_str() {
        let stmt = alloc_stmt_expr(alloc_method(alloc_var("s"), "as_str", vec![]));
        assert!(statement_has_as_str(&stmt));
    }

    #[test]
    fn test_statement_no_as_str() {
        let stmt = alloc_stmt_let("x", None, alloc_int(5));
        assert!(!statement_has_as_str(&stmt));
    }

    #[test]
    fn test_block_has_as_str() {
        let block = vec![alloc_stmt_expr(alloc_method(alloc_var("s"), "as_str", vec![]))];
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
        let expr = alloc_unary(UnaryOp::Ref, alloc_var("x"));
        assert!(expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_expression_is_explicit_ref_without_ref() {
        // x → false
        let expr = alloc_var("x");
        assert!(!expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_expression_is_explicit_ref_with_block() {
        // { &x } → true (recursive check)
        let expr = alloc_block(vec![alloc_stmt_expr(alloc_unary(UnaryOp::Ref, alloc_var("x")))]);
        assert!(expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_expression_is_explicit_ref_with_block_no_ref() {
        // { x } → false
        let expr = alloc_block(vec![alloc_stmt_expr(alloc_var("x"))]);
        assert!(!expression_is_explicit_ref(&expr));
    }

    #[test]
    fn test_block_has_explicit_ref_last_statement_ref() {
        // { let y = 1; &x } → true (only last statement matters)
        let block = vec![
            alloc_stmt_let("y", None, alloc_int(1)),
            alloc_stmt_expr(alloc_unary(UnaryOp::Ref, alloc_var("x"))),
        ];
        assert!(block_has_explicit_ref(&block));
    }

    #[test]
    fn test_block_has_explicit_ref_return_statement() {
        // { return &x; } → true
        let block = vec![alloc_stmt_return(Some(alloc_unary(UnaryOp::Ref, alloc_var("x"))))];
        assert!(block_has_explicit_ref(&block));
    }

    #[test]
    fn test_block_has_explicit_ref_no_ref() {
        // { x } → false
        let block = vec![alloc_stmt_expr(alloc_var("x"))];
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
        let block = vec![alloc_stmt_let("x", None, alloc_int(1))];
        assert!(!block_has_explicit_ref(&block));
    }
}

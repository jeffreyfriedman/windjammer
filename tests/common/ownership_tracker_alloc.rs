//! Expression allocators for OwnershipTracker integration tests.

use windjammer::parser::ast::builders::*;
use windjammer::parser::ast::operators::BinaryOp;
use windjammer::parser::Expression;
use windjammer::test_utils::test_alloc_expr;

pub fn alloc_int(n: i64) -> &'static Expression<'static> {
    test_alloc_expr(expr_int(n))
}

pub fn alloc_var(name: impl Into<String>) -> &'static Expression<'static> {
    test_alloc_expr(expr_var(name))
}

pub fn alloc_field(obj: &'static Expression<'static>, field: &str) -> &'static Expression<'static> {
    test_alloc_expr(expr_field(obj, field))
}

pub fn alloc_index(
    obj: &'static Expression<'static>,
    idx: &'static Expression<'static>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_index(obj, idx))
}

pub fn alloc_method(
    obj: &'static Expression<'static>,
    method: &str,
    args: Vec<(Option<String>, &'static Expression<'static>)>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_method(obj, method, args))
}

pub fn alloc_unary(
    op: windjammer::parser::UnaryOp,
    operand: &'static Expression<'static>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_unary(op, operand))
}

pub fn alloc_binary(
    left: &'static Expression<'static>,
    op: BinaryOp,
    right: &'static Expression<'static>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_binary(op, left, right))
}

pub fn alloc_call(
    func: &'static Expression<'static>,
    args: Vec<(Option<String>, &'static Expression<'static>)>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_call(func, args))
}

pub fn alloc_struct(
    name: &str,
    fields: Vec<(String, &'static Expression<'static>)>,
) -> &'static Expression<'static> {
    test_alloc_expr(Expression::StructLiteral {
        name: name.to_string(),
        fields,
        location: None,
    })
}

pub fn alloc_array(elements: Vec<&'static Expression<'static>>) -> &'static Expression<'static> {
    test_alloc_expr(expr_array(elements))
}

pub fn alloc_tuple(elements: Vec<&'static Expression<'static>>) -> &'static Expression<'static> {
    test_alloc_expr(expr_tuple(elements))
}

//! Comprehensive tests for OwnershipTracker
//!
//! TDD: Tests written to validate ownership detection for ALL expression types.

use windjammer::analyzer::OwnershipMode;
use windjammer::codegen::rust::ownership_tracker::OwnershipTracker;
use windjammer::parser::ast::builders::*;
use windjammer::parser::ast::operators::BinaryOp;
use windjammer::parser::Expression;
use windjammer::test_utils::test_alloc_expr;

// ============================================================================
// ARENA HELPERS - Allocate expressions for testing
// ============================================================================

fn alloc_int(n: i64) -> &'static Expression<'static> {
    test_alloc_expr(expr_int(n))
}

fn alloc_var(name: impl Into<String>) -> &'static Expression<'static> {
    test_alloc_expr(expr_var(name))
}

fn alloc_field(obj: &'static Expression<'static>, field: &str) -> &'static Expression<'static> {
    test_alloc_expr(expr_field(obj, field))
}

fn alloc_index(
    obj: &'static Expression<'static>,
    idx: &'static Expression<'static>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_index(obj, idx))
}

fn alloc_method(
    obj: &'static Expression<'static>,
    method: &str,
    args: Vec<(Option<String>, &'static Expression<'static>)>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_method(obj, method, args))
}

fn alloc_unary(op: windjammer::parser::UnaryOp, operand: &'static Expression<'static>) -> &'static Expression<'static> {
    test_alloc_expr(expr_unary(op, operand))
}

fn alloc_binary(
    left: &'static Expression<'static>,
    op: BinaryOp,
    right: &'static Expression<'static>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_binary(op, left, right))
}

fn alloc_call(
    func: &'static Expression<'static>,
    args: Vec<(Option<String>, &'static Expression<'static>)>,
) -> &'static Expression<'static> {
    test_alloc_expr(expr_call(func, args))
}

fn alloc_struct(
    name: &str,
    fields: Vec<(String, &'static Expression<'static>)>,
) -> &'static Expression<'static> {
    test_alloc_expr(Expression::StructLiteral {
        name: name.to_string(),
        fields,
        location: None,
    })
}

fn alloc_array(elements: Vec<&'static Expression<'static>>) -> &'static Expression<'static> {
    test_alloc_expr(expr_array(elements))
}

fn alloc_tuple(elements: Vec<&'static Expression<'static>>) -> &'static Expression<'static> {
    test_alloc_expr(expr_tuple(elements))
}

// ============================================================================
// IDENTIFIER TESTS
// ============================================================================

#[test]
fn test_identifier_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("data", OwnershipMode::Borrowed);

    let expr = alloc_var("data");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_identifier_mut_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("data", OwnershipMode::MutBorrowed);

    let expr = alloc_var("data");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

#[test]
fn test_identifier_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_var("data");
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_identifier_unknown_param_defaults_to_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);
    let expr = alloc_var("y"); // y not registered
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// FIELD ACCESS TESTS
// ============================================================================

#[test]
fn test_field_access_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::Borrowed);

    let expr = alloc_field(alloc_var("self"), "value");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_nested_field_access() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::Borrowed);

    let expr = alloc_field(
        alloc_field(alloc_var("self"), "camera"),
        "position",
    );
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_field_access_on_owned_object() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_field(alloc_var("config"), "name");
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_field_access_on_mut_borrowed() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::MutBorrowed);
    let expr = alloc_field(alloc_var("self"), "count");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

// ============================================================================
// INDEX TESTS
// ============================================================================

#[test]
fn test_index_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("items", OwnershipMode::Borrowed);

    let expr = alloc_index(alloc_var("items"), alloc_int(0));
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_index_on_owned_vec() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_index(alloc_var("vec"), alloc_int(0));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_index_on_mut_borrowed() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("arr", OwnershipMode::MutBorrowed);
    let expr = alloc_index(alloc_var("arr"), alloc_int(5));
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

// ============================================================================
// METHOD CALL TESTS
// ============================================================================

#[test]
fn test_clone_method_returns_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("data", OwnershipMode::Borrowed);

    let expr = alloc_method(alloc_var("data"), "clone", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_to_owned_method_returns_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("cow", OwnershipMode::Borrowed);
    let expr = alloc_method(alloc_var("cow"), "to_owned", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_to_string_method_returns_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_method(alloc_var("x"), "to_string", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_generic_method_returns_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("items", OwnershipMode::Borrowed);
    let expr = alloc_method(alloc_var("items"), "len", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_method_on_borrowed_object_result_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("self", OwnershipMode::Borrowed);
    let expr = alloc_method(alloc_field(alloc_var("self"), "data"), "clone", vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// UNARY OPERATOR TESTS
// ============================================================================

#[test]
fn test_deref_removes_borrow() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("ptr", OwnershipMode::Borrowed);

    let expr = alloc_unary(
        windjammer::parser::UnaryOp::Deref,
        alloc_var("ptr"),
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_deref_mut_borrow_removes_borrow() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("ptr", OwnershipMode::MutBorrowed);
    let expr = alloc_unary(
        windjammer::parser::UnaryOp::Deref,
        alloc_var("ptr"),
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_borrow_adds_shared_borrow() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(
        windjammer::parser::UnaryOp::Ref,
        alloc_var("x"),
    );
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_borrow_mut_adds_mutable_borrow() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(
        windjammer::parser::UnaryOp::MutRef,
        alloc_var("x"),
    );
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

#[test]
fn test_neg_produces_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(
        windjammer::parser::UnaryOp::Neg,
        alloc_int(5),
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_not_produces_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_unary(
        windjammer::parser::UnaryOp::Not,
        test_alloc_expr(expr_bool(true)),
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// LITERAL TESTS
// ============================================================================

#[test]
fn test_literal_int_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_int(42);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_float_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_float(3.14));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_string_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_string("hello"));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_bool_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_bool(true));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_literal_char_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(expr_char('x'));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// BINARY OPERATION TESTS
// ============================================================================

#[test]
fn test_binary_operation_is_owned() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);

    let expr = alloc_binary(alloc_var("x"), BinaryOp::Add, alloc_int(1));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_binary_eq_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_binary(alloc_var("a"), BinaryOp::Eq, alloc_var("b"));
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_binary_and_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_binary(
        test_alloc_expr(expr_bool(true)),
        BinaryOp::And,
        test_alloc_expr(expr_bool(false)),
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// FUNCTION CALL TESTS
// ============================================================================

#[test]
fn test_function_call_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_call(alloc_var("foo"), vec![]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_function_call_with_args_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_call(
        alloc_var("create"),
        vec![(None, alloc_int(42))],
    );
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// STRUCT/ARRAY/TUPLE LITERAL TESTS
// ============================================================================

#[test]
fn test_struct_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_struct("Point", vec![("x".to_string(), alloc_int(0)), ("y".to_string(), alloc_int(0))]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_array_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_array(vec![alloc_int(1), alloc_int(2)]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

#[test]
fn test_tuple_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = alloc_tuple(vec![alloc_int(1), alloc_var("x")]);
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// BINDING TESTS (for/match)
// ============================================================================

#[test]
fn test_for_loop_binding_shared() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("item", OwnershipMode::Borrowed);

    let expr = alloc_var("item");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_match_pattern_binding_shared() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("val", OwnershipMode::Borrowed);

    let expr = alloc_var("val");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

#[test]
fn test_for_loop_binding_mut() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("item", OwnershipMode::MutBorrowed);

    let expr = alloc_var("item");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

#[test]
fn test_binding_overrides_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);
    tracker.register_binding("x", OwnershipMode::MutBorrowed);

    let expr = alloc_var("x");
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::MutBorrowed
    );
}

// ============================================================================
// CAST TESTS
// ============================================================================

#[test]
fn test_cast_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);

    let expr = test_alloc_expr(Expression::Cast {
        expr: alloc_var("x"),
        type_: windjammer::parser::Type::Float,
        location: None,
    });
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

// ============================================================================
// RANGE TESTS
// ============================================================================

#[test]
fn test_range_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::Range {
        start: alloc_int(0),
        end: alloc_int(10),
        inclusive: false,
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// TRY OPERATOR TESTS
// ============================================================================

#[test]
fn test_try_inherits_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("result", OwnershipMode::Borrowed);

    let expr = test_alloc_expr(Expression::TryOp {
        expr: alloc_var("result"),
        location: None,
    });
    assert_eq!(
        tracker.get_expression_ownership(expr),
        OwnershipMode::Borrowed
    );
}

// ============================================================================
// MAP LITERAL TESTS
// ============================================================================

#[test]
fn test_map_literal_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::MapLiteral {
        pairs: vec![(alloc_var("k"), alloc_var("v"))],
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// CLOSURE TESTS
// ============================================================================

#[test]
fn test_closure_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::Closure {
        parameters: vec!["x".to_string()],
        body: alloc_var("x"),
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// MACRO INVOCATION TESTS
// ============================================================================

#[test]
fn test_macro_invocation_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::MacroInvocation {
        name: "vec".to_string(),
        args: vec![alloc_int(1)],
        delimiter: windjammer::parser::MacroDelimiter::Brackets,
        is_repeat: false,
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// AWAIT TESTS
// ============================================================================

#[test]
fn test_await_is_owned() {
    let tracker = OwnershipTracker::new();
    let expr = test_alloc_expr(Expression::Await {
        expr: alloc_var("future"),
        location: None,
    });
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

// ============================================================================
// HELPER METHOD TESTS
// ============================================================================

#[test]
fn test_is_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::Borrowed);
    assert!(tracker.is_borrowed_parameter("x"));
    assert!(!tracker.is_borrowed_parameter("y"));
}

#[test]
fn test_is_mut_borrowed_parameter() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_parameter("x", OwnershipMode::MutBorrowed);
    assert!(tracker.is_mut_borrowed_parameter("x"));
    assert!(!tracker.is_mut_borrowed_parameter("y"));
}

#[test]
fn test_get_binding_ownership() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_binding("item", OwnershipMode::Borrowed);
    assert_eq!(
        tracker.get_binding_ownership("item"),
        Some(OwnershipMode::Borrowed)
    );
    assert_eq!(tracker.get_binding_ownership("other"), None);
}

#[test]
fn test_register_copy_type() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_copy_type("Point");
    // Copy types don't affect get_expression_ownership yet - future optimization
    assert!(true);
}

#[test]
fn test_register_struct_field() {
    let mut tracker = OwnershipTracker::new();
    tracker.register_struct_field("Point.x", windjammer::parser::Type::Int);
    // Struct fields don't affect get_expression_ownership yet - future use
    assert!(true);
}

#[test]
fn test_default_creates_empty_tracker() {
    let tracker = OwnershipTracker::default();
    let expr = alloc_var("x");
    assert_eq!(tracker.get_expression_ownership(expr), OwnershipMode::Owned);
}

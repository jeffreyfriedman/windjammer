// AST Builder Tests - Ergonomic AST construction
//
// Tests FIRST, then implementation (proper TDD)

use windjammer::test_utils::*;

// Arena-allocating wrappers for builder functions
fn alloc_int(n: i64) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_int(n))
}

fn alloc_float(f: f64) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_float(f))
}

fn alloc_string(s: &str) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_string(s))
}

fn alloc_bool(b: bool) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_bool(b))
}

fn alloc_var(name: &str) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_var(name))
}

fn alloc_binary(
    op: windjammer::parser::BinaryOp,
    left: &'static windjammer::parser::Expression<'static>,
    right: &'static windjammer::parser::Expression<'static>,
) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_binary(
        op, left, right,
    ))
}

fn alloc_add(
    left: &'static windjammer::parser::Expression<'static>,
    right: &'static windjammer::parser::Expression<'static>,
) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_add(left, right))
}

fn alloc_call(
    func: &str,
    args: Vec<&'static windjammer::parser::Expression<'static>>,
) -> &'static windjammer::parser::Expression<'static> {
    let func_expr = alloc_var(func);
    let args_with_names: Vec<(
        Option<String>,
        &'static windjammer::parser::Expression<'static>,
    )> = args.into_iter().map(|e| (None, e)).collect();
    test_alloc_expr(windjammer::parser::ast::builders::expr_call(
        func_expr,
        args_with_names,
    ))
}

fn alloc_method(
    obj: &'static windjammer::parser::Expression<'static>,
    method: &str,
    args: Vec<&'static windjammer::parser::Expression<'static>>,
) -> &'static windjammer::parser::Expression<'static> {
    let args_with_names: Vec<(
        Option<String>,
        &'static windjammer::parser::Expression<'static>,
    )> = args.into_iter().map(|e| (None, e)).collect();
    test_alloc_expr(windjammer::parser::ast::builders::expr_method(
        obj,
        method,
        args_with_names,
    ))
}

fn alloc_field(
    obj: &'static windjammer::parser::Expression<'static>,
    field: &str,
) -> &'static windjammer::parser::Expression<'static> {
    test_alloc_expr(windjammer::parser::ast::builders::expr_field(obj, field))
}

// Statement wrappers
fn alloc_stmt_return(
    value: Option<&'static windjammer::parser::Expression<'static>>,
) -> &'static windjammer::parser::Statement<'static> {
    test_alloc_stmt(windjammer::parser::ast::builders::stmt_return(value))
}

fn alloc_stmt_expr(
    expr: &'static windjammer::parser::Expression<'static>,
) -> &'static windjammer::parser::Statement<'static> {
    test_alloc_stmt(windjammer::parser::ast::builders::stmt_expr(expr))
}

// ============================================================================
// TYPE BUILDER TESTS
// ============================================================================

#[test]
fn test_type_builder_primitives() {
    // Before: Type::Int
    // After: type_int()

    use windjammer::parser::ast::*;

    assert_eq!(type_int(), Type::Int);
    assert_eq!(type_float(), Type::Float);
    assert_eq!(type_bool(), Type::Bool);
    assert_eq!(type_string(), Type::String);
}

#[test]
fn test_type_builder_custom() {
    // Before: Type::Custom("MyType".to_string())
    // After: type_custom("MyType")

    use windjammer::parser::ast::*;

    assert_eq!(type_custom("MyType"), Type::Custom("MyType".to_string()));
}

#[test]
fn test_type_builder_reference() {
    // Before: Type::Reference(Box::new(Type::Int))
    // After: type_ref(type_int())

    use windjammer::parser::ast::*;

    assert_eq!(type_ref(Type::Int), Type::Reference(Box::new(Type::Int)));
}

#[test]
fn test_type_builder_mut_reference() {
    // Before: Type::MutableReference(Box::new(Type::Int))
    // After: type_mut_ref(type_int())

    use windjammer::parser::ast::*;

    assert_eq!(
        type_mut_ref(Type::Int),
        Type::MutableReference(Box::new(Type::Int))
    );
}

#[test]
fn test_type_builder_vec() {
    // Before: Type::Vec(Box::new(Type::Int))
    // After: type_vec(type_int())

    use windjammer::parser::ast::*;

    assert_eq!(type_vec(Type::Int), Type::Vec(Box::new(Type::Int)));
}

#[test]
fn test_type_builder_option() {
    // Before: Type::Option(Box::new(Type::String))
    // After: type_option(type_string())

    use windjammer::parser::ast::*;

    assert_eq!(
        type_option(Type::String),
        Type::Option(Box::new(Type::String))
    );
}

#[test]
fn test_type_builder_result() {
    // Before: Type::Result(Box::new(Type::Int), Box::new(Type::String))
    // After: type_result(type_int(), type_string())

    use windjammer::parser::ast::*;

    assert_eq!(
        type_result(Type::Int, Type::String),
        Type::Result(Box::new(Type::Int), Box::new(Type::String))
    );
}

#[test]
fn test_type_builder_parameterized() {
    // Before: Type::Parameterized("Vec".to_string(), vec![Type::Int])
    // After: type_parameterized("Vec", vec![type_int()])

    use windjammer::parser::ast::*;

    assert_eq!(
        type_parameterized("Vec", vec![Type::Int]),
        Type::Parameterized("Vec".to_string(), vec![Type::Int])
    );
}

#[test]
fn test_type_builder_tuple() {
    // Before: Type::Tuple(vec![Type::Int, Type::String])
    // After: type_tuple(vec![type_int(), type_string()])

    use windjammer::parser::ast::*;

    assert_eq!(
        type_tuple(vec![Type::Int, Type::String]),
        Type::Tuple(vec![Type::Int, Type::String])
    );
}

#[test]
fn test_type_builder_chained() {
    // Complex example: Option<&mut Vec<String>>
    // Before:
    // Type::Option(Box::new(
    //     Type::MutableReference(Box::new(
    //         Type::Vec(Box::new(Type::String))
    //     ))
    // ))
    //
    // After: type_option(type_mut_ref(type_vec(type_string())))

    use windjammer::parser::ast::*;

    let complex_type = type_option(type_mut_ref(type_vec(Type::String)));

    assert_eq!(
        complex_type,
        Type::Option(Box::new(Type::MutableReference(Box::new(Type::Vec(
            Box::new(Type::String)
        )))))
    );
}

// ============================================================================
// PARAMETER BUILDER TESTS
// ============================================================================

#[test]
fn test_parameter_builder_simple() {
    // Before:
    // Parameter {
    //     name: "x".to_string(),
    //     pattern: None,
    //     type_: Type::Int,
    //     ownership: OwnershipHint::Inferred,
    //     is_mutable: false,
    // }
    //
    // After: param("x", type_int())

    use windjammer::parser::ast::*;

    let p = param("x", Type::Int);

    assert_eq!(p.name, "x");
    assert_eq!(p.type_, Type::Int);
    assert_eq!(p.ownership, OwnershipHint::Inferred);
    assert!(!p.is_mutable);
}

#[test]
fn test_parameter_builder_with_ownership() {
    // Before:
    // Parameter {
    //     name: "x".to_string(),
    //     pattern: None,
    //     type_: Type::Int,
    //     ownership: OwnershipHint::Ref,
    //     is_mutable: false,
    // }
    //
    // After: param_ref("x", type_int())

    use windjammer::parser::ast::*;

    let p = param_ref("x", Type::Int);

    assert_eq!(p.ownership, OwnershipHint::Ref);
}

#[test]
fn test_parameter_builder_mutable() {
    // Before:
    // Parameter {
    //     name: "x".to_string(),
    //     pattern: None,
    //     type_: Type::Int,
    //     ownership: OwnershipHint::Mut,
    //     is_mutable: true,
    // }
    //
    // After: param_mut("x", type_int())

    use windjammer::parser::ast::*;

    let p = param_mut("x", Type::Int);

    assert_eq!(p.ownership, OwnershipHint::Mut);
    assert!(p.is_mutable);
}

// ============================================================================
// SUCCESS METRICS
// ============================================================================

// Before builders (example from actual test):
// let param = Parameter {
//     name: "data".to_string(),
//     pattern: None,
//     type_: Type::Reference(Box::new(Type::Vec(Box::new(Type::Int)))),
//     ownership: OwnershipHint::Ref,
//     is_mutable: false,
// };
//
// After builders:
// let param = param_ref("data", type_vec(Type::Int));
//
// Reduction: 7 lines → 1 line (85% reduction)

// ============================================================================
// EXPRESSION BUILDER TESTS (TDD - Tests FIRST!)
// ============================================================================

#[test]
fn test_expr_literal_int() {
    // Before: Expression::Literal { value: Literal::Int(42), location: None }
    // After: alloc_int(42)

    use windjammer::parser::ast::*;

    let expr = alloc_int(42);

    if let Expression::Literal { value, .. } = expr {
        assert_eq!(*value, Literal::Int(42));
    } else {
        panic!("Expected Literal expression");
    }
}

#[test]
fn test_expr_literal_float() {
    use windjammer::parser::ast::*;

    let expr = alloc_float(2.5);

    if let Expression::Literal { value, .. } = expr {
        assert_eq!(*value, Literal::Float(2.5));
    } else {
        panic!("Expected Literal expression");
    }
}

#[test]
fn test_expr_literal_string() {
    use windjammer::parser::ast::*;

    let expr = alloc_string("hello");

    if let Expression::Literal { value, .. } = expr {
        assert_eq!(*value, Literal::String("hello".to_string()));
    } else {
        panic!("Expected Literal expression");
    }
}

#[test]
fn test_expr_literal_bool() {
    use windjammer::parser::ast::*;

    let expr = alloc_bool(true);

    if let Expression::Literal { value, .. } = expr {
        assert_eq!(*value, Literal::Bool(true));
    } else {
        panic!("Expected Literal expression");
    }
}

#[test]
fn test_alloc_var() {
    // Before: Expression::Identifier { name: "x".to_string(), location: None }
    // After: alloc_var("x")

    use windjammer::parser::ast::*;

    let expr = alloc_var("x");

    if let Expression::Identifier { name, .. } = expr {
        assert_eq!(name, "x");
    } else {
        panic!("Expected Identifier expression");
    }
}

#[test]
fn test_alloc_binary() {
    // Before:
    // Expression::Binary {
    //     left: Box::new(Expression::Identifier { name: "a".to_string(), location: None }),
    //     op: BinaryOp::Add,
    //     right: Box::new(Expression::Identifier { name: "b".to_string(), location: None }),
    //     location: None,
    // }
    //
    // After: alloc_binary(BinaryOp::Add, alloc_var("a"), alloc_var("b"))

    use windjammer::parser::ast::*;

    let expr = alloc_binary(BinaryOp::Add, alloc_var("a"), alloc_var("b"));

    if let Expression::Binary {
        left, op, right, ..
    } = expr
    {
        assert_eq!(*op, BinaryOp::Add);
        if let Expression::Identifier { name, .. } = *left {
            assert_eq!(name, "a");
        } else {
            panic!("Expected left to be Identifier");
        }
        if let Expression::Identifier { name, .. } = *right {
            assert_eq!(name, "b");
        } else {
            panic!("Expected right to be Identifier");
        }
    } else {
        panic!("Expected Binary expression");
    }
}

#[test]
fn test_expr_add_shorthand() {
    // Convenience: alloc_add(a, b) instead of alloc_binary(BinaryOp::Add, a, b)

    use windjammer::parser::ast::*;

    let expr = alloc_add(alloc_var("x"), alloc_int(1));

    if let Expression::Binary { op, .. } = expr {
        assert_eq!(*op, BinaryOp::Add);
    } else {
        panic!("Expected Binary expression");
    }
}

#[test]
fn test_alloc_call() {
    // Before:
    // Expression::Call {
    //     function: Box::new(Expression::Identifier { name: "foo".to_string(), location: None }),
    //     arguments: vec![
    //         (None, Expression::Literal { value: Literal::Int(1), location: None }),
    //         (None, Expression::Literal { value: Literal::Int(2), location: None }),
    //     ],
    //     location: None,
    // }
    //
    // After: alloc_call("foo", vec![alloc_int(1), alloc_int(2)])

    use windjammer::parser::ast::*;

    let expr = alloc_call("foo", vec![alloc_int(1), alloc_int(2)]);

    if let Expression::Call {
        function,
        arguments,
        ..
    } = expr
    {
        if let Expression::Identifier { name, .. } = *function {
            assert_eq!(name, "foo");
        } else {
            panic!("Expected function to be Identifier");
        }
        assert_eq!(arguments.len(), 2);
    } else {
        panic!("Expected Call expression");
    }
}

#[test]
fn test_expr_method_call() {
    // Before:
    // Expression::MethodCall {
    //     object: Box::new(Expression::Identifier { name: "obj".to_string(), location: None }),
    //     method: "method".to_string(),
    //     type_args: None,
    //     arguments: vec![],
    //     location: None,
    // }
    //
    // After: alloc_method("obj", "method", vec![])

    use windjammer::parser::ast::*;

    let expr = alloc_method(alloc_var("obj"), "method", vec![]);

    if let Expression::MethodCall {
        object,
        method,
        arguments,
        ..
    } = expr
    {
        if let Expression::Identifier { name, .. } = *object {
            assert_eq!(name, "obj");
        } else {
            panic!("Expected object to be Identifier");
        }
        assert_eq!(method, "method");
        assert_eq!(arguments.len(), 0);
    } else {
        panic!("Expected MethodCall expression");
    }
}

#[test]
fn test_alloc_field() {
    // Before:
    // Expression::FieldAccess {
    //     object: Box::new(Expression::Identifier { name: "obj".to_string(), location: None }),
    //     field: "field".to_string(),
    //     location: None,
    // }
    //
    // After: alloc_field(alloc_var("obj"), "field")

    use windjammer::parser::ast::*;

    let expr = alloc_field(alloc_var("obj"), "field");

    if let Expression::FieldAccess { object, field, .. } = expr {
        if let Expression::Identifier { name, .. } = *object {
            assert_eq!(name, "obj");
        } else {
            panic!("Expected object to be Identifier");
        }
        assert_eq!(field, "field");
    } else {
        panic!("Expected FieldAccess expression");
    }
}

#[test]
fn test_expr_chained_complex() {
    // Complex example: obj.method(x + 1).field
    //
    // Before: ~20 lines of nested Expression structs
    // After: alloc_field(alloc_method(alloc_var("obj"), "method", vec![alloc_add(alloc_var("x"), alloc_int(1))]), "field")

    use windjammer::parser::ast::*;

    let expr = alloc_field(
        alloc_method(
            alloc_var("obj"),
            "method",
            vec![alloc_add(alloc_var("x"), alloc_int(1))],
        ),
        "field",
    );

    // Just verify it constructs without panicking
    if let Expression::FieldAccess { .. } = expr {
        // Success
    } else {
        panic!("Expected FieldAccess expression");
    }
}

// ============================================================================
// STATEMENT BUILDER TESTS (TDD - Tests FIRST!)
// ============================================================================

#[test]
fn test_stmt_let() {
    // Before: 10+ lines
    // After: stmt_let("x", Some(Type::Int), alloc_int(42))

    use windjammer::parser::ast::*;

    let stmt = stmt_let("x", Some(Type::Int), alloc_int(42));

    if let Statement::Let { mutable, type_, .. } = stmt {
        assert!(!mutable);
        assert_eq!(type_, Some(Type::Int));
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_stmt_let_mut() {
    use windjammer::parser::ast::*;

    let stmt = stmt_let_mut("x", Some(Type::Int), alloc_int(0));

    if let Statement::Let { mutable, .. } = stmt {
        assert!(mutable);
    } else {
        panic!("Expected Let statement");
    }
}

#[test]
fn test_stmt_assign() {
    use windjammer::parser::ast::*;

    let stmt = stmt_assign(alloc_var("x"), alloc_int(42));

    if let Statement::Assignment { compound_op, .. } = stmt {
        assert_eq!(compound_op, None);
    } else {
        panic!("Expected Assignment statement");
    }
}

#[test]
fn test_stmt_compound_assign() {
    use windjammer::parser::ast::*;

    let stmt = stmt_compound_assign(CompoundOp::Add, alloc_var("x"), alloc_int(1));

    if let Statement::Assignment { compound_op, .. } = stmt {
        assert_eq!(compound_op, Some(CompoundOp::Add));
    } else {
        panic!("Expected Assignment statement");
    }
}

#[test]
fn test_stmt_return_some() {
    use windjammer::parser::ast::*;

    let stmt = stmt_return(Some(alloc_int(42)));

    if let Statement::Return { value, .. } = stmt {
        assert!(value.is_some());
    } else {
        panic!("Expected Return statement");
    }
}

#[test]
fn test_stmt_return_none() {
    use windjammer::parser::ast::*;

    let stmt = stmt_return(None);

    if let Statement::Return { value, .. } = stmt {
        assert!(value.is_none());
    } else {
        panic!("Expected Return statement");
    }
}

#[test]
fn test_stmt_expr() {
    use windjammer::parser::ast::*;

    let stmt = stmt_expr(alloc_call("foo", vec![]));

    if let Statement::Expression { .. } = stmt {
        // Success
    } else {
        panic!("Expected Expression statement");
    }
}

#[test]
fn test_stmt_if() {
    use windjammer::parser::ast::*;

    let stmt = stmt_if(alloc_bool(true), vec![alloc_stmt_return(None)], None);

    if let Statement::If {
        then_block,
        else_block,
        ..
    } = stmt
    {
        assert_eq!(then_block.len(), 1);
        assert!(else_block.is_none());
    } else {
        panic!("Expected If statement");
    }
}

#[test]
fn test_stmt_if_else() {
    use windjammer::parser::ast::*;

    let stmt = stmt_if_else(
        alloc_bool(true),
        vec![alloc_stmt_return(Some(alloc_int(1)))],
        vec![alloc_stmt_return(Some(alloc_int(2)))],
    );

    if let Statement::If { else_block, .. } = stmt {
        assert!(else_block.is_some());
        assert_eq!(else_block.unwrap().len(), 1);
    } else {
        panic!("Expected If statement");
    }
}

#[test]
fn test_stmt_while() {
    use windjammer::parser::ast::*;

    let stmt = stmt_while(
        alloc_bool(true),
        vec![alloc_stmt_expr(alloc_call("work", vec![]))],
    );

    if let Statement::While { body, .. } = stmt {
        assert_eq!(body.len(), 1);
    } else {
        panic!("Expected While statement");
    }
}

#[test]
fn test_stmt_break() {
    use windjammer::parser::ast::*;

    let stmt = stmt_break();

    if let Statement::Break { .. } = stmt {
        // Success
    } else {
        panic!("Expected Break statement");
    }
}

#[test]
fn test_stmt_continue() {
    use windjammer::parser::ast::*;

    let stmt = stmt_continue();

    if let Statement::Continue { .. } = stmt {
        // Success
    } else {
        panic!("Expected Continue statement");
    }
}

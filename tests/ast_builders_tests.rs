// AST Builder Tests - Ergonomic AST construction
//
// Tests FIRST, then implementation (proper TDD)

use windjammer::parser::ast::{Type, Parameter, OwnershipHint};

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
    
    assert_eq!(
        type_ref(Type::Int),
        Type::Reference(Box::new(Type::Int))
    );
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
    
    assert_eq!(
        type_vec(Type::Int),
        Type::Vec(Box::new(Type::Int))
    );
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
        Type::Option(Box::new(
            Type::MutableReference(Box::new(
                Type::Vec(Box::new(Type::String))
            ))
        ))
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
    assert_eq!(p.is_mutable, false);
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
    assert_eq!(p.is_mutable, true);
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


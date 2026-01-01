//! Comprehensive Parser Type Tests
//!
//! These tests verify that the parser correctly parses all type annotations.
//! Types appear in function parameters, return types, let bindings, struct fields, etc.
//!
//! Note: Some types like i32, f32 may be parsed as Custom("i32") rather than
//! specific Type variants - both are valid depending on parser implementation.

use windjammer::lexer::Lexer;
use windjammer::parser::ast::*;
use windjammer::parser_impl::Parser;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn parse_program(input: &str) -> Program<'_> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    parser.parse().expect("Failed to parse program")
}

fn get_fn_param_type(input: &str) -> Type {
    let program = parse_program(input);
    if let Some(Item::Function { decl, .. }) = program.items.first() {
        if let Some(param) = decl.parameters.first() {
            return param.type_.clone();
        }
    }
    panic!("Failed to extract parameter type from: {}", input);
}

fn get_fn_return_type(input: &str) -> Option<Type> {
    let program = parse_program(input);
    if let Some(Item::Function { decl, .. }) = program.items.first() {
        return decl.return_type.clone();
    }
    panic!("Failed to extract return type from: {}", input);
}

fn get_struct_field_type(input: &str) -> Type {
    let program = parse_program(input);
    if let Some(Item::Struct { decl, .. }) = program.items.first() {
        if let Some(field) = decl.fields.first() {
            return field.field_type.clone();
        }
    }
    panic!("Failed to extract field type from: {}", input);
}

/// Check if a type is a numeric type (either specific variant or Custom)
#[allow(dead_code)]
fn is_numeric_type(ty: &Type, expected_name: &str) -> bool {
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float => true,
        Type::Custom(name) => name == expected_name,
        _ => false,
    }
}

// ============================================================================
// PRIMITIVE TYPES
// ============================================================================

#[test]
fn test_type_i32() {
    let ty = get_fn_param_type("fn foo(x: i32) { }");
    assert!(
        matches!(ty, Type::Int32 | Type::Custom(_)),
        "Expected Int32 or Custom, got {:?}",
        ty
    );
}

#[test]
fn test_type_int() {
    let ty = get_fn_param_type("fn foo(x: int) { }");
    assert!(
        matches!(ty, Type::Int | Type::Custom(_)),
        "Expected Int or Custom, got {:?}",
        ty
    );
}

#[test]
fn test_type_float() {
    let ty = get_fn_param_type("fn foo(x: float) { }");
    assert!(
        matches!(ty, Type::Float | Type::Custom(_)),
        "Expected Float or Custom, got {:?}",
        ty
    );
}

#[test]
fn test_type_bool() {
    let ty = get_fn_param_type("fn foo(x: bool) { }");
    assert!(matches!(ty, Type::Bool), "Expected Bool, got {:?}", ty);
}

#[test]
fn test_type_string() {
    let ty = get_fn_param_type("fn foo(x: string) { }");
    assert!(matches!(ty, Type::String), "Expected String, got {:?}", ty);
}

// ============================================================================
// CUSTOM/NAMED TYPES
// ============================================================================

#[test]
fn test_type_custom() {
    let ty = get_fn_param_type("fn foo(x: Point) { }");
    if let Type::Custom(name) = ty {
        assert_eq!(name, "Point");
    } else {
        panic!("Expected Custom type, got {:?}", ty);
    }
}

#[test]
fn test_type_custom_camelcase() {
    let ty = get_fn_param_type("fn foo(x: MyCustomType) { }");
    if let Type::Custom(name) = ty {
        assert_eq!(name, "MyCustomType");
    } else {
        panic!("Expected Custom type");
    }
}

// ============================================================================
// PARAMETERIZED (GENERIC) TYPES
// ============================================================================

#[test]
fn test_type_vec() {
    let ty = get_fn_param_type("fn foo(x: Vec<i32>) { }");
    match ty {
        Type::Vec(inner) => {
            // Inner can be Int32 or Custom("i32")
            assert!(matches!(*inner, Type::Int32 | Type::Custom(_)));
        }
        Type::Parameterized(name, args) => {
            assert_eq!(name, "Vec");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected Vec or Parameterized type, got {:?}", ty),
    }
}

#[test]
fn test_type_option() {
    let ty = get_fn_param_type("fn foo(x: Option<String>) { }");
    match ty {
        Type::Option(inner) => {
            // inner could be String or Custom("String")
            assert!(matches!(*inner, Type::String | Type::Custom(_)));
        }
        Type::Parameterized(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected Option or Parameterized type, got {:?}", ty),
    }
}

#[test]
fn test_type_result() {
    let ty = get_fn_param_type("fn foo(x: Result<i32, Error>) { }");
    match ty {
        Type::Result(ok, _err) => {
            // ok can be Int32 or Custom("i32")
            assert!(matches!(*ok, Type::Int32 | Type::Custom(_)));
        }
        Type::Parameterized(name, args) => {
            assert_eq!(name, "Result");
            assert_eq!(args.len(), 2);
        }
        _ => panic!("Expected Result or Parameterized type, got {:?}", ty),
    }
}

#[test]
fn test_type_parameterized() {
    let ty = get_fn_param_type("fn foo(x: HashMap<String, i32>) { }");
    if let Type::Parameterized(name, args) = ty {
        assert_eq!(name, "HashMap");
        assert_eq!(args.len(), 2);
    } else {
        panic!("Expected Parameterized type, got {:?}", ty);
    }
}

#[test]
fn test_type_nested_generic() {
    // Note: Nested generics with >> may require space: Vec<Option<i32> >
    // This is a known parser limitation with >> being parsed as shift operator
    let ty = get_fn_param_type("fn foo(x: Vec<Option<i32> >) { }");
    match ty {
        Type::Vec(inner) => {
            // Inner type should be Option<i32>
            assert!(matches!(
                *inner,
                Type::Option(_) | Type::Parameterized(_, _)
            ));
        }
        Type::Parameterized(name, args) => {
            assert_eq!(name, "Vec");
            assert_eq!(args.len(), 1);
        }
        _ => panic!("Expected Vec or Parameterized type, got {:?}", ty),
    }
}

// ============================================================================
// REFERENCE TYPES
// ============================================================================

#[test]
fn test_type_ref() {
    let ty = get_fn_param_type("fn foo(x: &i32) { }");
    if let Type::Reference(inner) = ty {
        // inner can be Int32 or Custom("i32")
        assert!(matches!(*inner, Type::Int32 | Type::Custom(_)));
    } else {
        panic!("Expected Reference type, got {:?}", ty);
    }
}

#[test]
fn test_type_mut_ref() {
    let ty = get_fn_param_type("fn foo(x: &mut i32) { }");
    if let Type::MutableReference(inner) = ty {
        // inner can be Int32 or Custom("i32")
        assert!(matches!(*inner, Type::Int32 | Type::Custom(_)));
    } else {
        panic!("Expected MutableReference type, got {:?}", ty);
    }
}

#[test]
fn test_type_ref_string() {
    let ty = get_fn_param_type("fn foo(x: &string) { }");
    if let Type::Reference(inner) = ty {
        assert!(matches!(*inner, Type::String));
    } else {
        panic!("Expected Reference type");
    }
}

#[test]
fn test_type_ref_custom() {
    let ty = get_fn_param_type("fn foo(x: &Point) { }");
    if let Type::Reference(inner) = ty {
        if let Type::Custom(name) = *inner {
            assert_eq!(name, "Point");
        } else {
            panic!("Expected Custom inner type");
        }
    } else {
        panic!("Expected Reference type");
    }
}

// ============================================================================
// ARRAY TYPES
// ============================================================================

#[test]
fn test_type_array() {
    let ty = get_fn_param_type("fn foo(x: [i32; 10]) { }");
    if let Type::Array(element, size) = ty {
        // element can be Int32 or Custom("i32")
        assert!(matches!(*element, Type::Int32 | Type::Custom(_)));
        assert_eq!(size, 10);
    } else {
        panic!("Expected Array type, got {:?}", ty);
    }
}

// ============================================================================
// TUPLE TYPES
// ============================================================================

#[test]
fn test_type_tuple_pair() {
    let ty = get_fn_param_type("fn foo(x: (i32, string)) { }");
    if let Type::Tuple(elements) = ty {
        assert_eq!(elements.len(), 2);
    } else {
        panic!("Expected Tuple type, got {:?}", ty);
    }
}

#[test]
fn test_type_tuple_triple() {
    let ty = get_fn_param_type("fn foo(x: (i32, float, bool)) { }");
    if let Type::Tuple(elements) = ty {
        assert_eq!(elements.len(), 3);
    } else {
        panic!("Expected Tuple type");
    }
}

#[test]
fn test_type_unit() {
    // Empty tuple () is the unit type
    let ret = get_fn_return_type("fn foo() -> () { }");
    if let Some(Type::Tuple(elements)) = ret {
        assert!(elements.is_empty());
    } else {
        panic!("Expected unit type (empty tuple), got {:?}", ret);
    }
}

// ============================================================================
// FUNCTION POINTER TYPES
// ============================================================================

#[test]
fn test_type_fn_pointer() {
    let ty = get_fn_param_type("fn foo(f: fn(i32) -> i32) { }");
    if let Type::FunctionPointer {
        params,
        return_type,
    } = ty
    {
        assert_eq!(params.len(), 1);
        assert!(return_type.is_some());
    } else {
        panic!("Expected FunctionPointer type, got {:?}", ty);
    }
}

#[test]
fn test_type_fn_pointer_no_return() {
    let ty = get_fn_param_type("fn foo(f: fn(i32)) { }");
    if let Type::FunctionPointer {
        params,
        return_type,
    } = ty
    {
        assert_eq!(params.len(), 1);
        assert!(return_type.is_none());
    } else {
        panic!("Expected FunctionPointer type");
    }
}

#[test]
fn test_type_fn_pointer_multiple_params() {
    let ty = get_fn_param_type("fn foo(f: fn(i32, string, bool) -> float) { }");
    if let Type::FunctionPointer { params, .. } = ty {
        assert_eq!(params.len(), 3);
    } else {
        panic!("Expected FunctionPointer type");
    }
}

// ============================================================================
// RETURN TYPES
// ============================================================================

#[test]
fn test_return_type_simple() {
    let ret = get_fn_return_type("fn foo() -> i32 { 42 }");
    assert!(ret.is_some());
    if let Some(ty) = ret {
        assert!(matches!(ty, Type::Int32 | Type::Custom(_)));
    }
}

#[test]
fn test_return_type_none() {
    let ret = get_fn_return_type("fn foo() { }");
    assert!(ret.is_none());
}

#[test]
fn test_return_type_vec() {
    let ret = get_fn_return_type("fn foo() -> Vec<i32> { Vec::new() }");
    match ret {
        Some(Type::Vec(_)) | Some(Type::Parameterized(_, _)) => {}
        _ => panic!("Expected Vec or Parameterized return type, got {:?}", ret),
    }
}

#[test]
fn test_return_type_option() {
    let ret = get_fn_return_type("fn foo() -> Option<String> { None }");
    match ret {
        Some(Type::Option(_)) | Some(Type::Parameterized(_, _)) => {}
        _ => panic!(
            "Expected Option or Parameterized return type, got {:?}",
            ret
        ),
    }
}

// ============================================================================
// STRUCT FIELD TYPES
// ============================================================================

#[test]
fn test_field_type_primitive() {
    let ty = get_struct_field_type("struct Point { x: i32 }");
    assert!(matches!(ty, Type::Int32 | Type::Custom(_)));
}

#[test]
fn test_field_type_string() {
    let ty = get_struct_field_type("struct Person { name: string }");
    assert!(matches!(ty, Type::String));
}

#[test]
fn test_field_type_vec() {
    let ty = get_struct_field_type("struct Container { items: Vec<Item> }");
    match ty {
        Type::Vec(_) | Type::Parameterized(_, _) => {}
        _ => panic!("Expected Vec or Parameterized field type, got {:?}", ty),
    }
}

#[test]
fn test_field_type_option() {
    let ty = get_struct_field_type("struct Node { parent: Option<Node> }");
    match ty {
        Type::Option(_) | Type::Parameterized(_, _) => {}
        _ => panic!("Expected Option or Parameterized field type, got {:?}", ty),
    }
}

// ============================================================================
// COMPLEX TYPE COMBINATIONS
// ============================================================================

#[test]
fn test_type_ref_to_vec() {
    let ty = get_fn_param_type("fn foo(x: &Vec<i32>) { }");
    if let Type::Reference(inner) = ty {
        match *inner {
            Type::Vec(_) | Type::Parameterized(_, _) => {}
            _ => panic!("Expected Vec or Parameterized inside Reference"),
        }
    } else {
        panic!("Expected Reference type");
    }
}

#[test]
fn test_type_mut_ref_to_parameterized() {
    let ty = get_fn_param_type("fn foo(x: &mut HashMap<String, i32>) { }");
    if let Type::MutableReference(inner) = ty {
        if let Type::Parameterized(name, _) = *inner {
            assert_eq!(name, "HashMap");
        } else {
            panic!("Expected Parameterized inside MutableReference");
        }
    } else {
        panic!("Expected MutableReference type");
    }
}

// ============================================================================
// TYPE IN GENERICS CONTEXT
// ============================================================================

#[test]
fn test_generic_fn_type_param() {
    let code = "fn foo<T>(x: T) -> T { x }";
    let program = parse_program(code);
    if let Some(Item::Function { decl, .. }) = program.items.first() {
        assert!(!decl.type_params.is_empty());
        // Parameter type should be a type variable T (parsed as Generic or Custom)
        if let Some(param) = decl.parameters.first() {
            match &param.type_ {
                Type::Generic(name) | Type::Custom(name) => assert_eq!(name, "T"),
                _ => panic!(
                    "Expected Generic or Custom type for param, got {:?}",
                    param.type_
                ),
            }
        }
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_generic_fn_multiple_type_params() {
    let code = "fn foo<T, U>(x: T, y: U) -> T { x }";
    let program = parse_program(code);
    if let Some(Item::Function { decl, .. }) = program.items.first() {
        assert_eq!(decl.type_params.len(), 2);
    } else {
        panic!("Expected Function");
    }
}

#[test]
fn test_generic_struct_type_param() {
    let code = "struct Container<T> { value: T }";
    let program = parse_program(code);
    if let Some(Item::Struct { decl, .. }) = program.items.first() {
        assert!(!decl.type_params.is_empty());
    } else {
        panic!("Expected Struct");
    }
}

// ============================================================================
// TYPE BOUNDS (WHERE CLAUSES)
// ============================================================================

#[test]
fn test_type_with_bound() {
    let code = "struct Container<T: Clone> { value: T }";
    let program = parse_program(code);
    if let Some(Item::Struct { decl, .. }) = program.items.first() {
        // Type params should have bounds
        assert!(!decl.type_params.is_empty());
        if let Some(tp) = decl.type_params.first() {
            assert!(!tp.bounds.is_empty());
        }
    } else {
        panic!("Expected Struct");
    }
}

#[test]
fn test_where_clause() {
    let code = r#"
    impl<T> Container<T> where T: Clone {
        fn clone_value(&self) -> T { self.value.clone() }
    }
    "#;
    let program = parse_program(code);
    if let Some(Item::Impl { block, .. }) = program.items.first() {
        assert!(!block.where_clause.is_empty());
    } else {
        panic!("Expected Impl");
    }
}

// ============================================================================
// INFER TYPE
// ============================================================================

#[test]
fn test_type_infer() {
    // The _ type placeholder for inference
    let ty = get_fn_param_type("fn foo(x: _) { }");
    assert!(matches!(ty, Type::Infer));
}

// ============================================================================
// TRAIT OBJECTS
// ============================================================================

#[test]
fn test_type_trait_object() {
    let ty = get_fn_param_type("fn foo(x: dyn Display) { }");
    if let Type::TraitObject(trait_name) = ty {
        assert_eq!(trait_name, "Display");
    } else {
        panic!("Expected TraitObject type, got {:?}", ty);
    }
}

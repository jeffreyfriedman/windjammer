// AST Builders - Ergonomic construction of AST nodes
//
// Provides builder functions to make test code dramatically more concise:
//
// Before: Type::Reference(Box::new(Type::Vec(Box::new(Type::Int))))
// After:  type_ref(type_vec(Type::Int))
//
// Expected impact: 60-80% reduction in test code lines

use super::types::{Type, TypeParam, AssociatedType};
use super::ownership::OwnershipHint;
use super::core::Parameter;

// ============================================================================
// TYPE BUILDERS
// ============================================================================

/// Build primitive type: int
pub fn type_int() -> Type {
    Type::Int
}

/// Build primitive type: float
pub fn type_float() -> Type {
    Type::Float
}

/// Build primitive type: bool
pub fn type_bool() -> Type {
    Type::Bool
}

/// Build primitive type: string
pub fn type_string() -> Type {
    Type::String
}

/// Build custom type: MyType
pub fn type_custom(name: impl Into<String>) -> Type {
    Type::Custom(name.into())
}

/// Build reference type: &T
pub fn type_ref(inner: Type) -> Type {
    Type::Reference(Box::new(inner))
}

/// Build mutable reference type: &mut T
pub fn type_mut_ref(inner: Type) -> Type {
    Type::MutableReference(Box::new(inner))
}

/// Build Vec type: Vec<T>
pub fn type_vec(inner: Type) -> Type {
    Type::Vec(Box::new(inner))
}

/// Build Option type: Option<T>
pub fn type_option(inner: Type) -> Type {
    Type::Option(Box::new(inner))
}

/// Build Result type: Result<T, E>
pub fn type_result(ok: Type, err: Type) -> Type {
    Type::Result(Box::new(ok), Box::new(err))
}

/// Build parameterized type: HashMap<K, V>
pub fn type_parameterized(name: impl Into<String>, args: Vec<Type>) -> Type {
    Type::Parameterized(name.into(), args)
}

/// Build tuple type: (T1, T2, ...)
pub fn type_tuple(elements: Vec<Type>) -> Type {
    Type::Tuple(elements)
}

/// Build array type: [T; N]
pub fn type_array(inner: Type, size: usize) -> Type {
    Type::Array(Box::new(inner), size)
}

/// Build generic type parameter: T
pub fn type_generic(name: impl Into<String>) -> Type {
    Type::Generic(name.into())
}

/// Build associated type: Self::Item
pub fn type_associated(base: impl Into<String>, assoc: impl Into<String>) -> Type {
    Type::Associated(base.into(), assoc.into())
}

/// Build trait object: dyn Trait
pub fn type_trait_object(name: impl Into<String>) -> Type {
    Type::TraitObject(name.into())
}

/// Build type inference placeholder: _
pub fn type_infer() -> Type {
    Type::Infer
}

/// Build int32 type
pub fn type_int32() -> Type {
    Type::Int32
}

/// Build uint type
pub fn type_uint() -> Type {
    Type::Uint
}

// ============================================================================
// PARAMETER BUILDERS
// ============================================================================

/// Build simple parameter with inferred ownership
pub fn param(name: impl Into<String>, type_: Type) -> Parameter {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Inferred,
        is_mutable: false,
    }
}

/// Build reference parameter
pub fn param_ref(name: impl Into<String>, type_: Type) -> Parameter {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Ref,
        is_mutable: false,
    }
}

/// Build mutable reference parameter
pub fn param_mut(name: impl Into<String>, type_: Type) -> Parameter {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Mut,
        is_mutable: true,
    }
}

/// Build owned parameter
pub fn param_owned(name: impl Into<String>, type_: Type) -> Parameter {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Owned,
        is_mutable: false,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_primitives() {
        assert_eq!(type_int(), Type::Int);
        assert_eq!(type_float(), Type::Float);
        assert_eq!(type_bool(), Type::Bool);
        assert_eq!(type_string(), Type::String);
    }

    #[test]
    fn test_type_custom() {
        assert_eq!(type_custom("MyType"), Type::Custom("MyType".to_string()));
    }

    #[test]
    fn test_type_reference() {
        assert_eq!(
            type_ref(Type::Int),
            Type::Reference(Box::new(Type::Int))
        );
    }

    #[test]
    fn test_type_mut_reference() {
        assert_eq!(
            type_mut_ref(Type::Int),
            Type::MutableReference(Box::new(Type::Int))
        );
    }

    #[test]
    fn test_type_vec() {
        assert_eq!(type_vec(Type::Int), Type::Vec(Box::new(Type::Int)));
    }

    #[test]
    fn test_type_option() {
        assert_eq!(
            type_option(Type::String),
            Type::Option(Box::new(Type::String))
        );
    }

    #[test]
    fn test_type_result() {
        assert_eq!(
            type_result(Type::Int, Type::String),
            Type::Result(Box::new(Type::Int), Box::new(Type::String))
        );
    }

    #[test]
    fn test_type_parameterized() {
        assert_eq!(
            type_parameterized("Vec", vec![Type::Int]),
            Type::Parameterized("Vec".to_string(), vec![Type::Int])
        );
    }

    #[test]
    fn test_type_tuple() {
        assert_eq!(
            type_tuple(vec![Type::Int, Type::String]),
            Type::Tuple(vec![Type::Int, Type::String])
        );
    }

    #[test]
    fn test_type_chained() {
        // Complex: Option<&mut Vec<String>>
        let complex = type_option(type_mut_ref(type_vec(Type::String)));

        assert_eq!(
            complex,
            Type::Option(Box::new(Type::MutableReference(Box::new(
                Type::Vec(Box::new(Type::String))
            ))))
        );
    }

    #[test]
    fn test_param_simple() {
        let p = param("x", Type::Int);

        assert_eq!(p.name, "x");
        assert_eq!(p.type_, Type::Int);
        assert_eq!(p.ownership, OwnershipHint::Inferred);
        assert!(!p.is_mutable);
    }

    #[test]
    fn test_param_ref() {
        let p = param_ref("x", Type::Int);

        assert_eq!(p.ownership, OwnershipHint::Ref);
        assert!(!p.is_mutable);
    }

    #[test]
    fn test_param_mut() {
        let p = param_mut("x", Type::Int);

        assert_eq!(p.ownership, OwnershipHint::Mut);
        assert!(p.is_mutable);
    }

    #[test]
    fn test_param_owned() {
        let p = param_owned("data", Type::String);

        assert_eq!(p.ownership, OwnershipHint::Owned);
        assert!(!p.is_mutable);
    }

    #[test]
    fn test_ergonomic_example() {
        // Before builders (7 lines):
        // Parameter {
        //     name: "data".to_string(),
        //     pattern: None,
        //     type_: Type::Reference(Box::new(Type::Vec(Box::new(Type::Int)))),
        //     ownership: OwnershipHint::Ref,
        //     is_mutable: false,
        // }

        // After builders (1 line):
        let param = param_ref("data", type_vec(Type::Int));

        assert_eq!(param.name, "data");
        assert_eq!(
            param.type_,
            Type::Vec(Box::new(Type::Int))
        );
        assert_eq!(param.ownership, OwnershipHint::Ref);
    }
}


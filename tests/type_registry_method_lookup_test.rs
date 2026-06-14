#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Test: Type Registry with Method Signature Lookup
///
/// This test validates the enhanced type registry that stores method signatures
/// organized by receiver type, enabling proper type-based method resolution.
///
/// Goal: Replace ALL hard-coded string matching with actual type lookups.
use std::collections::HashMap;

// Mock structures for testing (will be replaced with actual compiler types)
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum Type {
    Custom(String),
    Reference(Box<Type>),
    Vec(Box<Type>),
}

#[derive(Debug, Clone)]
struct MethodSignature {
    receiver_type: String,
    method_name: String,
    param_types: Vec<Type>,
    #[allow(dead_code)]
    return_type: Option<Type>,
}

struct TypeRegistry {
    /// NEW: Method signatures organized by receiver type
    /// HashMap<ReceiverType, HashMap<MethodName, Signature>>
    method_signatures_by_type: HashMap<String, HashMap<String, MethodSignature>>,
}

impl TypeRegistry {
    fn new() -> Self {
        Self {
            method_signatures_by_type: HashMap::new(),
        }
    }

    /// Register a method signature for a given type
    fn register_method(&mut self, sig: MethodSignature) {
        self.method_signatures_by_type
            .entry(sig.receiver_type.clone())
            .or_default()
            .insert(sig.method_name.clone(), sig);
    }

    /// Look up a method signature by receiver type and method name
    fn lookup_method(&self, receiver_type: &str, method_name: &str) -> Option<&MethodSignature> {
        self.method_signatures_by_type
            .get(receiver_type)?
            .get(method_name)
    }
}

#[test]
fn test_register_and_lookup_method() {
    let mut registry = TypeRegistry::new();

    // Register Vec::push signature
    let push_sig = MethodSignature {
        receiver_type: "Vec".to_string(),
        method_name: "push".to_string(),
        param_types: vec![Type::Custom("T".to_string())], // Owned T
        return_type: None,
    };
    registry.register_method(push_sig.clone());

    // Lookup should find it
    let found = registry.lookup_method("Vec", "push");
    assert!(found.is_some(), "Should find Vec::push");

    let sig = found.unwrap();
    assert_eq!(sig.method_name, "push");
    assert_eq!(sig.param_types.len(), 1);
}

#[test]
fn test_lookup_nonexistent_method() {
    let registry = TypeRegistry::new();

    // Lookup non-existent method
    let found = registry.lookup_method("Vec", "nonexistent");
    assert!(found.is_none(), "Should not find nonexistent method");
}

#[test]
fn test_multiple_methods_same_type() {
    let mut registry = TypeRegistry::new();

    // Register multiple String methods
    registry.register_method(MethodSignature {
        receiver_type: "String".to_string(),
        method_name: "contains".to_string(),
        param_types: vec![Type::Reference(Box::new(Type::Custom("str".to_string())))],
        return_type: Some(Type::Custom("bool".to_string())),
    });

    registry.register_method(MethodSignature {
        receiver_type: "String".to_string(),
        method_name: "push".to_string(),
        param_types: vec![Type::Custom("char".to_string())],
        return_type: None,
    });

    // Should find both
    assert!(registry.lookup_method("String", "contains").is_some());
    assert!(registry.lookup_method("String", "push").is_some());
}

#[test]
fn test_stdlib_signature_preload() {
    fn preload_stdlib_signatures() -> TypeRegistry {
        let mut registry = TypeRegistry::new();

        // Vec<T> methods
        registry.register_method(MethodSignature {
            receiver_type: "Vec".to_string(),
            method_name: "push".to_string(),
            param_types: vec![Type::Custom("T".to_string())], // Owned
            return_type: None,
        });

        registry.register_method(MethodSignature {
            receiver_type: "Vec".to_string(),
            method_name: "contains".to_string(),
            param_types: vec![Type::Reference(Box::new(Type::Custom("T".to_string())))], // &T
            return_type: Some(Type::Custom("bool".to_string())),
        });

        // String methods
        registry.register_method(MethodSignature {
            receiver_type: "String".to_string(),
            method_name: "contains".to_string(),
            param_types: vec![Type::Reference(Box::new(Type::Custom("str".to_string())))], // &str
            return_type: Some(Type::Custom("bool".to_string())),
        });

        registry.register_method(MethodSignature {
            receiver_type: "String".to_string(),
            method_name: "push_str".to_string(),
            param_types: vec![Type::Reference(Box::new(Type::Custom("str".to_string())))], // &str
            return_type: None,
        });

        registry
    }

    let registry = preload_stdlib_signatures();

    // Verify Vec methods
    let vec_push = registry.lookup_method("Vec", "push").unwrap();
    assert_eq!(vec_push.param_types.len(), 1);
    assert!(matches!(vec_push.param_types[0], Type::Custom(_))); // Owned T

    let vec_contains = registry.lookup_method("Vec", "contains").unwrap();
    assert!(matches!(vec_contains.param_types[0], Type::Reference(_))); // &T

    // Verify String methods
    let string_contains = registry.lookup_method("String", "contains").unwrap();
    assert!(matches!(string_contains.param_types[0], Type::Reference(_))); // &str
}

#[test]
fn test_should_add_ref_with_signature() {
    // This test demonstrates the proper logic for should_add_ref
    // based on ACTUAL type signatures, not hard-coded method names

    let _registry = TypeRegistry::new();
    // Imagine we have Vec::push registered with param_type = Owned T

    // Example decision logic:
    // if param_type is Owned T AND arg_type is T → DON'T add &
    // if param_type is &T AND arg_type is T → ADD &
    // if param_type is &str AND arg_type is String → ADD & (String → &str)

    // This is the PROPER way - based on types, not method names!

    let should_add_ref_proper = |param_type: &Type, arg_type: &Type| -> bool {
        match (param_type, arg_type) {
            // Param wants &str, arg is String → add &
            (Type::Reference(inner), Type::Custom(s))
                if matches!(&**inner, Type::Custom(str_ty) if str_ty == "str") && s == "String" =>
            {
                true
            }

            // Param wants &T, arg is T (non-Copy) → add &
            (Type::Reference(inner), arg_ty) if **inner == *arg_ty => true,

            // Otherwise, don't add &
            _ => false,
        }
    };

    // Test cases
    assert!(
        should_add_ref_proper(
            &Type::Reference(Box::new(Type::Custom("str".to_string()))),
            &Type::Custom("String".to_string())
        ),
        "String → &str should add &"
    );

    assert!(
        !should_add_ref_proper(
            &Type::Custom("T".to_string()), // Owned T
            &Type::Custom("T".to_string())
        ),
        "Owned T param with T arg should NOT add &"
    );
}

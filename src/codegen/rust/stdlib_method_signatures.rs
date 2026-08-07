//! Preloaded [`MethodSignature`] tables for core std-like types (`Vec`, `String`, `HashMap`).

use crate::analyzer::OwnershipMode;
use crate::codegen::rust::method_signature::MethodSignature;
use crate::parser::Type;
use std::collections::HashMap;

/// Initialize stdlib method signatures (Vec, String, HashMap, etc.).
/// Replaces hard-coded method name heuristics with proper type-based lookup.
pub(in crate::codegen::rust) fn init_stdlib_method_signatures(
) -> HashMap<String, HashMap<String, MethodSignature>> {
    let mut map = HashMap::new();

    // Vec<T> methods
    let mut vec_methods = HashMap::new();
    vec_methods.insert(
        "push".to_string(),
        MethodSignature::new(
            "Vec",
            "push",
            vec![Type::Custom("T".to_string())], // Owned T
            vec![OwnershipMode::Owned],
            None,
            true,
        ),
    );
    vec_methods.insert(
        "contains".to_string(),
        MethodSignature::new(
            "Vec",
            "contains",
            vec![Type::Reference(Box::new(Type::Custom("T".to_string())))], // &T
            vec![OwnershipMode::Borrowed],
            Some(Type::Bool),
            true,
        ),
    );
    vec_methods.insert(
        "insert".to_string(),
        MethodSignature::new(
            "Vec",
            "insert",
            vec![
                Type::Custom("usize".to_string()),
                Type::Custom("T".to_string()),
            ], // index: usize, element: T
            vec![OwnershipMode::Owned, OwnershipMode::Owned],
            None,
            true,
        ),
    );
    vec_methods.insert(
        "remove".to_string(),
        MethodSignature::new(
            "Vec",
            "remove",
            vec![Type::Custom("usize".to_string())], // index: usize (owned, not &usize)
            vec![OwnershipMode::Owned],
            Some(Type::Custom("T".to_string())),
            true,
        ),
    );
    vec_methods.insert(
        "get".to_string(),
        MethodSignature::new(
            "Vec",
            "get",
            vec![Type::Custom("usize".to_string())],
            vec![OwnershipMode::Owned],
            Some(Type::Option(Box::new(Type::Reference(Box::new(Type::Custom(
                "T".to_string(),
            )))))),
            true,
        ),
    );
    vec_methods.insert(
        "get_mut".to_string(),
        MethodSignature::new(
            "Vec",
            "get_mut",
            vec![Type::Custom("usize".to_string())],
            vec![OwnershipMode::Owned],
            Some(Type::Option(Box::new(Type::MutableReference(Box::new(
                Type::Custom("T".to_string()),
            ))))),
            true,
        ),
    );
    vec_methods.insert(
        "join".to_string(),
        MethodSignature::new(
            "Vec",
            "join",
            vec![Type::Reference(Box::new(Type::Custom("str".to_string())))],
            vec![OwnershipMode::Borrowed],
            Some(Type::String),
            true,
        ),
    );
    vec_methods.insert(
        "extend_from_slice".to_string(),
        MethodSignature::new(
            "Vec",
            "extend_from_slice",
            vec![Type::Reference(Box::new(Type::Custom("T".to_string())))],
            vec![OwnershipMode::Borrowed],
            None,
            true,
        ),
    );
    map.insert("Vec".to_string(), vec_methods);

    // String methods
    let mut string_methods = HashMap::new();
    string_methods.insert(
        "new".to_string(),
        MethodSignature::new(
            "String",
            "new",
            vec![],
            vec![],
            Some(Type::String),
            false, // static constructor
        ),
    );
    string_methods.insert(
        "contains".to_string(),
        MethodSignature::new(
            "String",
            "contains",
            vec![Type::Reference(Box::new(Type::Custom("str".to_string())))], // &str
            vec![OwnershipMode::Borrowed],
            Some(Type::Bool),
            true,
        ),
    );
    string_methods.insert(
        "push_str".to_string(),
        MethodSignature::new(
            "String",
            "push_str",
            vec![Type::Reference(Box::new(Type::Custom("str".to_string())))], // &str
            vec![OwnershipMode::Borrowed],
            None,
            true,
        ),
    );
    string_methods.insert(
        "starts_with".to_string(),
        MethodSignature::new(
            "String",
            "starts_with",
            vec![Type::Reference(Box::new(Type::Custom("str".to_string())))], // &str
            vec![OwnershipMode::Borrowed],
            Some(Type::Bool),
            true,
        ),
    );
    string_methods.insert(
        "ends_with".to_string(),
        MethodSignature::new(
            "String",
            "ends_with",
            vec![Type::Reference(Box::new(Type::Custom("str".to_string())))], // &str
            vec![OwnershipMode::Borrowed],
            Some(Type::Bool),
            true,
        ),
    );
    map.insert("String".to_string(), string_methods);

    // HashMap<K, V> methods
    let mut hashmap_methods = HashMap::new();
    hashmap_methods.insert(
        "get".to_string(),
        MethodSignature::new(
            "HashMap",
            "get",
            vec![Type::Reference(Box::new(Type::Custom("K".to_string())))], // &K
            vec![OwnershipMode::Borrowed],
            Some(Type::Option(Box::new(Type::Reference(Box::new(
                Type::Custom("V".to_string()),
            ))))),
            true,
        ),
    );
    hashmap_methods.insert(
        "insert".to_string(),
        MethodSignature::new(
            "HashMap",
            "insert",
            vec![Type::Custom("K".to_string()), Type::Custom("V".to_string())], // K, V (both owned)
            vec![OwnershipMode::Owned, OwnershipMode::Owned],
            Some(Type::Option(Box::new(Type::Custom("V".to_string())))),
            true,
        ),
    );
    hashmap_methods.insert(
        "contains_key".to_string(),
        MethodSignature::new(
            "HashMap",
            "contains_key",
            vec![Type::Reference(Box::new(Type::Custom("K".to_string())))], // &K
            vec![OwnershipMode::Borrowed],
            Some(Type::Bool),
            true,
        ),
    );
    hashmap_methods.insert(
        "remove".to_string(),
        MethodSignature::new(
            "HashMap",
            "remove",
            vec![Type::Reference(Box::new(Type::Custom("K".to_string())))], // &K
            vec![OwnershipMode::Borrowed],
            Some(Type::Option(Box::new(Type::Custom("V".to_string())))),
            true,
        ),
    );
    map.insert("Map".to_string(), hashmap_methods.clone());
    map.insert("OrderedMap".to_string(), hashmap_methods.clone());
    map.insert("SlotMap".to_string(), hashmap_methods.clone());
    map.insert("ConcurrentMap".to_string(), hashmap_methods.clone());
    map.insert("HashMap".to_string(), hashmap_methods);

    // HashSet<T> / BTreeSet<T> — lookup args are `&T` (Borrowed), not owned.
    let mut hashset_methods = HashMap::new();
    hashset_methods.insert(
        "contains".to_string(),
        MethodSignature::new(
            "HashSet",
            "contains",
            vec![Type::Reference(Box::new(Type::Custom("T".to_string())))],
            vec![OwnershipMode::Borrowed],
            Some(Type::Bool),
            true,
        ),
    );
    hashset_methods.insert(
        "remove".to_string(),
        MethodSignature::new(
            "HashSet",
            "remove",
            vec![Type::Reference(Box::new(Type::Custom("T".to_string())))],
            vec![OwnershipMode::Borrowed],
            Some(Type::Bool),
            true,
        ),
    );
    hashset_methods.insert(
        "insert".to_string(),
        MethodSignature::new(
            "HashSet",
            "insert",
            vec![Type::Custom("T".to_string())],
            vec![OwnershipMode::Owned],
            Some(Type::Bool),
            true,
        ),
    );
    map.insert("BTreeSet".to_string(), hashset_methods.clone());
    // Retarget cloned entries to BTreeSet receiver before inserting HashSet.
    if let Some(bt) = map.get_mut("BTreeSet") {
        for sig in bt.values_mut() {
            sig.receiver_type = "BTreeSet".to_string();
        }
    }
    map.insert("HashSet".to_string(), hashset_methods);

    map
}

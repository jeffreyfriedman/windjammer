//! Type conversion from Windjammer types to Rust types

use crate::parser::Type;

/// Convert a Windjammer type to its Rust equivalent
pub fn type_to_rust(type_: &Type) -> String {
    match type_ {
        Type::Int => "i64".to_string(),
        Type::Int32 => "i32".to_string(),
        Type::Uint => "u64".to_string(),
        Type::Float => "f64".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "String".to_string(),
        Type::Custom(name) => {
            // Convert Windjammer module.Type syntax to Rust module::Type
            name.replace('.', "::")
        }
        Type::Generic(name) => name.clone(), // Type parameter: T -> T
        Type::Associated(base, assoc_name) => {
            // Associated type: Self::Item -> Self::Item, T::Output -> T::Output
            format!("{}::{}", base, assoc_name)
        }
        Type::TraitObject(trait_name) => {
            // Trait object: dyn Trait -> Box<dyn Trait>
            // Note: Windjammer automatically boxes trait objects for convenience
            format!("Box<dyn {}>", trait_name)
        }
        Type::Parameterized(base, args) => {
            // Generic type: Vec<T> -> Vec<T>, HashMap<K, V> -> HashMap<K, V>
            format!(
                "{}<{}>",
                base,
                args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")
            )
        }
        Type::Option(inner) => format!("Option<{}>", type_to_rust(inner)),
        Type::Result(ok, err) => format!("Result<{}, {}>", type_to_rust(ok), type_to_rust(err)),
        Type::Vec(inner) => format!("Vec<{}>", type_to_rust(inner)),
        Type::Array(inner, size) => format!("[{}; {}]", type_to_rust(inner), size),
        Type::Reference(inner) => {
            // Special case: &[T] (slice) vs &Vec<T>
            if let Type::Vec(elem) = &**inner {
                format!("&[{}]", type_to_rust(elem))
            // Special case: &[T; N] stays as &[T; N]
            } else if let Type::Array(elem, size) = &**inner {
                format!("&[{}; {}]", type_to_rust(elem), size)
            // Special case: &str instead of &String (more idiomatic Rust)
            } else if matches!(**inner, Type::String) {
                "&str".to_string()
            // Special case: &dyn Trait (don't box when already a reference)
            } else if let Type::TraitObject(trait_name) = &**inner {
                format!("&dyn {}", trait_name)
            } else {
                format!("&{}", type_to_rust(inner))
            }
        }
        Type::MutableReference(inner) => {
            // Special case: &mut [T] (mutable slice) vs &mut Vec<T>
            if let Type::Vec(elem) = &**inner {
                format!("&mut [{}]", type_to_rust(elem))
            // Special case: &mut dyn Trait (don't box when already a reference)
            } else if let Type::TraitObject(trait_name) = &**inner {
                format!("&mut dyn {}", trait_name)
            } else {
                format!("&mut {}", type_to_rust(inner))
            }
        }
        Type::Tuple(types) => {
            let rust_types: Vec<String> = types.iter().map(type_to_rust).collect();
            format!("({})", rust_types.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
        assert_eq!(type_to_rust(&Type::Int), "i64");
        assert_eq!(type_to_rust(&Type::String), "String");
        assert_eq!(type_to_rust(&Type::Bool), "bool");
    }

    #[test]
    fn test_reference_types() {
        assert_eq!(
            type_to_rust(&Type::Reference(Box::new(Type::String))),
            "&str"
        );
        assert_eq!(type_to_rust(&Type::Reference(Box::new(Type::Int))), "&i64");
    }

    #[test]
    fn test_generic_types() {
        assert_eq!(type_to_rust(&Type::Vec(Box::new(Type::Int))), "Vec<i64>");
        assert_eq!(
            type_to_rust(&Type::Option(Box::new(Type::String))),
            "Option<String>"
        );
    }
}

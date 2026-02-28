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
            // Special case: Signal (without type params) -> windjammer_ui::reactivity::Signal
            if name == "Signal" {
                "windjammer_ui::reactivity::Signal".to_string()
            // Special case: "string" as custom type -> String (for type aliases)
            } else if name == "string" {
                "String".to_string()
            } else {
                // Convert Windjammer module.Type syntax to Rust module::Type
                name.replace('.', "::")
            }
        }
        Type::Generic(name) => name.clone(), // Type parameter: T -> T
        Type::Associated(base, assoc_name) => {
            // Associated type: Self::Item -> Self::Item, T::Output -> T::Output
            format!("{}::{}", base, assoc_name)
        }
        Type::TraitObject(trait_name) => {
            // THE WINDJAMMER WAY: Trait objects generate just `dyn Trait`.
            // When used inside Box<>, Vec<>, etc., the container handles the boxing.
            // When used as a bare type (e.g., field type without Box), the user
            // should use Box<dyn Trait> explicitly in Windjammer source.
            // This prevents double-boxing: Box<dyn System> was becoming Box<Box<dyn System>>.
            format!("dyn {}", trait_name)
        }
        Type::ImplTrait(trait_name) => {
            // THE WINDJAMMER WAY: `trait TraitName` in type position.
            // As a bare type (param or return), use static dispatch: `impl Trait`.
            // Context-dependent dynamic dispatch is handled by the surrounding type
            // (Reference, Vec, Box, Option) which check for ImplTrait in their arms.
            format!("impl {}", trait_name)
        }
        Type::Parameterized(base, args) => {
            // Special case: Signal<T> -> windjammer_ui::reactivity::Signal<T>
            if base == "Signal" {
                let rust_args: Vec<String> = args.iter().map(type_to_rust).collect();
                format!(
                    "windjammer_ui::reactivity::Signal<{}>",
                    rust_args.join(", ")
                )
            // Dynamic dispatch: Box<trait Foo> -> Box<dyn Foo>
            } else if base == "Box" && args.len() == 1 {
                if let Type::ImplTrait(trait_name) = &args[0] {
                    format!("Box<dyn {}>", trait_name)
                } else {
                    format!("Box<{}>", type_to_rust(&args[0]))
                }
            } else {
                // Generic type: Vec<T> -> Vec<T>, HashMap<K, V> -> HashMap<K, V>
                format!(
                    "{}<{}>",
                    base,
                    args.iter().map(type_to_rust).collect::<Vec<_>>().join(", ")
                )
            }
        }
        Type::Option(inner) => {
            // Dynamic dispatch: Option<trait Foo> -> Option<Box<dyn Foo>>
            if let Type::ImplTrait(trait_name) = &**inner {
                format!("Option<Box<dyn {}>>", trait_name)
            } else {
                format!("Option<{}>", type_to_rust(inner))
            }
        }
        Type::Result(ok, err) => {
            // Special case: Result<T, string> -> Result<T, String>
            // In Windjammer, `string` is the type, but Rust uses `String` for owned strings
            let ok_rust = type_to_rust(ok);
            let err_rust = type_to_rust(err);
            format!("Result<{}, {}>", ok_rust, err_rust)
        }
        Type::Vec(inner) => {
            // Dynamic dispatch: Vec<trait Foo> -> Vec<Box<dyn Foo>> (heterogeneous collection)
            if let Type::ImplTrait(trait_name) = &**inner {
                format!("Vec<Box<dyn {}>>", trait_name)
            } else {
                format!("Vec<{}>", type_to_rust(inner))
            }
        }
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
            // Dynamic dispatch: &trait Foo -> &dyn Foo
            } else if let Type::ImplTrait(trait_name) = &**inner {
                format!("&dyn {}", trait_name)
            } else {
                format!("&{}", type_to_rust(inner))
            }
        }
        Type::MutableReference(inner) => {
            // Special case: &mut dyn Trait (don't box when already a reference)
            if let Type::TraitObject(trait_name) = &**inner {
                format!("&mut dyn {}", trait_name)
            // Dynamic dispatch: &mut trait Foo -> &mut dyn Foo
            } else if let Type::ImplTrait(trait_name) = &**inner {
                format!("&mut dyn {}", trait_name)
            } else {
                // FIXED: Don't convert &mut Vec<T> to &mut [T] - slices can't push/pop!
                // Always preserve the exact inner type with &mut prefix
                format!("&mut {}", type_to_rust(inner))
            }
        }
        Type::RawPointer { mutable, pointee } => {
            // TDD: Raw pointer types for FFI
            // *const T -> *const T, *mut T -> *mut T
            if *mutable {
                format!("*mut {}", type_to_rust(pointee))
            } else {
                format!("*const {}", type_to_rust(pointee))
            }
        }
        Type::Tuple(types) => {
            let rust_types: Vec<String> = types.iter().map(type_to_rust).collect();
            format!("({})", rust_types.join(", "))
        }
        Type::Infer => "_".to_string(), // Type inference placeholder
        Type::FunctionPointer {
            params,
            return_type,
        } => {
            // TDD FIX: IDIOMATIC WINDJAMMER - Apply ownership inference to function pointer params
            // fn(string, i32) → fn(&String, i32) (string is borrowed by default)
            // fn(vec: Vec<T>) → fn(Vec<T>) (explicit type, keep as-is)
            let param_strs: Vec<String> = params.iter().map(|ty| {
                match ty {
                    // Idiomatic Windjammer: string parameters are borrowed
                    Type::String => "&String".to_string(),
                    Type::Custom(name) if name == "string" => "&String".to_string(),
                    // Already explicit references - keep as-is
                    Type::Reference(_) | Type::MutableReference(_) => type_to_rust(ty),
                    // Copy types - pass by value
                    Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => type_to_rust(ty),
                    Type::Custom(name) if matches!(name.as_str(), "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool" | "char" | "usize" | "isize") => type_to_rust(ty),
                    // Everything else - keep as-is (explicit types are respected)
                    _ => type_to_rust(ty),
                }
            }).collect();
            if let Some(ret) = return_type {
                format!("fn({}) -> {}", param_strs.join(", "), type_to_rust(ret))
            } else {
                format!("fn({})", param_strs.join(", "))
            }
        }
    }
}

/// Check if a type contains any references (including nested in Option, Result, etc.)
pub fn type_contains_reference(type_: &Type) -> bool {
    match type_ {
        Type::Reference(_) | Type::MutableReference(_) => true,
        // TDD: Raw pointers are NOT references (different lifetime rules)
        Type::RawPointer { .. } => false,
        Type::Option(inner) => type_contains_reference(inner),
        Type::Result(ok, err) => type_contains_reference(ok) || type_contains_reference(err),
        Type::Vec(inner) => type_contains_reference(inner),
        Type::Array(inner, _) => type_contains_reference(inner),
        Type::Tuple(types) => types.iter().any(type_contains_reference),
        Type::Parameterized(_, args) => args.iter().any(type_contains_reference),
        _ => false,
    }
}

/// Convert a Windjammer type to Rust, adding lifetime 'a to all references.
/// Used when the function signature requires explicit lifetime annotations.
pub fn type_to_rust_with_lifetime(type_: &Type) -> String {
    match type_ {
        Type::Reference(inner) => {
            // Special case: &Vec<T> → &'a [T]
            if let Type::Vec(elem) = &**inner {
                format!("&'a [{}]", type_to_rust_with_lifetime(elem))
            // Special case: &[T; N]
            } else if let Type::Array(elem, size) = &**inner {
                format!("&'a [{}; {}]", type_to_rust_with_lifetime(elem), size)
            // Special case: &String → &'a str
            } else if matches!(**inner, Type::String) {
                "&'a str".to_string()
            // Dynamic dispatch references
            } else if let Type::TraitObject(trait_name) = &**inner {
                format!("&'a dyn {}", trait_name)
            } else if let Type::ImplTrait(trait_name) = &**inner {
                format!("&'a dyn {}", trait_name)
            } else {
                format!("&'a {}", type_to_rust_with_lifetime(inner))
            }
        }
        Type::MutableReference(inner) => {
            if let Type::TraitObject(trait_name) = &**inner {
                format!("&'a mut dyn {}", trait_name)
            } else if let Type::ImplTrait(trait_name) = &**inner {
                format!("&'a mut dyn {}", trait_name)
            } else {
                format!("&'a mut {}", type_to_rust_with_lifetime(inner))
            }
        }
        // TDD: Raw pointers don't have lifetimes (unsafe, FFI)
        Type::RawPointer { mutable, pointee } => {
            if *mutable {
                format!("*mut {}", type_to_rust(pointee))
            } else {
                format!("*const {}", type_to_rust(pointee))
            }
        }
        // For container types, recurse to add lifetime to nested references
        Type::Option(inner) => format!("Option<{}>", type_to_rust_with_lifetime(inner)),
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            type_to_rust_with_lifetime(ok),
            type_to_rust_with_lifetime(err)
        ),
        Type::Tuple(types) => {
            let rust_types: Vec<String> = types.iter().map(type_to_rust_with_lifetime).collect();
            format!("({})", rust_types.join(", "))
        }
        Type::Parameterized(base, args) => {
            let rust_args: Vec<String> = args.iter().map(type_to_rust_with_lifetime).collect();
            format!("{}<{}>", base, rust_args.join(", "))
        }
        // Non-reference types: delegate to standard conversion
        _ => type_to_rust(type_),
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

    // =========================================================================
    // ImplTrait dispatch inference tests
    // =========================================================================

    #[test]
    fn test_impl_trait_bare_generates_static_dispatch() {
        // trait Foo in bare position -> impl Foo (static dispatch)
        assert_eq!(
            type_to_rust(&Type::ImplTrait("Describable".to_string())),
            "impl Describable"
        );
    }

    #[test]
    fn test_impl_trait_in_reference_generates_dyn() {
        // &trait Foo -> &dyn Foo (dynamic dispatch)
        assert_eq!(
            type_to_rust(&Type::Reference(Box::new(Type::ImplTrait(
                "Describable".to_string()
            )))),
            "&dyn Describable"
        );
    }

    #[test]
    fn test_impl_trait_in_mut_reference_generates_dyn() {
        // &mut trait Foo -> &mut dyn Foo
        assert_eq!(
            type_to_rust(&Type::MutableReference(Box::new(Type::ImplTrait(
                "Resettable".to_string()
            )))),
            "&mut dyn Resettable"
        );
    }

    #[test]
    fn test_impl_trait_in_vec_generates_boxed_dyn() {
        // Vec<trait Foo> -> Vec<Box<dyn Foo>>
        assert_eq!(
            type_to_rust(&Type::Vec(Box::new(Type::ImplTrait(
                "Describable".to_string()
            )))),
            "Vec<Box<dyn Describable>>"
        );
    }

    #[test]
    fn test_impl_trait_in_option_generates_boxed_dyn() {
        // Option<trait Foo> -> Option<Box<dyn Foo>>
        assert_eq!(
            type_to_rust(&Type::Option(Box::new(Type::ImplTrait(
                "Handler".to_string()
            )))),
            "Option<Box<dyn Handler>>"
        );
    }

    #[test]
    fn test_impl_trait_in_box_generates_dyn() {
        // Box<trait Foo> -> Box<dyn Foo>
        assert_eq!(
            type_to_rust(&Type::Parameterized(
                "Box".to_string(),
                vec![Type::ImplTrait("System".to_string())]
            )),
            "Box<dyn System>"
        );
    }
}

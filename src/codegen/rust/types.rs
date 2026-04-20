//! Type conversion from Windjammer types to Rust types

use crate::parser::Type;

/// Windjammer text types: owned Rust form is `String`; borrowed parameters use `&str`.
/// After parser normalization, all string types become `Type::String`. The `Custom` fallback
/// handles legacy metadata from pre-normalization builds.
#[inline]
pub fn is_windjammer_text_type(t: &Type) -> bool {
    matches!(t, Type::String)
        || matches!(
            t,
            Type::Custom(name) if matches!(name.as_str(), "string" | "String" | "str")
        )
}

/// In owned container slots (`Vec<string>`, `Option<string>`, `HashMap` values, etc.),
/// the Windjammer `string` type becomes Rust `String`. The `Custom("str")` fallback
/// handles legacy metadata from pre-normalization builds.
fn type_to_rust_mapped_owned_str_slot(type_: &Type, map_custom: &dyn Fn(&str) -> String) -> String {
    match type_ {
        Type::Custom(name) if name == "str" => "String".to_string(),
        _ => type_to_rust_mapped(type_, map_custom),
    }
}

/// Whether generic args to this container should use [`type_to_rust_mapped_owned_str_slot`] for
/// `str` (e.g. `HashMap<str, str>` → `HashMap<String, String>`).
fn parameterized_base_uses_owned_str_slots(base: &str) -> bool {
    matches!(base, "HashMap" | "BTreeMap" | "BTreeSet" | "HashSet")
}

/// Convert a Windjammer type to its Rust equivalent
pub fn type_to_rust(type_: &Type) -> String {
    type_to_rust_mapped(type_, &|s| s.to_string())
}

/// Like [`type_to_rust_mapped`], but skips stdlib type mappings (e.g., Map → HashMap) when the
/// type name matches a user-defined import alias.
pub fn type_to_rust_mapped_with_aliases(
    type_: &Type,
    map_custom: &dyn Fn(&str) -> String,
    import_aliases: &std::collections::HashSet<String>,
) -> String {
    match type_ {
        Type::Parameterized(base, args) if import_aliases.contains(base.as_str()) => {
            let rust_args: Vec<String> = args
                .iter()
                .map(|a| type_to_rust_mapped_with_aliases(a, map_custom, import_aliases))
                .collect();
            let base_rust = map_custom(base).replace('.', "::");
            format!("{}<{}>", base_rust, rust_args.join(", "))
        }
        Type::Custom(name) if import_aliases.contains(name.as_str()) => {
            map_custom(name).replace('.', "::")
        }
        _ => type_to_rust_mapped(type_, map_custom),
    }
}

/// Like [`type_to_rust`], but applies `map_custom` to non-special `Custom` names and to
/// `Parameterized` base names (after Signal/Box special cases) before `module.Type` → `module::Type`.
pub fn type_to_rust_mapped(type_: &Type, map_custom: &dyn Fn(&str) -> String) -> String {
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
            // Bare `str` (params, returns): Rust `&str`. Owned slots use
            // [`type_to_rust_mapped_owned_str_slot`]; `Box<str>` is special-cased.
            } else if name == "str" {
                "&str".to_string()
            } else {
                // Convert Windjammer module.Type syntax to Rust module::Type
                map_custom(name).replace('.', "::")
            }
        }
        Type::Generic(name) => name.clone(), // Type parameter: T -> T
        Type::Associated(base, assoc_name) => {
            // Parser may classify some module paths as associated types (`ffi::GpuVertex` before
            // type_parser disambiguation, or edge cases). Apply the same `map_custom` pipeline as
            // `Custom` so `qualify_parent_child_external_path` can insert `ffi::api::...`.
            let dotted = format!("{}::{}", base, assoc_name).replace("::", ".");
            map_custom(&dotted).replace('.', "::")
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
                let rust_args: Vec<String> = args
                    .iter()
                    .map(|a| type_to_rust_mapped(a, map_custom))
                    .collect();
                format!(
                    "windjammer_ui::reactivity::Signal<{}>",
                    rust_args.join(", ")
                )
            // Dynamic dispatch: Box<trait Foo> -> Box<dyn Foo>
            } else if base == "Box" && args.len() == 1 {
                if let Type::ImplTrait(trait_name) = &args[0] {
                    format!("Box<dyn {}>", trait_name)
                } else if matches!(&args[0], Type::Custom(name) if name == "str") {
                    "Box<str>".to_string()
                } else {
                    format!("Box<{}>", type_to_rust_mapped(&args[0], map_custom))
                }
            // Windjammer stdlib Map<K,V> -> std::collections::HashMap<K,V>
            } else if base == "Map" && args.len() == 2 {
                let rust_args: Vec<String> = args
                    .iter()
                    .map(|a| type_to_rust_mapped_owned_str_slot(a, map_custom))
                    .collect();
                format!("HashMap<{}>", rust_args.join(", "))
            } else {
                // Generic type: Vec<T> -> Vec<T>, HashMap<K, V> -> HashMap<K, V>
                let base_rust = map_custom(base).replace('.', "::");
                let map_arg = |a: &Type| -> String {
                    if parameterized_base_uses_owned_str_slots(base) {
                        type_to_rust_mapped_owned_str_slot(a, map_custom)
                    } else {
                        type_to_rust_mapped(a, map_custom)
                    }
                };
                format!(
                    "{}<{}>",
                    base_rust,
                    args.iter().map(map_arg).collect::<Vec<_>>().join(", ")
                )
            }
        }
        Type::Option(inner) => {
            // Dynamic dispatch: Option<trait Foo> -> Option<Box<dyn Foo>>
            if let Type::ImplTrait(trait_name) = &**inner {
                format!("Option<Box<dyn {}>>", trait_name)
            } else {
                format!(
                    "Option<{}>",
                    type_to_rust_mapped_owned_str_slot(inner, map_custom)
                )
            }
        }
        Type::Result(ok, err) => {
            // Special case: Result<T, string> -> Result<T, String>
            // In Windjammer, `string` is the type, but Rust uses `String` for owned strings
            let ok_rust = type_to_rust_mapped_owned_str_slot(ok, map_custom);
            let err_rust = type_to_rust_mapped_owned_str_slot(err, map_custom);
            format!("Result<{}, {}>", ok_rust, err_rust)
        }
        Type::Vec(inner) => {
            // Dynamic dispatch: Vec<trait Foo> -> Vec<Box<dyn Foo>> (heterogeneous collection)
            if let Type::ImplTrait(trait_name) = &**inner {
                format!("Vec<Box<dyn {}>>", trait_name)
            } else {
                format!(
                    "Vec<{}>",
                    type_to_rust_mapped_owned_str_slot(inner, map_custom)
                )
            }
        }
        Type::Array(inner, size) => {
            format!("[{}; {}]", type_to_rust_mapped(inner, map_custom), size)
        }
        Type::Reference(inner) => {
            // Special case: &[T] (slice) vs &Vec<T>
            if let Type::Vec(elem) = &**inner {
                format!("&[{}]", type_to_rust_mapped(elem, map_custom))
            // Special case: &[T; N] stays as &[T; N]
            } else if let Type::Array(elem, size) = &**inner {
                format!("&[{}; {}]", type_to_rust_mapped(elem, map_custom), size)
            // Special case: &str instead of &String (more idiomatic Rust)
            } else if is_windjammer_text_type(inner) {
                "&str".to_string()
            // Special case: &dyn Trait (don't box when already a reference)
            } else if let Type::TraitObject(trait_name) = &**inner {
                format!("&dyn {}", trait_name)
            // Dynamic dispatch: &trait Foo -> &dyn Foo
            } else if let Type::ImplTrait(trait_name) = &**inner {
                format!("&dyn {}", trait_name)
            } else {
                format!("&{}", type_to_rust_mapped(inner, map_custom))
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
                format!("&mut {}", type_to_rust_mapped(inner, map_custom))
            }
        }
        Type::RawPointer { mutable, pointee } => {
            // TDD: Raw pointer types for FFI
            // *const T -> *const T, *mut T -> *mut T
            if *mutable {
                format!("*mut {}", type_to_rust_mapped(pointee, map_custom))
            } else {
                format!("*const {}", type_to_rust_mapped(pointee, map_custom))
            }
        }
        Type::Tuple(types) => {
            let rust_types: Vec<String> = types
                .iter()
                .map(|t| type_to_rust_mapped(t, map_custom))
                .collect();
            format!("({})", rust_types.join(", "))
        }
        Type::Infer => "_".to_string(), // Type inference placeholder
        Type::FunctionPointer {
            params,
            return_type,
        } => {
            // WINDJAMMER DESIGN: Function pointers use &str (not &String!)
            // fn(string, i32) → fn(&str, i32) - idiomatic Rust, no Clippy warnings
            // fn(vec: Vec<T>) → fn(&Vec<T>) - borrowed for non-Copy types
            let param_strs: Vec<String> = params
                .iter()
                .map(|ty| {
                    match ty {
                        // WINDJAMMER DESIGN: String → &str for borrowed parameters
                        Type::String => "&str".to_string(),
                        Type::Custom(name) if name == "string" => "&str".to_string(),
                        // Already explicit references - keep as-is
                        Type::Reference(_) | Type::MutableReference(_) => {
                            type_to_rust_mapped(ty, map_custom)
                        }
                        // Copy types - pass by value
                        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => {
                            type_to_rust_mapped(ty, map_custom)
                        }
                        Type::Custom(name)
                            if matches!(
                                name.as_str(),
                                "i32"
                                    | "i64"
                                    | "u32"
                                    | "u64"
                                    | "f32"
                                    | "f64"
                                    | "bool"
                                    | "char"
                                    | "usize"
                                    | "isize"
                            ) =>
                        {
                            type_to_rust_mapped(ty, map_custom)
                        }
                        // Everything else - keep as-is (explicit types are respected)
                        _ => type_to_rust_mapped(ty, map_custom),
                    }
                })
                .collect();
            if let Some(ret) = return_type {
                format!(
                    "fn({}) -> {}",
                    param_strs.join(", "),
                    type_to_rust_mapped(ret, map_custom)
                )
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
            } else if is_windjammer_text_type(inner) {
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
        assert_eq!(
            type_to_rust(&Type::Reference(Box::new(Type::Custom("str".to_string())))),
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

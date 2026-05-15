//! Generic type parameter and `impl Trait` handling for ownership / signatures.

use crate::parser::Type;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// Check if a type is a generic type parameter (like T, G, S, T1, T2, etc.)
    /// or an impl Trait parameter (like `impl Describable`).
    /// Generic type parameters are typically single uppercase letters or uppercase with numbers.
    /// impl Trait parameters use static dispatch and should always be Owned
    /// (adding & would change the trait bound from `T: Trait` to `&T: Trait`).
    /// This matches the logic in codegen/rust/generator.rs is_generic_type().
    pub(crate) fn is_generic_type_param(ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => {
                // Generic type parameters are single uppercase letters, optionally followed by a digit.
                // Examples: T, U, K, V, S, G, T1, T2
                // NOT: BVH, GPU, API, SVO, AABB, AABB3 (these are concrete type names)
                let len = name.len();
                if len == 1 {
                    name.chars().next().is_some_and(|c| c.is_uppercase())
                } else if len == 2 {
                    let mut chars = name.chars();
                    let first = chars.next().unwrap();
                    let second = chars.next().unwrap();
                    first.is_uppercase() && second.is_ascii_digit()
                } else {
                    false
                }
            }
            // impl Trait parameters (e.g., `item: impl Describable`) should always be Owned.
            // Borrowing would change from `impl Trait` to `&impl Trait`, breaking trait dispatch.
            Type::ImplTrait(_) => true,
            _ => false,
        }
    }
}

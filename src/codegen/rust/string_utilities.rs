//! String Utilities
//!
//! Helper functions for string type analysis and codegen decisions.
//! These are pure functions with no state dependencies.

use crate::parser::Type;

/// Check if return type expects owned String in Rust.
/// Enclosing function/slot expects owned `String` in Rust (`string` / `String` in Windjammer).
pub fn return_type_expects_owned_string(ret: &Option<Type>) -> bool {
    match ret {
        Some(Type::String) => true,
        Some(Type::Custom(n)) if n == "String" || n == "string" => true,
        _ => false,
    }
}

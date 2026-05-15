//! Type-based method signature metadata for Rust codegen.

use crate::analyzer::OwnershipMode;
use crate::parser::Type;

/// Method signature for type-based parameter resolution
/// Stores the full signature of a method including parameter types and ownership
#[derive(Debug, Clone)]
pub struct MethodSignature {
    /// Name of the receiver type (e.g., "Vec", "String", "Inventory")
    pub receiver_type: String,
    /// Method name (e.g., "push", "contains", "has_item")
    pub method_name: String,
    /// Parameter types (in order, excluding self)
    pub param_types: Vec<Type>,
    /// Parameter ownership modes (Borrowed, MutBorrowed, Owned)
    pub param_ownership: Vec<OwnershipMode>,
    /// Return type (if any)
    pub return_type: Option<Type>,
    /// Whether method has a self receiver (vs. static method)
    pub has_self_receiver: bool,
}

impl MethodSignature {
    /// Create a new method signature
    pub fn new(
        receiver_type: impl Into<String>,
        method_name: impl Into<String>,
        param_types: Vec<Type>,
        param_ownership: Vec<OwnershipMode>,
        return_type: Option<Type>,
        has_self_receiver: bool,
    ) -> Self {
        Self {
            receiver_type: receiver_type.into(),
            method_name: method_name.into(),
            param_types,
            param_ownership,
            return_type,
            has_self_receiver,
        }
    }
}

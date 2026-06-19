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
    /// Convert analyzed method metadata into a `FunctionSignature` for call-site lowering.
    pub fn to_function_signature(&self) -> crate::analyzer::FunctionSignature {
        let mut param_types = self.param_types.clone();
        let mut param_ownership = self.param_ownership.clone();
        if self.has_self_receiver {
            param_types.insert(0, Type::Custom(self.receiver_type.clone()));
            param_ownership.insert(0, OwnershipMode::MutBorrowed);
        }
        crate::analyzer::FunctionSignature {
            name: format!("{}::{}", self.receiver_type, self.method_name),
            param_types,
            param_ownership,
            return_type: self.return_type.clone(),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: self.has_self_receiver,
            is_extern: false,
        }
    }

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

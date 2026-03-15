//! Ownership Tracking System for Code Generation
//!
//! Comprehensive ownership detection for ALL expression types. Tracks:
//! - Function parameters (from type annotations and analyzer inference)
//! - For-loop bindings (from iterable ownership)
//! - Match/if-let pattern bindings (from scrutinee ownership)
//!
//! Philosophy: "Compiler Does the Hard Work" - systematic ownership tracking, no guessing.

use crate::analyzer::OwnershipMode;
use crate::parser::ast::operators::UnaryOp;
use crate::parser::{Expression, Type};
use std::collections::HashMap;

/// Tracks ownership of variables during code generation.
/// Enables systematic queries instead of ad-hoc guessing.
#[derive(Debug, Clone)]
pub struct OwnershipTracker {
    /// Variable name -> ownership mode (parameters + bindings)
    variables: HashMap<String, OwnershipMode>,
    /// Struct field types (for future field access analysis)
    struct_fields: HashMap<String, Type>,
    /// Types that implement Copy (from @derive(Copy) registry)
    copy_types: std::collections::HashSet<String>,
}

impl OwnershipTracker {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            struct_fields: HashMap::new(),
            copy_types: std::collections::HashSet::new(),
        }
    }

    /// Register a struct field type (for future field access analysis)
    pub fn register_struct_field(&mut self, field_path: &str, type_: Type) {
        self.struct_fields.insert(field_path.to_string(), type_);
    }

    /// Register a type as Copy (from @derive(Copy) in source)
    pub fn register_copy_type(&mut self, name: &str) {
        self.copy_types.insert(name.to_string());
        // Also register base name for qualified types (e.g., module::Type)
        if let Some(base) = name.split("::").last() {
            if base != name {
                self.copy_types.insert(base.to_string());
            }
        }
    }

    /// Set Copy types from the global registry (called when generator receives copy_types_registry)
    pub fn set_copy_types_registry(&mut self, names: &std::collections::HashSet<String>) {
        for name in names {
            self.register_copy_type(name);
        }
    }

    /// Register a function parameter with its ownership
    pub fn register_parameter(&mut self, name: &str, ownership: OwnershipMode) {
        self.variables.insert(name.to_string(), ownership);
    }

    /// Register a binding (from for-loop, match arm, etc.)
    pub fn register_binding(&mut self, name: &str, ownership: OwnershipMode) {
        self.variables.insert(name.to_string(), ownership);
    }

    /// Remove a binding (when exiting scope, e.g., for-loop body)
    pub fn unregister_binding(&mut self, name: &str) {
        self.variables.remove(name);
    }

    /// Clear all variable bindings (call when entering new function)
    pub fn clear_function_scope(&mut self) {
        self.variables.clear();
    }

    /// Get ownership of a variable by name
    pub fn get_variable_ownership(&self, name: &str) -> Option<OwnershipMode> {
        self.variables.get(name).copied()
    }

    /// Check if a name is a borrowed parameter (alias for get_variable_ownership)
    pub fn is_borrowed_parameter(&self, name: &str) -> bool {
        self.get_variable_ownership(name) == Some(OwnershipMode::Borrowed)
    }

    /// Check if a name is a mutably borrowed parameter
    pub fn is_mut_borrowed_parameter(&self, name: &str) -> bool {
        self.get_variable_ownership(name) == Some(OwnershipMode::MutBorrowed)
    }

    /// Get ownership for a binding (alias for get_variable_ownership)
    pub fn get_binding_ownership(&self, name: &str) -> Option<OwnershipMode> {
        self.get_variable_ownership(name)
    }

    /// Check if variable is tracked as borrowed (shared or mut)
    pub fn is_variable_borrowed(&self, name: &str) -> bool {
        self.get_variable_ownership(name).map_or(false, |o| {
            matches!(o, OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
        })
    }

    /// Derive ownership from inferred type (for local_var_types)
    pub fn ownership_from_type(ty: &Type) -> OwnershipMode {
        match ty {
            Type::Reference(_) => OwnershipMode::Borrowed,
            Type::MutableReference(_) => OwnershipMode::MutBorrowed,
            _ => OwnershipMode::Owned,
        }
    }

    /// Get ownership of an expression - comprehensive handling for ALL expression types.
    pub fn get_expression_ownership(&self, expr: &Expression) -> OwnershipMode {
        match expr {
            Expression::Identifier { name, .. } => {
                self.get_variable_ownership(name).unwrap_or(OwnershipMode::Owned)
            }
            Expression::FieldAccess { object, .. } => self.get_expression_ownership(object),
            Expression::Index { object, .. } => self.get_expression_ownership(object),
            Expression::MethodCall {
                object,
                method,
                ..
            } => self.get_method_call_ownership(object, method),
            Expression::Unary { op, operand, .. } => self.get_unary_ownership(*op, operand),
            Expression::Cast { expr, .. } => self.get_expression_ownership(expr),
            Expression::TryOp { expr, .. } => self.get_expression_ownership(expr),
            Expression::Block { statements, .. } => {
                if let Some(last) = statements.last() {
                    if let crate::parser::Statement::Expression { expr, .. } = last {
                        return self.get_expression_ownership(expr);
                    }
                }
                OwnershipMode::Owned
            }
            // Literals, binary ops, calls, literals - always owned
            Expression::Literal { .. }
            | Expression::Binary { .. }
            | Expression::Call { .. }
            | Expression::StructLiteral { .. }
            | Expression::MapLiteral { .. }
            | Expression::Array { .. }
            | Expression::Tuple { .. }
            | Expression::Range { .. }
            | Expression::Closure { .. }
            | Expression::MacroInvocation { .. }
            | Expression::Await { .. }
            | Expression::ChannelSend { .. }
            | Expression::ChannelRecv { .. } => OwnershipMode::Owned,
        }
    }

    fn get_method_call_ownership(&self, object: &Expression, method: &str) -> OwnershipMode {
        match method {
            "clone" | "to_owned" | "to_string" | "into_iter" => OwnershipMode::Owned,
            _ => self.get_expression_ownership(object),
        }
    }

    fn get_unary_ownership(&self, op: UnaryOp, _operand: &Expression) -> OwnershipMode {
        match op {
            UnaryOp::Deref => OwnershipMode::Owned,
            UnaryOp::Ref => OwnershipMode::Borrowed,
            UnaryOp::MutRef => OwnershipMode::MutBorrowed,
            UnaryOp::Not | UnaryOp::Neg => OwnershipMode::Owned,
        }
    }

    /// Check if a type is in the Copy registry
    pub fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => {
                self.copy_types.contains(name.as_str())
                    || name.split("::").last().map_or(false, |b| self.copy_types.contains(b))
            }
            _ => false,
        }
    }

    /// Derive ownership from parameter type (for explicit &T, &mut T in signature)
    pub fn ownership_from_param_type(param_type: &Type) -> OwnershipMode {
        match param_type {
            Type::Reference(_) => OwnershipMode::Borrowed,
            Type::MutableReference(_) => OwnershipMode::MutBorrowed,
            _ => OwnershipMode::Owned,
        }
    }
}

impl Default for OwnershipTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup_parameter() {
        let mut tracker = OwnershipTracker::new();
        tracker.register_parameter("data", OwnershipMode::Borrowed);
        assert_eq!(
            tracker.get_variable_ownership("data"),
            Some(OwnershipMode::Borrowed)
        );
    }

    #[test]
    fn test_register_binding() {
        let mut tracker = OwnershipTracker::new();
        tracker.register_binding("item", OwnershipMode::Borrowed);
        assert_eq!(
            tracker.get_variable_ownership("item"),
            Some(OwnershipMode::Borrowed)
        );
    }

    #[test]
    fn test_clear_function_scope() {
        let mut tracker = OwnershipTracker::new();
        tracker.register_parameter("x", OwnershipMode::Owned);
        tracker.clear_function_scope();
        assert_eq!(tracker.get_variable_ownership("x"), None);
    }

    #[test]
    fn test_ownership_from_param_type() {
        assert_eq!(
            OwnershipTracker::ownership_from_param_type(&Type::Reference(Box::new(Type::Int))),
            OwnershipMode::Borrowed
        );
        assert_eq!(
            OwnershipTracker::ownership_from_param_type(&Type::MutableReference(Box::new(
                Type::Int
            ))),
            OwnershipMode::MutBorrowed
        );
        assert_eq!(
            OwnershipTracker::ownership_from_param_type(&Type::Int),
            OwnershipMode::Owned
        );
    }

    #[test]
    fn test_register_copy_type() {
        let mut tracker = OwnershipTracker::new();
        tracker.register_copy_type("Vec3");
        assert!(tracker.is_copy_type(&Type::Custom("Vec3".to_string())));
        assert!(tracker.is_copy_type(&Type::Custom("math::Vec3".to_string())));
    }
}

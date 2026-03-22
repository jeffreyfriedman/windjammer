//! Copy Semantics Layer - Applies Rust's Copy rules to ownership tracking
//!
//! Key insight: In Rust, Copy types behave differently from non-Copy types:
//! - `let x = &copy_value` → x is &T, but using x auto-copies to T
//! - `let (a, b) = &(1, 2)` → a and b are i32 (owned), not &i32
//! - `struct.copy_field` from &struct → owned T, not &T
//!
//! This layer converts tracked ownership (Borrowed/MutBorrowed/Owned) to
//! EFFECTIVE ownership based on whether the type implements Copy.
//! Also encodes when Rust applies automatic dereferencing or copying,
//! used by rust_coercion_rules to determine required coercions.

use crate::analyzer::OwnershipMode;
use crate::parser::Type;
use std::collections::HashSet;

pub struct CopySemantics {
    copy_types: HashSet<String>,
}

impl CopySemantics {
    pub fn new() -> Self {
        Self {
            copy_types: HashSet::new(),
        }
    }

    /// Register a Copy type from @derive(Copy) registry
    pub fn register_copy_type(&mut self, name: &str) {
        self.copy_types.insert(name.to_string());
        if let Some(base) = name.split("::").last() {
            if base != name {
                self.copy_types.insert(base.to_string());
            }
        }
    }

    /// Set the entire Copy registry
    pub fn set_copy_types(&mut self, types: &HashSet<String>) {
        for ty in types {
            self.register_copy_type(ty);
        }
    }

    /// Get the type to check for Copy semantics.
    /// For references (&T, &mut T), we check the POINTEE (T) - when we use a reference
    /// in a binary op, Rust auto-derefs to the inner type. So &i32 in `a + b` uses i32's Copy.
    fn type_for_copy_check(ty: &Type) -> &Type {
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => inner,
            _ => ty,
        }
    }

    /// Check if a type implements Copy
    pub fn is_type_copy(&self, ty: &Type) -> bool {
        let ty = Self::type_for_copy_check(ty);
        match ty {
            // Primitives are always Copy (parser Type variants)
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,

            // Custom types: check registry or known primitives
            Type::Custom(name) => {
                self.copy_types.contains(name.as_str())
                    || name
                        .split("::")
                        .last()
                        .map_or(false, |b| self.copy_types.contains(b))
                    || matches!(
                        name.as_str(),
                        "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
                            | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
                            | "f32" | "f64" | "bool" | "char" | "int"
                    )
            }

            // Tuples are Copy if all elements are Copy
            Type::Tuple(types) => types.iter().all(|t| self.is_type_copy(t)),

            // Option<T> is Copy if T is Copy
            Type::Option(inner) => self.is_type_copy(inner),

            // Array[T; N] is Copy if T is Copy
            Type::Array(inner, _) => self.is_type_copy(inner),

            // References (after stripping): Vec, Result, etc. are not Copy
            _ => false,
        }
    }

    /// Get EFFECTIVE ownership after applying Copy semantics
    ///
    /// Key insight: &Copy in Rust auto-copies, yielding owned values
    pub fn effective_ownership(
        &self,
        tracked_ownership: OwnershipMode,
        expr_type: &Type,
    ) -> OwnershipMode {
        match tracked_ownership {
            // &Copy or &mut Copy → Owned (Rust auto-copies)
            OwnershipMode::Borrowed | OwnershipMode::MutBorrowed
                if self.is_type_copy(expr_type) =>
            {
                OwnershipMode::Owned // Copy gives you owned value!
            }
            // &Non-Copy → still Borrowed (can't auto-copy)
            // Owned → still Owned
            other => other,
        }
    }

    /// Check if expression ACTUALLY needs explicit deref in generated Rust
    ///
    /// Rust auto-derefs in many contexts, we should NOT add * there
    pub fn needs_explicit_deref(
        &self,
        context: DerefContext,
        ownership: OwnershipMode,
        expr_type: &Type,
    ) -> bool {
        let is_copy = self.is_type_copy(expr_type);

        match (ownership, is_copy, context) {
            // Copy types: Rust auto-copies in most contexts
            (
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                true,
                DerefContext::Comparison,
            ) => false,
            (
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                true,
                DerefContext::MethodCall,
            ) => false,
            (
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                true,
                DerefContext::FieldAccess,
            ) => false,
            (
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                true,
                DerefContext::BinaryOp,
            ) => false,
            (
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                true,
                DerefContext::StructLiteral,
            ) => false,
            (
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                true,
                DerefContext::FunctionArg,
            ) => false,
            (
                OwnershipMode::Borrowed | OwnershipMode::MutBorrowed,
                true,
                DerefContext::Standalone,
            ) => false,

            // Non-Copy from borrow: need .clone(), not *
            (OwnershipMode::Borrowed | OwnershipMode::MutBorrowed, false, _) => false,

            // Owned: never need deref
            (OwnershipMode::Owned, _, _) => false,
        }
    }
}

impl Default for CopySemantics {
    fn default() -> Self {
        Self::new()
    }
}

/// Context in which an expression is used - affects Rust's auto-deref/auto-copy behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerefContext {
    /// Comparison operators (==, !=, <, etc.) - Rust auto-derefs and auto-copies
    Comparison,
    /// Method call receiver - Rust auto-derefs
    MethodCall,
    /// Field access (expr.field) - Rust auto-derefs
    FieldAccess,
    /// Binary operators (+, -, etc.) - Rust auto-derefs and auto-copies
    BinaryOp,
    /// Struct literal field - Rust auto-copies for Copy types
    StructLiteral,
    /// Function argument - Rust auto-copies for Copy types
    FunctionArg,
    /// Standalone expression (assignment RHS, return, etc.) - no special coercion
    Standalone,
}

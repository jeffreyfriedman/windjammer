//! Rust Coercion Rules - Determines when Rust auto-derefs/auto-copies
//!
//! Rust has complex coercion rules:
//! - Auto-deref for method calls, field access, comparisons
//! - Auto-copy for Copy types in many contexts
//! - Explicit deref/clone needed in other contexts
//!
//! This module encodes these rules systematically.

use crate::analyzer::OwnershipMode;
use crate::codegen::rust::copy_semantics::DerefContext;

pub struct RustCoercionRules;

impl RustCoercionRules {
    /// Check if Rust auto-derefs in this context
    pub fn rust_auto_derefs(context: DerefContext) -> bool {
        matches!(
            context,
            DerefContext::Comparison
                | DerefContext::MethodCall
                | DerefContext::FieldAccess
                | DerefContext::BinaryOp
        )
    }

    /// Check if Rust auto-copies in this context (for Copy types)
    pub fn rust_auto_copies(context: DerefContext, is_copy: bool) -> bool {
        is_copy
            && matches!(
                context,
                DerefContext::Comparison
                    | DerefContext::BinaryOp
                    | DerefContext::StructLiteral
                    | DerefContext::FunctionArg
            )
    }

    /// Determine required coercion to go from source to target ownership
    pub fn required_coercion(
        source_ownership: OwnershipMode,
        target_ownership: OwnershipMode,
        is_copy: bool,
        context: DerefContext,
    ) -> Coercion {
        use OwnershipMode::*;

        match (source_ownership, target_ownership, is_copy) {
            // Source and target match - no coercion
            (Owned, Owned, _) => Coercion::None,
            (Borrowed, Borrowed, _) => Coercion::None,
            (MutBorrowed, MutBorrowed, _) => Coercion::None,

            // Borrowed → Owned
            (Borrowed, Owned, true) if Self::rust_auto_copies(context, true) => Coercion::None,
            (Borrowed, Owned, true) => Coercion::Deref,
            (Borrowed, Owned, false) => Coercion::Clone,

            (MutBorrowed, Owned, true) if Self::rust_auto_copies(context, true) => Coercion::None,
            (MutBorrowed, Owned, true) => Coercion::Deref,
            (MutBorrowed, Owned, false) => Coercion::Clone,

            // Owned → Borrowed
            (Owned, Borrowed, _) => Coercion::Borrow,
            (Owned, MutBorrowed, _) => Coercion::BorrowMut,

            // Borrow ↔ MutBorrow (generally invalid, but handle gracefully)
            (Borrowed, MutBorrowed, _) => Coercion::None,  // Can't upgrade &T to &mut T
            (MutBorrowed, Borrowed, _) => Coercion::None,  // Can downgrade &mut T to &T implicitly
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coercion {
    None,      // No change needed
    Deref,     // Add *expr
    Clone,     // Add expr.clone()
    Borrow,    // Add &expr
    BorrowMut, // Add &mut expr
}

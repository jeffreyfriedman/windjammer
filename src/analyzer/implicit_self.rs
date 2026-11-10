//! The receiver of a method that uses `self` without declaring it.

use crate::parser::{FunctionDecl, Type};

use super::{Analyzer, OwnershipMode};

impl<'ast> Analyzer<'ast> {
    /// The IR solver never sees such a `self` (it has no parameter), so the analyzer's decision is final: a `self`
    /// that is returned is consumed. The analyzer's seed for a builder (`self.f = x; self`) is `&mut`, which the solver
    /// raises to `self` for a declared receiver; here nobody would, and `&mut self` returned as `Builder` is rustc E0308.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_implicit_self_receiver_ownership(
        &self,
        func: &FunctionDecl<'ast>,
        modifies_fields: bool,
        returns_self: bool,
        body_moves_fields: bool,
        consumes_self: bool,
        calls_mutating: bool,
        calls_owned_on_self: bool,
    ) -> OwnershipMode {
        let mode = self.resolve_impl_self_receiver_ownership(func, modifies_fields, returns_self, body_moves_fields, consumes_self, calls_mutating, calls_owned_on_self);
        if mode == OwnershipMode::MutBorrowed && returns_self && !matches!(func.return_type, Some(Type::Reference(_) | Type::MutableReference(_))) {
            OwnershipMode::Owned
        } else {
            mode
        }
    }
}

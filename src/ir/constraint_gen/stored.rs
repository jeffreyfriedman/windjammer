//! OWN-STORED for local copies: `let mut out = state` moves the parameter into a local the body owns and
//! changes, so the parameter's use is an owned use, as for a struct field or `push(state)`. Borrowing it and
//! copying inside would add a clone wherever the caller's value is dead.

use super::*;

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// `let mut x = p` where `p` names a by-value parameter (not a local that shadows it, not `self`).
    pub(super) fn emit_owned_for_mut_local_of_param(&mut self, mutable: bool, pattern: &Pattern<'ast>, value: &Expression<'ast>) {
        let (true, Pattern::Identifier(_) | Pattern::MutBinding(_), Expression::Identifier { name, location }) = (mutable || matches!(pattern, Pattern::MutBinding(_)), pattern, value) else { return };
        if name == "self" || self.scopes.iter().any(|s| s.contains_key(name)) {
            return;
        }
        let by_value = self.analyzed.decl.parameters.iter().any(|p| p.name == *name && !matches!(p.type_, Type::Reference(_) | Type::MutableReference(_)));
        if let Some(&var) = self.param_vars.get(name).filter(|_| by_value) {
            self.cs.add_because(
                Constraint::OwnershipIs(var, OwnedType::Owned),
                origin("OWN-STORED", format!("`{name}` is moved into the mutable local it initialises"), location),
            );
        }
    }
}

/// OWN-MUT-PARAM-COPY (SPEC-OWN-008): `fn f(mut s: S)` declares that the callee changes its own copy, so the
/// parameter is owned and the caller's value is untouched (a mutated parameter without `mut` is a mutable
/// borrow instead, OWN-MUT-ASSIGN).
pub(super) fn seed_mut_declared_param(cs: &mut ConstraintSet, var: ConstraintVar, param: &crate::parser::ast::core::Parameter<'_>) {
    if param.is_mutable && param.name != "self" && !matches!(param.type_, Type::Reference(_) | Type::MutableReference(_)) {
        cs.add_because(
            Constraint::OwnershipIs(var, OwnedType::Owned),
            origin("OWN-MUT-PARAM-COPY", format!("`{}` is declared `mut`: the callee changes its own copy", param.name), &None),
        );
    }

}

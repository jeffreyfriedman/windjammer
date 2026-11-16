//! OWN-STORED for local copies: `let mut out = state` moves the parameter into a local the body owns and
//! changes, so the parameter's use is an owned use, as for a struct field or `push(state)`. Borrowing it and
//! copying inside would add a clone wherever the caller's value is dead.

use super::*;

impl<'a, 'ast> AstConstraintWalker<'a, 'ast> {
    /// `let mut x = p` where `p` names a by-value parameter (not a local that shadows it, not `self`).
    pub(super) fn emit_owned_for_mut_local_of_param(&mut self, mutable: bool, pattern: &Pattern<'ast>, value: &Expression<'ast>) {
        if mutable || matches!(pattern, Pattern::MutBinding(_)) {
            self.emit_owned_for_moved_field(value);
        }
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

    /// OWN-CALL-FORMAL for a receiver: `store.add(t)` where `Store::add` takes `self` by value consumes `store`, so a
    /// by-value parameter of a struct of this program passed there is owned. A field receiver (`self.a.take()`) does
    /// not make its root owned: that would spread a consuming callee to every reader of the root.
    pub(super) fn emit_owned_for_consumed_receiver(&mut self, object: &Expression<'ast>) {
        let Expression::Identifier { name, location } = object else { return };
        if name == "self" || self.scopes.iter().any(|s| s.contains_key(name)) {
            return;
        }
        let fields = self.struct_field_types.unwrap_or_else(|| super::empty_struct_field_types());
        let user_struct = |t: &Type| matches!(t, Type::Custom(n) if fields.contains_key(n)) && field_type_owns_data(t, fields, 0);
        let by_value = self.analyzed.decl.parameters.iter().any(|p| p.name == *name && matches!(p.ownership, crate::parser::OwnershipHint::Inferred) && user_struct(&p.type_));
        if let Some(&var) = self.param_vars.get(name).filter(|_| by_value) {
            self.cs.add_because(
                Constraint::OwnershipIs(var, OwnedType::Owned),
                origin("OWN-CALL-FORMAL", format!("`{name}` is the receiver of a method that takes `self` by value"), location),
            );
        }
    }

    /// OWN-STORED for a field (`Holder { bag: p.bag }`, `let mut next = self.items`) in a function that returns a value:
    /// storing a non-Copy field of a by-value parameter or receiver somewhere that owns it takes the parameter owned,
    /// so the field moves instead of being copied. A returned field (a getter, `fn name(self) -> string { self.name }`) is
    /// not: owning the whole receiver there costs every caller holding a borrow a copy of all of it. Not when the body also changes it (directly, through a field or a
    /// `&mut self` method): the caller sees that change, so the parameter stays a mutable borrow and the field is
    /// copied (owner decision, pilot-7 hygiene list).
    pub(super) fn emit_owned_for_moved_field(&mut self, value: &Expression<'ast>) {
        let Expression::FieldAccess { object, location, .. } = value else { return };
        // Only a function that produces a value from its parameter (a builder, wither or consuming transform) owns
        // it; a unit method (`flush(self) { store(self.path) }`) keeps its borrow and copies, so its `&mut self`
        // callers need no copy of the whole receiver.
        if self.analyzed.decl.return_type.as_ref().is_none_or(|t| matches!(t, Type::Tuple(items) if items.is_empty())) {
            return;
        }
        let Some(root) = self.root_identifier(object) else { return };
        if self.scopes.iter().any(|s| s.contains_key(&root)) {
            return;
        }
        let declared = self.analyzed.decl.parameters.iter().find(|p| p.name == root);
        let by_value = match declared {
            Some(p) => !matches!(p.type_, Type::Reference(_) | Type::MutableReference(_)) && !matches!(p.ownership, crate::parser::OwnershipHint::Ref | crate::parser::OwnershipHint::Mut),
            None => root == "self" && crate::ir::node::implicit_self_mode(self.analyzed).is_some(),
        };
        let a = self.analyzed;
        let changed = a.mutated_parameters.contains(&root) || a.field_mutated_parameters.contains(&root)
            || a.inferred_ownership.get(&root) == Some(&OwnershipMode::MutBorrowed);
        let Some(&var) = self.param_vars.get(&root).filter(|_| by_value && !changed) else { return };
        let fields = self.struct_field_types.unwrap_or_else(|| super::empty_struct_field_types());
        let Some(ty) = crate::ir::receiver_type::infer_receiver_type(value, &a.decl, fields) else { return };
        if !field_type_owns_data(&ty, fields, 0) {
            return;
        }
        self.cs.add_because(
            Constraint::OwnershipIs(var, OwnedType::Owned),
            origin("OWN-STORED", format!("a non-Copy field of `{root}` is moved out of it"), location),
        );
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

/// Whether moving a value of `ty` moves owned data: text, a collection, a struct of this program with such a field.
/// A name the program does not declare as a struct (an enum, a type from a crate) is left alone: only declarations
/// decide here.
fn field_type_owns_data(ty: &Type, fields: &HashMap<String, HashMap<String, Type>>, depth: usize) -> bool {
    match ty {
        Type::String | Type::Vec(_) | Type::Parameterized(..) | Type::Result(..) => true,
        Type::Option(inner) => field_type_owns_data(inner, fields, depth + 1),
        Type::Array(inner, _) => field_type_owns_data(inner, fields, depth + 1),
        Type::Tuple(items) => items.iter().any(|t| field_type_owns_data(t, fields, depth + 1)),
        Type::Custom(name) => depth < 4 && fields.get(name).is_some_and(|f| f.values().any(|t| field_type_owns_data(t, fields, depth + 1))),
        _ => false,
    }
}

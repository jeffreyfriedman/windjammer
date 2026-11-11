//! Small binding-type decisions shared by `let`, `match` and method-call lowering: what type a local
//! bound from `self` has, the type of a payload bound through a reference scrutinee, and when a
//! consuming method call on a reused binding gets a copy.

use crate::parser::{Expression, Type};

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// `Self` inside an impl names the impl's type (`let mut host = self` -> `Host`), so its methods resolve
    /// (`host.execute(..)` is `Host::execute`).
    pub(super) fn impl_self_type_named(&self, ty: Type) -> Type {
        match (&ty, &self.current_struct_name) {
            (Type::Custom(n), Some(sn)) if n == "Self" && self.in_impl_block => {
                Type::Custom(sn.split('<').next().unwrap_or(sn).to_string())
            }
            _ => ty,
        }
    }

    /// [`Self::impl_self_type_named`] for a receiver type name.
    pub(super) fn impl_self_name(&self, name: String) -> String {
        match &self.current_struct_name {
            Some(sn) if name == "Self" => sn.split('<').next().unwrap_or(sn).to_string(),
            _ => name,
        }
    }

    /// Payload bindings of a match whose scrutinee binds by reference: a non-Copy payload has type `&T`
    /// (`match b { Val::Name(y) => x == y }` compares `String` with `&String`). Copy payloads bind by value.
    /// `copied`: the scrutinee is `.copied()` (owned Copy payloads). `by_ref`: it binds by reference.
    pub(super) fn arm_payload_types(&self, value: &Expression<'ast>, pattern: &crate::parser::Pattern<'ast>, copied: bool, by_ref: bool) -> Vec<(String, Type)> {
        let mut entries = if copied {
            self.infer_match_bound_types_from_copied_option(value, pattern)
        } else {
            self.infer_match_bound_types(value, pattern)
        };
        for entry in entries.iter_mut().filter(|_| by_ref) {
            if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) && !self.is_type_copy(&entry.1) {
                entry.1 = Type::Reference(Box::new(entry.1.clone()));
            }
        }
        entries
    }

    /// CLONE-USE-AFTER-MOVE on a receiver: an owned binding (not `self`) passed by value to a method that
    /// consumes its receiver (`self` by value) and read again after this statement
    /// (`let ok = active.has(k); (active, ok)`) gives the method a copy.
    pub(super) fn copy_reused_owned_receiver(&self, object: &Expression<'ast>, method: &str, obj_str: &mut String) {
        let Expression::Identifier { name, .. } = object else { return };
        if name == "self"
            || obj_str.ends_with(".clone()")
            || obj_str.starts_with('&')
            || self.borrowed_iterator_vars.contains(name)
            || self.inferred_borrowed_params.contains(name)
            || self.inferred_mut_borrowed_params.contains(name)
            || !self.local_binding_reused_after_current_statement(name)
        {
            return;
        }
        let Some(recv_ty) = self.infer_expression_type(object).filter(|t| !self.is_type_copy(t)) else { return };
        let consumes = Self::type_to_name(&recv_ty).is_some_and(|tn| {
            let base = tn.split('<').next().unwrap_or(&tn).to_string();
            let leaf = base.rsplit("::").next().unwrap_or(&base).to_string();
            [format!("{base}::{method}"), format!("{leaf}::{method}")].iter().any(|q| {
                self.get_signature_with_global(q).is_some_and(|sig| {
                    sig.has_self_receiver
                        && matches!(self.effective_method_self_ownership(q, sig), crate::analyzer::OwnershipMode::Owned)
                })
            })
        });
        if consumes {
            *obj_str = format!("{obj_str}.clone()");
        }
    }
}

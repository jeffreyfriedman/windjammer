//! Method call receiver codegen (object expr + recv fixes).

use crate::parser::Expression;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn mc_build_method_receiver_string(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
    ) -> String {
        // METHOD CALL CONTEXT: Suppress Vec index auto-clone when generating the
        // object of a method call. Methods take &self or &mut self, so Rust allows
        // calling methods on &T returned by Vec indexing without cloning.
        // e.g., self.lights[i].is_enabled() → no need to clone the whole Light2D
        let prev_field_access = self.in_field_access_object;
        self.in_field_access_object = true;
        // DOUBLE-CLONE FIX: When the source has explicit .clone(), suppress auto-clone
        // on the object to prevent .clone().clone(). The explicit clone IS the clone.
        let prev_explicit_clone = self.in_explicit_clone_call;
        if method == "clone" {
            self.in_explicit_clone_call = true;
        }
        // Suppress .into() coercion on receiver of .to_string() / .to_owned() / .clone()
        // since those methods already produce an owned value.
        let prev_coerce = self.coerce_string_literals_to_owned;
        if matches!(method, "to_string" | "to_owned" | "clone") {
            self.coerce_string_literals_to_owned = false;
        }
        let mut obj_str = self.generate_expression_with_precedence(object);
        self.coerce_string_literals_to_owned = prev_coerce;
        self.in_field_access_object = prev_field_access;
        self.in_explicit_clone_call = prev_explicit_clone;
        // E0507: `collection[i].method(args)` on non-Copy elements must clone before the call.
        // Rust cannot move out of a Vec index. Even when the registry says `&self`, library
        // multipass metadata can disagree with the emitted receiver (`self` vs `&self`), so
        // always clone indexed non-Copy receivers (extra clone on `&self` methods is correct).
        if matches!(object, Expression::Index { .. }) && !obj_str.ends_with(".clone()") {
            let is_copy = self
                .infer_expression_type(object)
                .as_ref()
                .is_some_and(|t| self.is_type_copy(t));
            if !is_copy {
                obj_str = format!("{}.clone()", obj_str);
            }
        }

        // E0507: `borrowed_var.method(args)` when the method consumes `self` (owned receiver)
        // and the variable is a borrowed iterator variable (from `for x in &collection`).
        // Must clone: `condition.clone().evaluate(state)` instead of `condition.evaluate(state)`.
        if let Expression::Identifier { name, .. } = object {
            let is_borrowed_iter = self.borrowed_iterator_vars.contains(name) && method != "clone";
            let is_mut_borrowed_param = self.inferred_mut_borrowed_params.contains(name)
                && method != "clone";
            if is_borrowed_iter || is_mut_borrowed_param {
                if let Some(recv_ty) = self.infer_expression_type(object) {
                    if !self.is_type_copy(&recv_ty) {
                        if let Some(tn) = Self::type_to_name(&recv_ty) {
                            let qualified = format!("{}::{}", tn, method);
                            let sig_opt = self
                                .signature_registry
                                .get_signature(&qualified)
                                .or_else(|| self.signature_registry.get_signature(method));
                            if let Some(sig) = sig_opt {
                                if sig.has_self_receiver
                                    && sig.param_ownership.first()
                                        == Some(&crate::analyzer::OwnershipMode::Owned)
                                    && !obj_str.ends_with(".clone()")
                                {
                                    obj_str = format!("{}.clone()", obj_str);
                                }
                            } else if is_borrowed_iter && !is_mut_borrowed_param && !obj_str.ends_with(".clone()") {
                                // Unknown signature on borrowed iterator var — clone conservatively (E0507).
                                obj_str = format!("{}.clone()", obj_str);
                            }
                            // For &mut params with unknown signatures, do NOT clone.
                            // &mut refs can call &self and &mut self methods without cloning.
                        }
                    }
                }
            }
        }

        // DOUBLE-CLONE SAFETY NET: If the object was auto-cloned by the FieldAccess
        // handler and this IS a .clone() call, strip the redundant auto-clone.
        // e.g., "stack.item.clone()" from auto-clone + ".clone()" from source
        //     → should be "stack.item.clone()", not "stack.item.clone().clone()"
        if method == "clone" && obj_str.ends_with(".clone()") {
            obj_str = obj_str[..obj_str.len() - 8].to_string();
        }

        // TDD FIX: Option::unwrap() move error prevention
        // TDD FIX: AUTO-CLONE Option::unwrap() on borrowed fields
        // When calling .unwrap() on a borrowed Option field, we must clone before unwrap:
        //   node.children.unwrap() where node is &Node → ERROR: cannot move from &Option
        //   node.children.clone().unwrap() → ✅ OK
        // THE WINDJAMMER WAY: Users write .unwrap() naturally, compiler handles ownership
        if method == "unwrap" {
            // Check if object is a field access (node.children) that needs clone
            let needs_clone = if let Expression::FieldAccess {
                object: field_obj, ..
            } = object
            {
                // Is this accessing a field on a borrowed parameter?
                if let Expression::Identifier { ref name, .. } = **field_obj {
                    // Check if the identifier is an inferred borrowed parameter
                    self.inferred_borrowed_params.contains(name)
                } else {
                    false
                }
            } else {
                false
            };

            if needs_clone && !obj_str.contains(".clone()") {
                obj_str = format!("{}.clone()", obj_str);
            }
        }

        // E0507 fix: Option::map on self.field with &self must use .as_ref().map(...)
        // self.children.map(|c| ...) with &self → self.children.as_ref().map(|c| ...)
        if method == "map"
            && self.inferred_borrowed_params.contains("self")
            && self.codegen_expression_traces_to_self(object)
            && !obj_str.contains(".as_ref()")
        {
            obj_str = format!("{}.as_ref()", obj_str);
        }

        obj_str
    }

    /// True when the resolved method signature takes `&self` or `&mut self` (not owned `self`).
    /// Unknown signatures return false so index elements are cloned before the call (E0507-safe).
    fn method_call_receiver_is_ref(
        &self,
        object: &Expression<'ast>,
        method: &str,
    ) -> bool {
        let Some(recv_ty) = self.infer_expression_type(object) else {
            return false;
        };
        let Some(tn) = Self::type_to_name(&recv_ty) else {
            return false;
        };
        let qualified = format!("{tn}::{method}");
        let sig_opt = self
            .signature_registry
            .get_signature(&qualified)
            .or_else(|| self.signature_registry.get_signature(method));
        let Some(sig) = sig_opt else {
            return false;
        };
        if !sig.has_self_receiver {
            return false;
        }
        matches!(
            sig.param_ownership.first(),
            Some(crate::analyzer::OwnershipMode::Borrowed
                | crate::analyzer::OwnershipMode::MutBorrowed)
        )
    }
}

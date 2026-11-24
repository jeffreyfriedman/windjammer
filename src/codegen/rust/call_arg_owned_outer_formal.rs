//! Final call-site pass for owned outer formals.

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Final call-site pass for owned outer formals: borrow in `if` conditions, clone for owned callees elsewhere.
    pub(crate) fn finalize_owned_outer_formal_call_arg(
        &self,
        coerced: &mut String,
        arg_expr: &Expression<'ast>,
        callee_wants_shared_borrow: bool,
        callee_wants_owned: bool,
    ) {
        let Expression::Identifier { name, .. } = arg_expr else {
            return;
        };
        // Copy formals/locals auto-copy — peel stale `&x` into owned slots,
        // never invent `.clone()` from owned-outer / reuse finalize.
        if self.binding_name_is_copy(name)
            || self.call_arg_is_copy_identity(arg_expr, None)
            || self.current_function_params.iter().any(|p| {
                p.name == *name && matches!(&p.type_, Type::Custom(n) if self.enum_is_unit_copy(n))
            })
        {
            if callee_wants_owned
                && !callee_wants_shared_borrow
                && coerced.starts_with('&')
                && !coerced.starts_with("&mut ")
            {
                *coerced = crate::codegen::rust::expression_utilities::borrow_base_expr(coerced)
                    .to_string();
            }
            return;
        }
        if !self.caller_keeps_owned_outer_formal(name)
            && !self.local_binding_is_owned_non_copy(name)
        {
            return;
        }
        if self.in_if_condition {
            // Signature-driven: never borrow Copy / owned-pass args in `if` conditions.
            // The forward-ref heuristic exists to avoid moving non-Copy formals; Copy
            // and callee-owned contracts must remain by-value (`update(dt)` not `&dt`).
            // Copy aggregates (`through: Lsn`) are not `is_copy_pass_by_value_formal`
            // but still pass by value into owned formals (regression-060).
            let caller_copy_aggregate = self.current_function_params.iter().any(|p| {
                p.name == *name
                    && self.is_type_copy(&p.type_)
                    && !crate::type_classification::is_copy_pass_by_value_formal(&p.type_)
            });
            if self.caller_param_is_copy_pass_by_value(name) || caller_copy_aggregate {
                return;
            }
            // Callee owned formal: shared `&binding` / clone→borrow is never valid
            // (ReBAC `contains_string(out)` into `items: Vec<String>` inside `if`).
            // Reuse after the condition requires `.clone()`, not `&` — except if-facade
            // forward-ref (param in condition + then/else): borrow at guard, move in branches.
            if callee_wants_owned && !callee_wants_shared_borrow {
                let body: Vec<_> = self.current_function_body.iter().copied().collect();
                let if_facade_forward_ref = self.current_fn_forward_ref_if_params.contains(name)
                    && self.param_used_in_if_with_condition_and_branches(&body, name);
                if if_facade_forward_ref {
                    if coerced.starts_with("&mut ") {
                        *coerced = format!(
                            "&{}",
                            crate::codegen::rust::expression_utilities::borrow_base_expr(coerced),
                        );
                    } else if coerced.ends_with(".clone()") {
                        let base = coerced.trim_end_matches(".clone()").trim();
                        *coerced = format!("&{base}");
                    } else if !coerced.starts_with('&') {
                        *coerced = format!("&{coerced}");
                    }
                    return;
                }
                if coerced.starts_with('&') && !coerced.starts_with("&mut ") {
                    *coerced =
                        crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(
                            coerced,
                        );
                } else if !coerced.ends_with(".clone()")
                    && !coerced.starts_with("&mut ")
                    && (self.current_fn_forward_ref_if_params.contains(name)
                        || self.param_used_in_if_with_condition_and_branches(&body, name))
                {
                    *coerced = format!("{coerced}.clone()");
                }
                return;
            }
            if !callee_wants_shared_borrow
                && !self.current_fn_forward_ref_if_params.contains(name)
                && !self.param_used_in_if_with_condition_and_branches(
                    &self
                        .current_function_body
                        .iter()
                        .copied()
                        .collect::<Vec<_>>(),
                    name,
                )
            {
                return;
            }
            if !coerced.starts_with('&') {
                if coerced.ends_with(".clone()") {
                    let base = coerced.trim_end_matches(".clone()").trim();
                    *coerced = format!("&{base}");
                } else {
                    *coerced = format!("&{coerced}");
                }
            }
            return;
        }
        if callee_wants_owned
            && !callee_wants_shared_borrow
            && (coerced.starts_with("&mut ") || coerced.starts_with('&'))
        {
            // Match-arm bindings into owned formals must move (strip `&`), same as
            // other owned slots — do not preserve a stale shared borrow.
            *coerced =
                crate::codegen::rust::expression_utilities::coerce_borrowed_arg_to_owned(coerced);
        } else if callee_wants_owned
            && !callee_wants_shared_borrow
            && !coerced.ends_with(".clone()")
            && !coerced.starts_with('&')
            && !coerced.starts_with("&mut ")
            && (self.current_fn_forward_ref_if_params.contains(name)
                || self.current_fn_mixed_forwarder_params.contains(name))
        {
            // Bare owned formals move by value. Only clone when auto-clone says this
            // statement reuses the binding — never unconditional `.clone()` on
            // `local.merge(remote)` / other single-use owned method args.
            let needs_reuse = self
                .auto_clone_analysis
                .as_ref()
                .is_some_and(|a| a.needs_clone(name, self.current_statement_idx).is_some());
            if needs_reuse {
                *coerced = format!("{coerced}.clone()");
            }
        } else if callee_wants_shared_borrow
            && !callee_wants_owned
            && !coerced.starts_with('&')
            && !self.callee_call_uses_rust_auto_borrow_for_owned_struct(arg_expr)
        {
            *coerced = format!("&{coerced}");
        }
    }
}

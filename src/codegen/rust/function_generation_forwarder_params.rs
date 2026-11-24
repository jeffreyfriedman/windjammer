//! Which parameters of a function forward to both owned and borrowing callees (mixed forwarders).

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Fill `current_fn_mixed_forwarder_params` and `current_fn_forward_ref_if_params` for `func`.
    pub(in crate::codegen::rust) fn compute_mixed_forwarder_params(&mut self, func: &FunctionDecl<'ast>) {
        self.current_fn_mixed_forwarder_params.clear();
        self.current_fn_forward_ref_if_params.clear();
        for param in &func.parameters {
            if param.name == "self" {
                continue;
            }
            // Single-arg pure delegates keep owned formals — not mixed forwarders.
            if self.param_is_single_arg_call_only_delegate(param, func) {
                continue;
            }
            let if_branch_facade = self
                .param_used_in_if_with_condition_and_branches(func.body.as_slice(), &param.name)
                || self.param_used_in_if_else_both_branches(func.body.as_slice(), &param.name);
            // Multi-arg single-call delegates borrow at formals — not mixed forwarders.
            if !if_branch_facade
                && self.param_only_used_as_call_argument(func.body.as_slice(), &param.name, func)
                && self.count_non_self_params(func) >= 2
            {
                continue;
            }
            let passes_owned =
                self.param_passes_to_wj_owned_sibling_call(func.body.as_slice(), &param.name, func)
                    || self.param_passed_to_owned_self_method_arg(
                        func.body.as_slice(),
                        &param.name,
                        func,
                    );
            let passes_borrow =
                self.param_passed_to_borrowing_callee(func.body.as_slice(), &param.name, func);
            if (passes_owned && passes_borrow) || if_branch_facade {
                self.current_fn_mixed_forwarder_params
                    .insert(param.name.clone());
            }
            if if_branch_facade {
                self.current_fn_forward_ref_if_params
                    .insert(param.name.clone());
            } else if self.body_forwards_param_in_if_condition(&param.name, func)
                || self.param_has_forward_ref_keep_owned(func.body.as_slice(), &param.name, func)
            {
                self.current_fn_forward_ref_if_params
                    .insert(param.name.clone());
            }
        }
    }
}

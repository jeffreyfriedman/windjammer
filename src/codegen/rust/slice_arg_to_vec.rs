//! A slice-typed call argument at a `Vec` formal is copied into an owned `Vec` (split out of `ir_call_site.rs`).

use super::generator::CodeGenerator;
use crate::parser::{Expression, Type};

impl<'ast> CodeGenerator<'ast> {
    /// A call argument the declarations type as a slice (`"abc".as_bytes()` returns `&[u8]`, declared `&Vec<u8>`)
    /// passed to a `Vec<T>` / `&Vec<T>` formal: Rust needs an owned `Vec` there, so the slice is copied with
    /// `.to_vec()` (and borrowed again when the formal is a reference). Decided from the argument's declared type
    /// and the formal, not from the method's name.
    pub(crate) fn convert_slice_arg_to_vec_formal(&self, arg_expr: &Expression<'ast>, sig: &crate::analyzer::FunctionSignature, arg_index: usize, arg_str: &mut String) {
        let pidx = sig.arg_param_index(arg_index);
        let Some(formal) = sig.formal_param_type(pidx) else { return };
        if !matches!(arg_expr, Expression::MethodCall { .. }) {
            return;
        }
        let (declared_ref, formal_bare) = match formal {
            Type::Reference(inner) => (true, inner.as_ref()),
            other => (false, other),
        };
        // the emitted formal: a body-inferred borrow makes `Vec<u8>` a `&Vec<u8>`
        let formal_is_ref = declared_ref
            || crate::ir::emission_contract::callee_emits_shared_rust_ref_param(sig, pidx)
            || crate::ir::signature_bridge::call_site_needs_shared_ref_at_emit(sig, pidx);
        if !matches!(formal_bare, Type::Vec(_)) {
            return;
        }
        let Some(Type::Reference(slice)) = self.infer_expression_type(arg_expr) else { return };
        if !matches!(slice.as_ref(), Type::Vec(_)) || arg_str.ends_with(".to_vec()") {
            return;
        }
        let mut base = crate::codegen::rust::expression_utilities::borrow_base_expr(arg_str).to_string();
        crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut base);
        *arg_str = format!("{}{base}.to_vec()", if formal_is_ref { "&" } else { "" });
    }

    /// Each argument of a call is checked against the callee's `Vec` formals ([`Self::convert_slice_arg_to_vec_formal`]).
    pub(crate) fn convert_slice_args_to_vec_formals(
        &self,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        sig: Option<&crate::analyzer::FunctionSignature>,
        processed_args: &mut [String],
    ) {
        let Some(sig) = sig else { return };
        for (i, arg_str) in processed_args.iter_mut().enumerate() {
            if let Some((_, arg_expr)) = arguments.get(i) {
                self.convert_slice_arg_to_vec_formal(arg_expr, sig, i, arg_str);
            }
        }
    }

    /// An iterator chain, or a collection adapter called on the collection itself, yielding `Option<&T>` where the
    /// function returns `Option<T>`.
    pub(crate) fn owned_return_needs_cloned(&self, object: &Expression<'ast>, method: &str) -> bool {
        self.find_needs_cloned_for_owned_return(object) || self.collection_adapter_needs_cloned_for_owned_return(object, method)
    }
}

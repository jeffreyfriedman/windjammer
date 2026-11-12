//! `map.get(key).unwrap_or("")` over a map of owned text: the value is a borrowed `&String` in Rust and an
//! owned piece of text in Windjammer, so a default that is owned text is applied to a clone of the stored value.

use crate::parser::*;

use crate::codegen::rust::CodeGenerator;
use crate::ir::stdlib_tables::CoreSurface;

impl<'ast> CodeGenerator<'ast> {
    /// A call on an `Option<&T>` whose declared parameter (`std/rust_core.wji`) is the option's payload, given an owned
    /// `T` (`map.get(k).unwrap_or("")`, `outer.get(k).unwrap_or(HashMap::new())`): the call yields an owned `T`,
    /// the stored value cloned. `T` is text or a standard collection. Returns that owned type.
    pub(in crate::codegen::rust) fn ref_default_call_type(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> Option<Type> {
        let [(_, arg)] = arguments else { return None };
        let literal_text = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
        let new_collection = matches!(arg, Expression::Call { function, arguments, .. }
            if arguments.is_empty() && matches!(function, Expression::FieldAccess { .. } | Expression::Identifier { .. }));
        if !literal_text && !new_collection && !matches!(self.infer_expression_type(arg), Some(Type::String)) {
            return None;
        }
        let recv = self.infer_expression_type(object)?;
        let Type::Option(payload) = &recv else { return None };
        let Type::Reference(target) = payload.as_ref() else { return None };
        let head_matches = |t: &Type| crate::type_classification::type_to_registry_base(t) == crate::type_classification::type_to_registry_base(target);
        let owned_default = if new_collection {
            self.infer_expression_type(arg).is_some_and(|t| head_matches(&t))
        } else {
            crate::codegen::rust::types::is_windjammer_text_type(target)
        };
        let cloneable = crate::codegen::rust::types::is_windjammer_text_type(target)
            || crate::type_classification::type_is_vec_container(target)
            || matches!(target.as_ref(), Type::Parameterized(n, _) | Type::Custom(n) if crate::type_classification::is_map_type_name(n));
        (owned_default && cloneable && CoreSurface::parse().method_signature(&recv, method, &[])?.params.first() == Some(payload.as_ref()))
            .then(|| target.as_ref().clone())
    }

    /// `opt.unwrap_or(owned)` over `Option<&T>` becomes `opt.cloned().unwrap_or(owned)`.
    pub(super) fn mc_lower_ref_text_option_default(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        args: &[String],
        obj_str: &str,
    ) -> Option<String> {
        if args.len() != 1 || obj_str.ends_with(".cloned()") || self.ref_default_call_type(object, method, arguments).is_none() {
            return None;
        }
        Some(format!("{obj_str}.cloned().{method}({})", args[0]))
    }
}

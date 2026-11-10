//! `map.get(key).unwrap_or("")` over a map of owned text: the value is a borrowed `&String` in Rust and an
//! owned piece of text in Windjammer, so a default that is owned text is applied to a clone of the stored value.

use crate::parser::*;

use crate::codegen::rust::CodeGenerator;
use crate::ir::stdlib_tables::CoreSurface;

impl<'ast> CodeGenerator<'ast> {
    /// A call on an `Option<&String>` whose declared parameter (`std/rust_core.wji`) is the option's payload,
    /// given owned text, becomes `opt.cloned().<call>(text)`.
    pub(super) fn mc_lower_ref_text_option_default(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        args: &[String],
        obj_str: &str,
    ) -> Option<String> {
        let [(_, arg)] = arguments else { return None };
        if args.len() != 1 || obj_str.ends_with(".cloned()") {
            return None;
        }
        let recv = self.infer_expression_type(object)?;
        let Type::Option(payload) = &recv else { return None };
        let Type::Reference(target) = payload.as_ref() else { return None };
        if !crate::codegen::rust::types::is_windjammer_text_type(target) {
            return None;
        }
        let owned_text = matches!(arg, Expression::Literal { value: Literal::String(_), .. })
            || matches!(self.infer_expression_type(arg), Some(Type::String));
        if !owned_text {
            return None;
        }
        let sig = CoreSurface::parse().method_signature(&recv, method, &[])?;
        (sig.params.first() == Some(payload.as_ref())).then(|| format!("{obj_str}.cloned().{method}({})", args[0]))
    }
}

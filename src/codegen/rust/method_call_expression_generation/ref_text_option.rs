//! `map.get(key).unwrap_or("")` over a map of owned text: the value is a borrowed `&String` in Rust and an
//! owned piece of text in Windjammer, so the default is applied to a clone of the stored value.

use crate::parser::*;

use crate::codegen::rust::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// `opt.unwrap_or(default)` where `opt: Option<&String>` becomes `opt.cloned().unwrap_or(default)`.
    pub(super) fn mc_lower_ref_text_option_unwrap_or(
        &self,
        object: &Expression<'ast>,
        method: &str,
        args: &[String],
        obj_str: &str,
    ) -> Option<String> {
        if method != "unwrap_or" || args.len() != 1 || obj_str.ends_with(".cloned()") {
            return None;
        }
        let Some(Type::Option(inner)) = self.infer_expression_type(object) else { return None };
        let Type::Reference(target) = inner.as_ref() else { return None };
        crate::codegen::rust::types::is_windjammer_text_type(target)
            .then(|| format!("{obj_str}.cloned().unwrap_or({})", args[0]))
    }
}

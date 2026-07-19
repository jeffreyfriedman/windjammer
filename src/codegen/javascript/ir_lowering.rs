//! JavaScript backend IR integration.

use crate::ir::{analyze_and_lower, resolve_call_arg_actual_type, safety_type_from_signature_param, IrContext, SafetyType};
use crate::ir::target_encodings::{encode_call_argument, Target};
use crate::parser::Program;

/// Run shared IR analysis for JS codegen.
pub fn prepare_ir_context<'ast>(program: &Program<'ast>) -> Option<IrContext<'ast>> {
    analyze_and_lower(program).ok()
}

/// Encode a call argument for JavaScript using IR coercion rules.
pub fn encode_js_call_argument(
    ctx: &IrContext<'_>,
    callee_name: &str,
    arg_index: usize,
    arg_str: &str,
) -> Option<String> {
    let sig = ctx
        .registry
        .get_signature(callee_name)
        .or_else(|| ctx.registry.lookup_method(callee_name))?;

    let param_idx = sig.arg_param_index(arg_index);
    let expected = safety_type_from_signature_param(sig, param_idx);
    let actual = resolve_call_arg_actual_type(&ctx.module, arg_str);
    Some(encode_call_argument(
        &actual,
        &expected,
        Target::JavaScript,
        arg_str,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::parse_wj_source;
    use std::path::Path;

    #[test]
    fn js_ir_context_smoke() {
        let (_parser, program) =
            parse_wj_source(Path::new("test.wj"), "pub fn f() -> i32 { 1 }").expect("parse");
        let ctx = prepare_ir_context(&program).expect("IR pipeline should run");
        assert!(!ctx.module.functions.is_empty());
    }
}

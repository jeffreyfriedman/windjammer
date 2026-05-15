//! `Call(FieldAccess)` lowering: treat as method call with signature-aware arguments.

use crate::parser::*;

use super::super::CodeGenerator;
use super::argument_generation;

/// Parser sometimes emits `Call { function: FieldAccess { .. }, args }` instead of `MethodCall`.
#[allow(clippy::too_many_arguments)]
pub(in crate::codegen::rust) fn generate_call_on_field_access<'ast>(
    gen: &mut CodeGenerator<'ast>,
    call_obj: &'ast Expression<'ast>,
    call_method: &str,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> String {
    let prev_explicit_clone = gen.in_explicit_clone_call;
    if call_method == "clone" {
        gen.in_explicit_clone_call = true;
    }
    let mut obj_str = gen.generate_expression(call_obj);
    gen.in_explicit_clone_call = prev_explicit_clone;

    if call_method == "clone" && obj_str.ends_with(".clone()") {
        obj_str = obj_str[..obj_str.len() - 8].to_string();
    }

    let type_name = gen.infer_type_name(call_obj);
    let method_signature = type_name
        .as_ref()
        .map(|tn| format!("{}::{}", tn, call_method))
        .and_then(|q| gen.signature_registry.get_signature(&q).cloned())
        .or_else(|| {
            if let Expression::Identifier { name: mod_name, .. } = call_obj {
                let qualified = format!("{}::{}", mod_name, call_method);
                if let Some(sig) = gen.signature_registry.get_signature(&qualified) {
                    return Some(sig.clone());
                }
            }
            if super::super::stdlib_method_traits::is_common_stdlib_method(call_method) {
                None
            } else {
                gen.signature_registry.get_signature(call_method).cloned()
            }
        });

    let args: Vec<String> = if let Some(ref sig) = method_signature {
        argument_generation::field_access_method_args_with_signature(
            gen,
            sig,
            call_method,
            &method_signature,
            &type_name,
            arguments,
        )
    } else {
        argument_generation::field_access_method_args_fallback(
            gen,
            call_method,
            &type_name,
            arguments,
        )
    };

    let call_str = format!("{}.{}({})", obj_str, call_method, args.join(", "));

    let is_extern_call = method_signature.as_ref().is_some_and(|sig| sig.is_extern)
        || gen
            .signature_registry
            .get_signature(call_method)
            .is_some_and(|sig| sig.is_extern)
        || gen.extern_function_names.contains(call_method);

    if is_extern_call && !gen.in_unsafe_block {
        format!("(unsafe {{ {} }})", call_str)
    } else {
        call_str
    }
}

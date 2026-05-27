//! `Call(FieldAccess)` lowering: treat as method call with signature-aware arguments.

use crate::analyzer::OwnershipMode;
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

    let runtime_module = match call_obj {
        Expression::Identifier { name, .. } if gen.is_imported_runtime_std_module(name) => {
            Some(name.as_str())
        }
        _ => None,
    };

    let mut args: Vec<String> = if let Some(ref sig) = method_signature {
        argument_generation::field_access_method_args_with_signature(
            gen,
            sig,
            call_method,
            &method_signature,
            &type_name,
            runtime_module,
            arguments,
        )
    } else {
        argument_generation::field_access_method_args_fallback(
            gen,
            call_method,
            &type_name,
            runtime_module,
            arguments,
        )
    };

    // Post-process module-qualified calls: borrow owned String args when the registry
    // says the callee takes `string` by borrow (lowers to `&str` in Rust).
    let effective_sig = method_signature.clone().or_else(|| {
        gen.signature_registry.get_signature(call_method).cloned()
    });
    if let Some(ref sig) = effective_sig {
        let callee_is_extern = sig.is_extern
            || gen
                .signature_registry
                .get_signature(call_method)
                .is_some_and(|s| s.is_extern);
        args = args
            .iter()
            .enumerate()
            .map(|(i, arg_str)| {
                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                let borrow = !callee_is_extern
                    && !arg_str.contains("string_to_ffi(")
                    && sig
                        .param_ownership
                        .get(sig_param_idx)
                        .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed))
                    && sig.param_types.get(sig_param_idx).is_some_and(
                        crate::codegen::rust::types::is_windjammer_text_type,
                    );
                if borrow && !arg_str.starts_with('&') && !arg_str.starts_with('"') {
                    format!("&{arg_str}")
                } else {
                    arg_str.clone()
                }
            })
            .collect();
    }

    // Type constructors: Vec::new(), HashMap::with_capacity() — not instance methods.
    let separator = match call_obj {
        Expression::Identifier { name, .. } => {
            if CodeGenerator::is_enum_variant_qualified_path(name)
                || name.chars().next().is_some_and(|c| c.is_uppercase())
                || gen.is_imported_runtime_std_module(name)
            {
                "::"
            } else {
                "."
            }
        }
        _ => ".",
    };
    let call_str = format!("{}{}{}({})", obj_str, separator, call_method, args.join(", "));

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

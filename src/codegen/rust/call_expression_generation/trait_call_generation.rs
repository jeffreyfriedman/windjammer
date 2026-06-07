//! `Call(FieldAccess)` lowering: treat as method call with signature-aware arguments.

use crate::analyzer::OwnershipMode;
use crate::codegen::rust::call_signature_resolution;
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
    if matches!(call_method, "clone" | "to_owned" | "to_vec" | "into_iter") {
        gen.in_explicit_clone_call = true;
    }
    let mut obj_str = gen.generate_expression(call_obj);
    gen.in_explicit_clone_call = prev_explicit_clone;

    if matches!(call_method, "clone" | "to_owned" | "to_vec" | "into_iter")
        && obj_str.ends_with(".clone()")
    {
        obj_str = obj_str[..obj_str.len() - 8].to_string();
    }

    let type_name = gen.infer_type_name(call_obj);

    // Build qualified name for the resolver. Try inferred type first,
    // then fall back to the identifier itself (handles both `Emitter::new`
    // where Emitter is a type, and `gpu::load_shader` where gpu is a module).
    let qualified_name = type_name
        .as_ref()
        .map(|tn| format!("{}::{}", tn, call_method))
        .or_else(|| {
            if let Expression::Identifier { name, .. } = call_obj {
                Some(format!("{}::{}", name, call_method))
            } else {
                None
            }
        });

    let resolved = qualified_name.as_deref().and_then(|name| {
        call_signature_resolution::resolve_call_signature(
            &gen.signature_registry,
            name,
            type_name.as_deref(),
            arguments.len(),
            &gen.module_alias_map,
        )
        // Reject suffix matches when we have a type qualifier — finding
        // OtherType::new when we asked for Emitter::new is wrong.
        .filter(|r| {
            !matches!(
                r.resolution_method,
                call_signature_resolution::ResolutionMethod::ArgCountValidated
            )
        })
    });
    let method_signature = resolved.as_ref().map(|r| r.sig.clone());

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

    // Borrow owned String args when the resolved signature says the callee
    // takes `string` by borrow (lowers to `&str` in Rust).
    if let Some(ref sig) = method_signature {
        let callee_is_extern = sig.is_extern;
        args = args
            .iter()
            .enumerate()
            .map(|(i, arg_str)| {
                let sig_param_idx = sig.arg_param_index(i);
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
                    let arg_is_str_param = arguments.get(i).is_some_and(|(_, arg_expr)| {
                        if let Expression::Identifier { name, .. } = *arg_expr {
                            gen.identifier_already_ref(name)
                        } else if let Expression::Unary { op: UnaryOp::Ref, operand, .. } = *arg_expr {
                            if let Expression::Identifier { name, .. } = &**operand {
                                gen.identifier_already_ref(name)
                            } else { false }
                        } else { false }
                    });
                    if arg_is_str_param {
                        arg_str.clone()
                    } else {
                        format!("&{arg_str}")
                    }
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
        || gen.extern_function_names.contains(call_method);

    if is_extern_call && !gen.in_unsafe_block {
        format!("(unsafe {{ {} }})", call_str)
    } else {
        call_str
    }
}

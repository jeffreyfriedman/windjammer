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

    let resolved = type_name
        .as_ref()
        .and_then(|tn| {
            gen.lookup_method_signature_on_receiver_type(tn, call_method, arguments.len())
                .map(|sig| call_signature_resolution::ResolvedSignature {
                    sig,
                    qualified_key: format!("{tn}::{call_method}"),
                    resolution_method: call_signature_resolution::ResolutionMethod::MethodRegistry,
                    has_collision: false,
                })
        })
        .or_else(|| {
            qualified_name.as_deref().and_then(|name| {
                gen.resolve_call_signature_with_global(name, type_name.as_deref(), arguments.len())
                    .filter(|r| match r.resolution_method {
                        call_signature_resolution::ResolutionMethod::ArgCountValidated => type_name
                            .as_ref()
                            .is_some_and(|tn| {
                                call_signature_resolution::arg_count_validated_matches_receiver(
                                    &r.qualified_key,
                                    tn,
                                    call_method,
                                )
                            }),
                        _ => true,
                    })
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
                    && matches!(
                        crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                            sig, i,
                        ),
                        OwnershipMode::Borrowed,
                    )
                    && sig
                        .param_types
                        .get(sig_param_idx)
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
                let arg_is_copy_scalar = arguments.get(i).is_some_and(|(_, arg_expr)| {
                    if let Some(t) = gen.infer_expression_type(arg_expr) {
                        gen.is_type_copy(&t)
                            && !crate::codegen::rust::types::is_windjammer_text_type(&t)
                    } else if let Expression::Identifier { name, .. } = *arg_expr {
                        gen.current_function_params
                            .iter()
                            .find(|p| p.name == *name)
                            .is_some_and(|p| {
                                gen.is_type_copy(&p.type_)
                                    && !crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                            })
                    } else {
                        false
                    }
                });
                let arg_is_text_compatible = arguments.get(i).is_some_and(|(_, arg_expr)| {
                    gen.infer_expression_type(arg_expr)
                        .as_ref()
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                });
                if borrow
                    && arg_is_text_compatible
                    && !arg_is_copy_scalar
                    && !arg_str.starts_with('&')
                    && !arg_str.starts_with('"')
                {
                    let arg_is_str_param = arguments.get(i).is_some_and(|(_, arg_expr)| {
                        if let Expression::Identifier { name, .. } = *arg_expr {
                            gen.identifier_already_ref(name)
                        } else if let Expression::Unary {
                            op: UnaryOp::Ref,
                            operand,
                            ..
                        } = *arg_expr
                        {
                            if let Expression::Identifier { name, .. } = &**operand {
                                gen.identifier_already_ref(name)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
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
    let call_str = format!(
        "{}{}{}({})",
        obj_str,
        separator,
        call_method,
        args.join(", ")
    );

    let is_extern_call = method_signature.as_ref().is_some_and(|sig| sig.is_extern)
        || gen.extern_function_names.contains(call_method);

    if is_extern_call && !gen.in_unsafe_block {
        format!("(unsafe {{ {} }})", call_str)
    } else {
        call_str
    }
}

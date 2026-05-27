//! Plain function call lowering (after `Call(FieldAccess)` is handled elsewhere).

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::CodeGenerator;

#[allow(clippy::too_many_lines)]
pub(in crate::codegen::rust) fn generate_plain_function_call<'ast>(
    gen: &mut CodeGenerator<'ast>,
    func_name: &str,
    function: &Expression<'ast>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
) -> String {
    let mut func_str = gen.generate_expression(function);

    // Windjammer stdlib type mapping: Map::method → HashMap::method
    if func_str.starts_with("Map::") {
        func_str = func_str.replacen("Map::", "HashMap::", 1);
    }

    // E0282 turbofish: Vec::new() / HashSet::new() → Vec::<T>::new() / HashSet::<T>::new()
    // when the function return type provides the element type.
    // Skip when suppress_collection_turbofish is set (let binding already has type ascription).
    if arguments.is_empty() && !gen.suppress_collection_turbofish {
        if func_str == "Vec::new" {
            if let Some(Type::Vec(inner)) = &gen.current_function_return_type {
                func_str = format!("Vec::<{}>::new", gen.type_to_rust(inner));
            }
        } else if func_str == "HashSet::new" {
            if let Some(Type::Parameterized(base, args)) = &gen.current_function_return_type {
                if base == "HashSet" && args.len() == 1 {
                    func_str = format!("HashSet::<{}>::new", gen.type_to_rust(&args[0]));
                }
            }
        } else if func_str == "HashMap::new" {
            if let Some(Type::Parameterized(base, args)) = &gen.current_function_return_type {
                if base == "HashMap" && args.len() == 2 {
                    func_str = format!(
                        "HashMap::<{}, {}>::new",
                        gen.type_to_rust(&args[0]),
                        gen.type_to_rust(&args[1])
                    );
                }
            }
        }
    }

    // In an impl block, bare function calls to sibling methods need qualified dispatch.
    // Instance methods (take self) → self.method(args)
    // Static methods → Self::method(args)
    if gen.in_impl_block
        && !func_name.contains("::")
        && gen.current_impl_methods.contains(func_name)
    {
        if gen.current_impl_instance_methods.contains(func_name) {
            func_str = format!("self.{}", func_str);
        } else {
            func_str = format!("Self::{}", func_str);
        }
    }

    // E0282 turbofish: Some(expr) → Some::<T>(expr)
    // Only needed when the type parameter is truly ambiguous
    // (e.g. numeric literals outside a typed context). In return
    // position or when the inner type involves references/structs,
    // Rust infers the type from the function signature.
    if func_str == "Some" && arguments.len() == 1 {
        if let Some(Type::Option(inner)) = &gen.current_function_return_type {
            let inner_rust = gen.type_to_rust(inner);
            let is_ambiguous_primitive = matches!(
                inner.as_ref(),
                Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
            );
            if is_ambiguous_primitive {
                func_str = format!("Some::<{}>", inner_rust);
            }
        }
    }

    // WINDJAMMER PHILOSOPHY: Some/Ok/Err with string literals need .to_string()
    // Some("literal") -> Some("literal".to_string())
    // Ok("literal") -> Ok("literal".to_string())
    // Err("literal") -> Err("literal".to_string())
    // Also: Some(borrowed_iterator_var) -> Some(borrowed_iterator_var.clone())

    // TDD FIX (Bug #2): Detect ALL enum constructors AND tuple struct constructors
    // Pattern: Some/Ok/Err, Module::Variant, or TupleStruct(args)
    let is_std_enum = matches!(func_name, "Some" | "Ok" | "Err");
    let is_custom_enum = func_name.contains("::") && {
        let parts: Vec<&str> = func_name.split("::").collect();
        parts.len() == 2
            && parts[0].chars().next().is_some_and(|c| c.is_uppercase())
            && parts[1].chars().next().is_some_and(|c| c.is_uppercase())
    };
    // Tuple struct constructors: Point(x, y), Id(42)
    // Uppercase name without :: that is a known tuple struct
    let is_tuple_struct_constructor = !is_std_enum
        && !is_custom_enum
        && !func_name.contains("::")
        && func_name.chars().next().is_some_and(|c| c.is_uppercase())
        && gen.tuple_struct_names.contains(func_name);

    if is_std_enum || is_custom_enum || is_tuple_struct_constructor {
        // Enum variant constructors need owned values (Some(T), Ok(T), Err(E)).
        // Set owned context so index expressions use .clone() instead of &,
        // BUT only for arguments that aren't already explicit references.
        let prev_owned_context = gen.in_owned_value_context;
        let generated_args: Vec<String> = arguments
            .iter()
            .map(|(_label, arg)| {
                let is_explicit_ref = matches!(
                    arg,
                    Expression::Unary {
                        op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                        ..
                    }
                );
                if !is_explicit_ref {
                    gen.in_owned_value_context = true;
                }
                let result = gen.generate_expression(arg);
                gen.in_owned_value_context = prev_owned_context;
                result
            })
            .collect();

        let has_format_arg = generated_args
            .iter()
            .any(|arg_str| arg_str.contains("format!("));

        if has_format_arg {
            // Extract format!() macros to temp variables
            let mut temp_decls = String::new();
            let mut temp_counter = 0;
            let fixed_args: Vec<String> = generated_args
                .iter()
                .map(|arg_str| {
                    if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                        // Strip leading & if present
                        let format_expr = if arg_str.starts_with("&") {
                            arg_str.strip_prefix("&").unwrap()
                        } else {
                            arg_str
                        };
                        // Extract to temp var
                        let temp_name = format!("_temp{}", temp_counter);
                        temp_counter += 1;
                        temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));

                        // TDD FIX: Don't add & for owned parameters
                        // Err(format!(...)) should be Err(_temp0), not Err(&_temp0)
                        // Original arg didn't have &, so pass owned value
                        if arg_str.starts_with("&") {
                            format!("&{}", temp_name)
                        } else {
                            temp_name
                        }
                    } else {
                        arg_str.clone()
                    }
                })
                .collect();

            return format!(
                "{{ {}{}({}) }}",
                temp_decls,
                func_str,
                fixed_args.join(", ")
            );
        }

        let args: Vec<String> = generated_args
            .iter()
            .enumerate()
            .map(|(i, arg_str)| {
                // Get the original argument expression for type checking
                let arg = &arguments[i].1;
                let result = arg_str.clone();

                // Auto-convert string literals to String for Option/Result wrappers
                if matches!(
                    arg,
                    Expression::Literal {
                        value: Literal::String(_),
                        ..
                    }
                ) {
                    format!("{}.to_string()", result)
                } else if let Expression::Identifier { name, .. } = arg {
                    // BUGFIX: Don't clone if function returns Option<&T>, Option<&mut T>, or Result<&T, E>
                    // When returning Option<&Squad>, Some(squad) should NOT become Some(squad.clone())

                    // Check if return type is Option<&T> or Option<&mut T> (reference inside)
                    let returns_option_ref = match &gen.current_function_return_type {
                        Some(Type::Option(inner_type)) => {
                            matches!(**inner_type, Type::Reference(_) | Type::MutableReference(_))
                        }
                        _ => false,
                    };

                    // Check if return type is Result<&T, E> or Result<&mut T, E>
                    let returns_result_ref = match &gen.current_function_return_type {
                        Some(Type::Result(ok_type, _err_type)) => {
                            matches!(**ok_type, Type::Reference(_) | Type::MutableReference(_))
                        }
                        _ => false,
                    };

                    // AUTO-CONVERT: Borrowed variables in enum constructors need
                    // ownership conversion since the wrapper takes ownership.
                    // &str params → .to_string(), other borrowed → .clone()
                    // UNLESS returning Option<&T>, Result<&T, E>, etc.
                    if !returns_option_ref
                        && !returns_result_ref
                        && !result.ends_with(".clone()")
                        && !result.ends_with(".to_string()")
                        && !result.trim_start().starts_with('*')
                    {
                        if gen.str_ref_optimized_params.contains(name.as_str()) {
                            format!("{}.to_string()", result)
                        } else if gen.borrowed_iterator_vars.contains(name)
                            || gen.inferred_borrowed_params.contains(name.as_str())
                        {
                            format!("{}.clone()", result)
                        } else {
                            result
                        }
                    } else {
                        result
                    }
                } else {
                    result
                }
            })
            .collect();
        return format!("{}({})", func_str, args.join(", "));
    }

    // Look up signature and clone it to avoid borrow conflicts
    // THE WINDJAMMER WAY: Try qualified name first, then simple name
    // e.g., "Sound::new" -> try "Sound::new", then "new"

    // TDD FIX: Function pointer signature extraction
    // When calling a function pointer parameter (e.g., has_item(arg1, arg2)),
    // extract the signature from the parameter's type instead of the registry
    let mut signature = if let Some(param) = gen
        .current_function_params
        .iter()
        .find(|p| p.name == func_name)
    {
        // Check if this parameter is a function pointer
        if let Type::FunctionPointer {
            params,
            return_type,
        } = &param.type_
        {
            // TDD FIX: Build signature from function pointer type
            // CRITICAL: Match the conversion logic in types.rs type_to_rust()!
            // fn(string, i32) in Windjammer → fn(&String, i32) in Rust
            //
            // Conversion rules (from types.rs lines 148-160):
            // - Type::String → "&String" → Borrowed
            // - Type::Custom("string") → "&String" → Borrowed
            // - Type::Reference(_) → "&T" → Borrowed
            // - Copy types (Int, Bool, etc.) → owned → Owned
            // - Everything else → as-is (keep explicit types)
            let param_ownership: Vec<OwnershipMode> = params
                .iter()
                .map(|ty| {
                    match ty {
                        // Idiomatic Windjammer: string parameters are borrowed (types.rs:151)
                        Type::String => OwnershipMode::Borrowed,
                        Type::Custom(name) if name == "string" => OwnershipMode::Borrowed,
                        // Explicit references - borrowed (types.rs:154)
                        Type::Reference(_) | Type::MutableReference(_) => OwnershipMode::Borrowed,
                        // Copy types - owned (types.rs:156-157)
                        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => {
                            OwnershipMode::Owned
                        }
                        Type::Custom(name)
                            if matches!(
                                name.as_str(),
                                "i32"
                                    | "i64"
                                    | "u32"
                                    | "u64"
                                    | "f32"
                                    | "f64"
                                    | "bool"
                                    | "char"
                                    | "usize"
                                    | "isize"
                            ) =>
                        {
                            OwnershipMode::Owned
                        }
                        // Everything else - keep as-is (types.rs:159)
                        // For non-Copy custom types, default is as-is, which means Owned in this context
                        // (the analyzer will have determined the correct type already)
                        _ => OwnershipMode::Owned,
                    }
                })
                .collect();

            Some(crate::analyzer::FunctionSignature {
                name: func_name.to_string(),
                param_types: params.clone(),
                param_ownership,
                return_type: return_type.as_ref().map(|t| (**t).clone()),
                return_ownership: OwnershipMode::Owned, // Functions return owned by default
                has_self_receiver: false,
                is_extern: false,
            })
        } else {
            // Not a function pointer - try registry
            gen.signature_registry.get_signature(func_name).cloned()
        }
    } else {
        // Not a parameter - try registry lookup
        let direct = gen.signature_registry.get_signature(func_name).cloned();
        direct.or_else(|| {
            if let Some(pos) = func_name.rfind("::") {
                let qualifier = &func_name[..pos];
                let simple_name = &func_name[pos + 2..];
                let is_type_qualifier = qualifier.chars().next().is_some_and(|c| c.is_uppercase());
                if is_type_qualifier {
                    gen.signature_registry.get_signature(simple_name).cloned()
                } else {
                    // For module-qualified calls (e.g., draw::draw_text),
                    // try progressively shorter qualified names.
                    // Do NOT fall back to simple name - it may collide
                    // with a different module's function with the same name.
                    let parts: Vec<&str> = func_name.split("::").collect();
                    let mut found = None;
                    for start in (0..parts.len().saturating_sub(1)).rev() {
                        let candidate = parts[start..].join("::");
                        if let Some(sig) = gen.signature_registry.get_signature(&candidate) {
                            found = Some(sig.clone());
                            break;
                        }
                    }
                    found
                }
            } else {
                None
            }
        })
    };

    // For module-qualified calls (e.g., gpu::load_compute_shader_from_file),
    // the signature lookup above may fail. Try resolving through module aliases
    // first (e.g., `use crate::ffi::gpu_safe as gpu` → try gpu_safe::func),
    // then fall back to the simple name.
    let mut signature_from_simple_fallback = false;
    if signature.is_none() && func_name.contains("::") {
        let qualifier = func_name.split("::").next().unwrap_or("");
        let simple = func_name.rsplit("::").next().unwrap_or(func_name);

        // Try resolving through module alias map first
        if let Some(original_module) = gen.module_alias_map.get(qualifier) {
            let resolved_name = format!("{}::{}", original_module, simple);
            if let Some(resolved_sig) = gen.signature_registry.get_signature(&resolved_name) {
                signature = Some(resolved_sig.clone());
            }
        }

        // If alias resolution didn't work, try simple-name fallback
        // with arg count validation to avoid name collisions.
        if signature.is_none() {
            if let Some(found) = gen
                .signature_registry
                .find_signature_by_name_and_arg_count(simple, arguments.len())
            {
                signature = Some(found.clone());
                signature_from_simple_fallback = true;
            }
        }
    }

    // Check if this is an extern function call for unsafe wrapping + FFI str handling.
    // TDD FIX: When a signature was found via simple-name fallback for a
    // module-qualified call (e.g. vnode_ffi::vnode_element), suppress extern
    // detection ONLY when the signature is NOT explicitly extern. If the
    // signature has is_extern=true, the function really is extern (e.g.
    // input::input_is_key_pressed) and must be wrapped in unsafe.
    let is_extern_call = if signature_from_simple_fallback && func_name.contains("::") {
        signature.as_ref().is_some_and(|sig| sig.is_extern)
    } else if let Some(ref sig) = signature {
        sig.is_extern
    } else {
        // Module-qualified calls without a signature are same-crate helpers, not extern FFI.
        if func_name.contains("::") {
            false
        } else {
            let simple = func_name.rsplit("::").next().unwrap_or(func_name);
            gen.extern_function_names.contains(simple)
        }
    };

    let mut args: Vec<String> = super::argument_generation::collect_regular_function_arguments(
        gen,
        func_name,
        func_str.as_str(),
        arguments,
        &signature,
        signature_from_simple_fallback,
        is_extern_call,
    );

    // Borrow owned String args when registry says callee takes borrowed `string` (&str in Rust).
    // Never borrow `string_to_ffi(...)` — extern FFI expects owned FfiString.
    if let Some(ref sig) = signature {
        args = args
            .iter()
            .enumerate()
            .map(|(i, arg_str)| {
                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                let borrow = !is_extern_call
                    && !sig.is_extern
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

    // TDD FIX (Bug #3): Extract format!() macros in arguments to temp variables
    // The args vec has already been generated as Rust strings
    // Check if any contain format!() and extract them
    let has_format_arg = args.iter().any(|arg_str| arg_str.contains("format!("));

    // WINDJAMMER FFI: Extern functions returning string use FfiString - wrap with ffi_to_string
    let returns_string = signature
        .as_ref()
        .and_then(|s| s.return_type.as_ref())
        .is_some_and(|t| {
            matches!(t, Type::String)
                || matches!(t, Type::Custom(n) if n == "string" || n == "String")
        });

    // WINDJAMMER PHILOSOPHY: Auto-wrap extern function calls in unsafe blocks
    // THE WINDJAMMER WAY: Users shouldn't have to write `unsafe` manually
    let call_result = if has_format_arg {
        // Extract format!() macros to temp variables
        let mut temp_decls = String::new();
        let mut temp_counter = 0;
        let fixed_args: Vec<String> = args
            .iter()
            .map(|arg_str| {
                if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                    // TDD FIX (Bug #16 COMPLETE): Check if original had & to preserve intent
                    let has_borrow_prefix = arg_str.starts_with("&");
                    // Strip leading & if present
                    let format_expr = if has_borrow_prefix {
                        &arg_str[1..]
                    } else {
                        arg_str
                    };
                    // Extract to temp var
                    let temp_name = format!("_temp{}", temp_counter);
                    temp_counter += 1;
                    temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));

                    // TDD FIX: Only add & if original had it!
                    // format!() returns owned String, so if caller wants owned, pass temp directly
                    // If caller wants borrowed, pass &temp (when original was &format!())
                    if has_borrow_prefix {
                        format!("&{}", temp_name)
                    } else {
                        temp_name
                    }
                } else {
                    arg_str.clone()
                }
            })
            .collect();

        let call_expr = format!("{}({})", func_str, fixed_args.join(", "));

        // Wrap in unsafe block if extern, otherwise regular block
        // Parenthesize so the block can be used as a sub-expression (e.g., in comparisons)
        if is_extern_call && !gen.in_unsafe_block {
            format!("(unsafe {{ {}{}  }})", temp_decls, call_expr)
        } else {
            format!("{{ {}{} }}", temp_decls, call_expr)
        }
    } else {
        // No format!() args - generate normally with optional unsafe wrapper
        let call_str = format!("{}({})", func_str, args.join(", "));
        if is_extern_call && !gen.in_unsafe_block {
            format!("(unsafe {{ {} }})", call_str)
        } else {
            call_str
        }
    };

    // Wrap extern string return with ffi_to_string
    if is_extern_call && returns_string {
        format!("windjammer_runtime::ffi::ffi_to_string({})", call_result)
    } else {
        call_result
    }
}

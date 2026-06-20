//! Plain function call lowering (after `Call(FieldAccess)` is handled elsewhere).

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::super::CodeGenerator;

/// Map static impl calls to `Type::method` + receiver context for signature lookup.
///
/// `Self::method` and `Type::method` (when `Type` is the enclosing impl struct) must both
/// supply receiver type. Without it, `resolve_call_signature` falls through declaration
/// stubs to arg-count suffix matches and mis-lowers borrows (e.g. `grid.clone()` for
/// `FpsCamera::collides_aabb` in library builds).
fn signature_lookup_for_call<'ast>(
    gen: &CodeGenerator<'ast>,
    func_name: &str,
) -> (String, Option<String>) {
    if gen.in_impl_block {
        if let Some(ref tn) = gen.current_struct_name {
            if let Some(method) = func_name.strip_prefix("Self::") {
                return (format!("{tn}::{method}"), Some(tn.clone()));
            }
            if let Some((qualifier, method)) = func_name.rsplit_once("::") {
                if qualifier == tn.as_str()
                    && qualifier.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    && !method.contains("::")
                {
                    return (func_name.to_string(), Some(tn.clone()));
                }
            }
        }
    }
    (func_name.to_string(), None)
}

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

    // Bare `min(a, b)` on floats → Rust float min (no unqualified `min` in scope).
    if func_name == "min" && arguments.len() == 2 {
        use crate::type_inference::FloatType;
        let lc = gen.float_class_for_binary_operand(arguments[0].1);
        let rc = gen.float_class_for_binary_operand(arguments[1].1);
        func_str = match (lc, rc) {
            (Some(FloatType::F64), _) | (_, Some(FloatType::F64)) => "f64::min".to_string(),
            _ => "f32::min".to_string(),
        };
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
                let prev_in_call_arg = gen.in_call_argument_generation;
                gen.in_call_argument_generation = true;
                let result = gen.generate_expression(arg);
                gen.in_call_argument_generation = prev_in_call_arg;
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
                            gen.maybe_auto_clone(name, &result)
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

    // Function pointer signature extraction: when calling a function pointer
    // parameter (e.g., has_item(arg1, arg2)), build the signature from the
    // parameter's type instead of registry lookup.
    let mut signature = gen
        .current_function_params
        .iter()
        .find(|p| p.name == func_name)
        .and_then(|param| {
            if let Type::FunctionPointer {
                params,
                return_type,
            } = &param.type_
            {
                let param_ownership: Vec<OwnershipMode> = params
                    .iter()
                    .map(|ty| match ty {
                        Type::String => OwnershipMode::Borrowed,
                        Type::Custom(name) if name == "string" => OwnershipMode::Borrowed,
                        Type::Reference(_) | Type::MutableReference(_) => OwnershipMode::Borrowed,
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
                        _ => OwnershipMode::Owned,
                    })
                    .collect();

                Some(crate::analyzer::FunctionSignature {
                    name: func_name.to_string(),
                    param_types: params.clone(),
                    formal_param_types: params.clone(),
                    param_ownership,
                    return_type: return_type.as_ref().map(|t| (**t).clone()),
                    return_ownership: OwnershipMode::Owned,
                    has_self_receiver: false,
                    is_extern: false,
                })
            } else {
                None
            }
        });

    // Unified signature resolution: local registry first, then converged library-wide registry.
    let mut resolved_via_fallback = false;
    let (sig_lookup_name, sig_receiver_type) = signature_lookup_for_call(gen, func_name);
    if signature.is_none() {
        if let Some(ref tn) = sig_receiver_type {
            let method = sig_lookup_name.rsplit("::").next().unwrap_or(func_name);
            signature = gen.lookup_method_signature_on_receiver_type(tn, method, arguments.len());
        }
    }
    if signature.is_none() {
        if let Some(r) = gen.resolve_call_signature_with_global(
            &sig_lookup_name,
            sig_receiver_type.as_deref(),
            arguments.len(),
        ) {
            let method = sig_lookup_name.rsplit("::").next().unwrap_or(func_name);
            let accept = sig_receiver_type.as_ref().map_or(true, |tn| {
                crate::codegen::rust::call_signature_resolution::accept_method_resolution_for_receiver(
                    &r, tn, method,
                )
            });
            if accept {
                resolved_via_fallback = matches!(
                    r.resolution_method,
                    crate::codegen::rust::call_signature_resolution::ResolutionMethod::ArgCountValidated
                );
                signature = Some(r.sig);
            }
        }
    }

    // Extern detection: resolved signature is authoritative. For fallback
    // resolutions on module-qualified calls, only trust explicit is_extern.
    let is_extern_call = if resolved_via_fallback && func_name.contains("::") {
        signature.as_ref().is_some_and(|sig| sig.is_extern)
    } else if let Some(ref sig) = signature {
        sig.is_extern
    } else if func_name.contains("::") {
        // Module-qualified call without a resolved signature — check if the
        // base name (after the last `::`) is a known extern function. This
        // handles cross-module extern calls like `api::gpu_create_buffer()`
        // where signature resolution may miss the extern flag.
        let base_name = func_name.rsplit("::").next().unwrap_or(func_name);
        gen.extern_function_names.contains(base_name)
    } else {
        gen.extern_function_names.contains(func_name)
    };

    let mut args: Vec<String> = super::argument_generation::collect_regular_function_arguments(
        gen,
        func_name,
        func_str.as_str(),
        arguments,
        &signature,
        resolved_via_fallback,
        is_extern_call,
    );

    // Borrow owned args when registry says callee takes `&T` / `&mut T`.
    // Never borrow `string_to_ffi(...)` — extern FFI expects owned FfiString.
    if let Some(ref sig) = signature {
        args = args
            .iter()
            .enumerate()
            .map(|(i, arg_str)| {
                if is_extern_call || sig.is_extern || arg_str.contains("string_to_ffi(") {
                    return arg_str.clone();
                }
                let ownership =
                    crate::codegen::rust::call_signature_resolution::effective_param_ownership_for_arg(
                        sig, i,
                    );
                if crate::codegen::rust::call_site_borrow::is_stale_borrow_on_owned_copy_formal(
                    sig, i,
                ) {
                    if let Some((_, arg_expr)) = arguments.get(i) {
                        if let Expression::Identifier { name, .. } = arg_expr {
                            if gen.inferred_borrowed_params.contains(name)
                                || gen.inferred_mut_borrowed_params.contains(name)
                                || gen.identifier_already_ref(name)
                            {
                                return arg_str.clone();
                            }
                        }
                    }
                    let mut s = arg_str.clone();
                    if s.starts_with("&mut ") {
                        s = s.strip_prefix("&mut ").unwrap_or(&s).to_string();
                    } else if s.starts_with('&') {
                        s = s.trim_start_matches('&').to_string();
                    }
                    if !s.ends_with(".clone()") {
                        s = format!("{s}.clone()");
                    }
                    return s;
                }
                match ownership {
                    OwnershipMode::MutBorrowed if !arg_str.starts_with("&mut ") =>
                    {
                        let arg_already_mut_ref = if let Some((_, arg_expr)) = arguments.get(i) {
                            if let Expression::Identifier { name, .. } = arg_expr {
                                gen.identifier_already_mut_ref(name)
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        if arg_already_mut_ref {
                            return arg_str.clone();
                        }
                        let mut s = arg_str.clone();
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut s);
                        if s.starts_with('&') && !s.starts_with("&mut ") {
                            format!("&mut {}", s.trim_start_matches('&'))
                        } else {
                            format!("&mut {s}")
                        }
                    }
                    OwnershipMode::Borrowed
                        if !arg_str.starts_with('&') && !arg_str.starts_with('"') =>
                    {
                        let mut s = arg_str.clone();
                        crate::codegen::rust::expression_utilities::strip_trailing_clone(&mut s);
                        let arg_already_ref = if let Some((_, arg_expr)) = arguments.get(i) {
                            if let Expression::Identifier { name, .. } = arg_expr {
                                gen.identifier_already_ref(name)
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        if arg_already_ref {
                            s
                        } else {
                            format!("&{s}")
                        }
                    }
                    _ => arg_str.clone(),
                }
            })
            .collect();
    }
    let needs_format_temp = |arg_str: &str| -> bool {
        arg_str.contains("format!(")
            || arg_str.contains("write!(&mut __s,")
            || (arg_str.contains("string_to_ffi(")
                && (arg_str.contains("format!(") || arg_str.contains("write!(&mut __s,")))
    };
    let has_format_arg = args.iter().any(|arg_str| needs_format_temp(arg_str));

    /// Strip `string_to_ffi(...)` wrapper for temp extraction of the inner expression.
    fn unwrap_string_to_ffi(arg_str: &str) -> (&str, bool) {
        const PREFIX: &str = "windjammer_runtime::ffi::string_to_ffi(";
        if let Some(rest) = arg_str.strip_prefix(PREFIX) {
            if let Some(inner) = rest.strip_suffix(')') {
                return (inner, true);
            }
        }
        (arg_str, false)
    }

    fn extract_format_like_arg(
        arg_str: &str,
        temp_decls: &mut String,
        temp_counter: &mut i32,
    ) -> Option<String> {
        let (inner, was_ffi) = unwrap_string_to_ffi(arg_str);
        let has_borrow_prefix = inner.starts_with('&');
        let format_expr = if has_borrow_prefix {
            &inner[1..]
        } else {
            inner
        };
        let needs_extract = format_expr.starts_with("format!(")
            || format_expr.starts_with("{") && format_expr.contains("write!(&mut __s,");
        if !needs_extract {
            return None;
        }
        let temp_name = format!("_temp{}", temp_counter);
        *temp_counter += 1;
        temp_decls.push_str(&format!("let {} = {}; ", temp_name, format_expr));
        let pass_expr = if has_borrow_prefix {
            format!("&{}", temp_name)
        } else {
            temp_name
        };
        Some(if was_ffi {
            format!("windjammer_runtime::ffi::string_to_ffi({})", pass_expr)
        } else {
            pass_expr
        })
    }

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
        let mut temp_counter = 0i32;
        let fixed_args: Vec<String> = args
            .iter()
            .map(|arg_str| {
                if let Some(fixed) =
                    extract_format_like_arg(arg_str, &mut temp_decls, &mut temp_counter)
                {
                    fixed
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

//! Call expression generation
//!
//! Handles code generation for function calls including:
//! - Regular function calls
//! - Test macros (assert_*, property_test, etc.)
//! - Print/println macros
//! - Type casting and auto-clone insertion
//! - Parameter type balancing

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::{ast_utilities, expression_helpers, expression_utilities, float_type_utilities, macro_conversion, string_analysis, type_classification_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a function call expression
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn generate_call_expression(
        &mut self,
        function: &Expression<'ast>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> String {
        // Extract function name for signature lookup
        let func_name = ast_utilities::extract_function_name(function);

                // THE WINDJAMMER WAY: User-defined functions always take priority
                // over built-in name mappings. If the user defines a function with
                // the same name as a test macro or runtime function (e.g., their own
                // `assert_approx`), their definition wins. We check the signature
                // registry: if the function exists and is NOT extern, it's user-defined.
                //
                // EXCEPTION: print/println/eprintln/eprint always convert to macros
                // (Rust requires them to be macros, not functions)

                // Try print/println/eprintln macro conversion FIRST (before user-defined check)
                if let Some(print_macro) = self.try_generate_print_macro(&func_name, arguments) {
                    return print_macro;
                }

                let is_user_defined = self
                    .signature_registry
                    .get_signature(&func_name)
                    .map(|sig| !sig.is_extern)
                    .unwrap_or(false);

                if !is_user_defined {
                    // Try test macro conversion first
                    if let Some(macro_call) = self.try_generate_test_macro(&func_name, arguments) {
                        return macro_call;
                    }

                    // Try test runtime function qualification
                    if let Some(qualified_call) = self.try_qualify_test_function(&func_name, arguments) {
                        return qualified_call;
                    }
                }

                // Special case: convert assert() to assert!()
                if func_name == "assert" {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("assert!({})", args.join(", "));
                }

                // TDD FIX: Call(FieldAccess) → method call WITH SIGNATURE LOOKUP
                // When the parser produces Call { function: FieldAccess { object, field }, args }
                // instead of MethodCall { object, method, args }, we need to:
                // 1. Handle it as a method call (not function call)
                // 2. Do signature lookup to get parameter ownership info
                // 3. Apply correct ownership conversions (& vs .clone() etc.)
                //
                // This was the AUTO-CLONE BUG: method calls skipped signature lookup!
                if let Expression::FieldAccess {
                    object: call_obj,
                    field: call_method,
                    ..
                } = function
                {
                    // DOUBLE-CLONE FIX: When the method is .clone(), suppress auto-clone on
                    // the object to prevent .clone().clone(). Same as MethodCall handler.
                    let prev_explicit_clone = self.in_explicit_clone_call;
                    if call_method == "clone" {
                        self.in_explicit_clone_call = true;
                    }
                    let mut obj_str = self.generate_expression(call_obj);
                    self.in_explicit_clone_call = prev_explicit_clone;
                    // DOUBLE-CLONE SAFETY NET: Strip redundant auto-clone from object
                    if call_method == "clone" && obj_str.ends_with(".clone()") {
                        obj_str = obj_str[..obj_str.len() - 8].to_string();
                    }

                    // TDD FIX: Lookup method signature for ownership inference
                    // Prefer `Type::method` (matches MethodCall path) so `HashMap::get` wins over wrong `get`.
                    let type_name = self.infer_type_name(call_obj);
                    let method_signature = type_name
                        .as_ref()
                        .map(|tn| format!("{}::{}", tn, call_method))
                        .and_then(|q| {
                            self.signature_registry.get_signature(&q).cloned()
                        })
                        .or_else(|| {
                            // When `call_obj` is a module identifier (e.g., `draw` in `draw::draw_text`),
                            // infer_type_name returns None. Try module-qualified lookup directly.
                            if let Expression::Identifier { name: mod_name, .. } = call_obj {
                                let qualified = format!("{}::{}", mod_name, call_method);
                                if let Some(sig) = self.signature_registry.get_signature(&qualified)
                                {
                                    return Some(sig.clone());
                                }
                            }
                            if super::stdlib_method_traits::is_common_stdlib_method(&call_method) {
                                None
                            } else {
                                let bare_sig =
                                    self.signature_registry.get_signature(&call_method).cloned();
                                bare_sig
                            }
                        });

                    // Generate arguments with ownership awareness (same logic as regular Call)
                    let args: Vec<String> = if let Some(ref sig) = method_signature {
                        arguments
                            .iter()
                            .enumerate()
                            .flat_map(|(i, (_label, arg))| {
                                let arg_to_generate =
                                    expression_utilities::strip_unary_ref_for_collection_key_arg(&call_method, i, arg);
                                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                                self.coerce_string_literals_to_owned = false;
                                let prev_match_arm_str = self.in_match_arm_needing_string;
                                self.in_match_arm_needing_string = false;
                                let mut arg_str = self.generate_expression(arg_to_generate);
                                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                                self.in_match_arm_needing_string = prev_match_arm_str;

                                // Apply ownership conversion based on signature
                                let sig_param_idx = if sig.has_self_receiver {
                                    i + 1
                                } else {
                                    i
                                };
                                if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                                    match ownership {
                                        OwnershipMode::Borrowed => {
                                            // PHASE 1: Generate &String parameters for correctness
                                            let is_string_literal = matches!(
                                                arg_to_generate,
                                                Expression::Literal {
                                                    value: Literal::String(_),
                                                    ..
                                                }
                                            );
                                            let is_user_closure_param =
                                                if let Expression::Identifier { name, .. } =
                                                    arg_to_generate
                                                {
                                                    self.in_user_written_closure
                                                        && self.user_closure_params.contains(name)
                                                } else {
                                                    false
                                                };

                                            let mut string_literal_converted_here = false;

                                            // PHASE 2: String literals need conversion for &String parameters (but not &str!)
                                            if is_string_literal {
                                                // Check if parameter is explicitly &str
                                                let param_is_str_ref = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                                    matches!(t, Type::Reference(inner) if matches!(**inner, Type::Custom(ref name) if name == "str"))
                                                });

                                                if param_is_str_ref {
                                                    // Parameter is &str - pass literal directly (already a &str)
                                                    // No conversion needed!
                                                } else {
                                                    // Parameter is Type::String (becomes &String in Rust)
                                                    let param_is_string = sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                                        matches!(t, Type::String) || matches!(t, Type::Custom(ref name) if name == "string")
                                                    });
                                                    if param_is_string {
                                                        // Parameter is &String - need conversion
                                                        arg_str = format!("&{}.to_string()", arg_str);
                                                        string_literal_converted_here = true;
                                                    }
                                                }
                                            } else if !is_user_closure_param {
                                                let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                                    arg_to_generate,
                                                    &arg_str,
                                                    call_method.as_str(),
                                                    i,
                                                    &method_signature,
                                                    &self.usize_variables,
                                                    &self.current_function_params,
                                                    &self.borrowed_iterator_vars,
                                                    &self.inferred_borrowed_params,
                                                    arguments.len(),
                                                    type_name.as_deref(),
                                                    Some(&self.local_var_types),
                                                    Some(&self.stdlib_method_signatures),
                                                    Some(&self.method_signatures_by_type),
                                                    &self.match_arm_bindings, // TDD FIX: E0308 fix
                                                );
                                                if should_ref {
                                                    arg_str = format!("&{}", arg_str);
                                                }
                                            }

                                            arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                                                &method_signature,
                                                sig_param_idx,
                                                arg_to_generate,
                                                arg_str,
                                                string_literal_converted_here,
                                            );
                                        }
                                        OwnershipMode::MutBorrowed => {
                                            let is_already_mut_ref =
                                                if let Expression::Identifier { name, .. } = arg_to_generate {
                                                    let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                                        param.name == *name
                                                            && matches!(&param.type_, crate::parser::Type::MutableReference(_))
                                                    });
                                                    let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                                    explicit_mut_ref || inferred_mut_ref
                                                } else {
                                                    false
                                                };
                                            if !expression_helpers::is_reference_expression(arg_to_generate)
                                                && !is_already_mut_ref
                                            {
                                                let mut mut_arg_str = if arg_str.ends_with(".clone()") {
                                                    arg_str[..arg_str.len() - 8].to_string()
                                                } else {
                                                    arg_str
                                                };
                                                if mut_arg_str.starts_with("&") && !mut_arg_str.starts_with("&mut ") {
                                                    mut_arg_str = mut_arg_str[1..].to_string();
                                                }
                                                arg_str = format!("&mut {}", mut_arg_str);
                                            }
                                        }
                                        OwnershipMode::Owned => {
                                            // String literal coercion: "foo" → "foo".to_string()
                                            // when param expects owned String
                                            let is_str_lit = matches!(
                                                arg_to_generate,
                                                Expression::Literal { value: Literal::String(_), .. }
                                            );
                                            // Also handle &str parameters being passed to methods expecting String
                                            let is_str_param = matches!(
                                                arg_to_generate,
                                                Expression::Identifier { name, .. }
                                                    if self.current_function_params.iter().any(|p| {
                                                        &p.name == name && matches!(
                                                            &p.type_,
                                                            Type::Reference(inner) if matches!(**inner, Type::Custom(ref s) if s == "str")
                                                        )
                                                    })
                                            );
                                            if is_str_lit || is_str_param {
                                                let is_explicit_str_ref = sig.param_types.get(sig_param_idx)
                                                    .is_some_and(|t| matches!(t, Type::Reference(inner) if
                                                        matches!(**inner, Type::String) ||
                                                        matches!(**inner, Type::Custom(ref s) if s == "str")
                                                    ));
                                                if !is_explicit_str_ref {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                }
                                            }
                                            // Destination wants owned - add .clone() for borrowed sources
                                            if let Expression::FieldAccess {
                                                object: field_obj,
                                                ..
                                            } = arg_to_generate
                                            {
                                                if let Expression::Identifier { name, .. } =
                                                    &**field_obj
                                                {
                                                    let is_borrowed =
                                                        self.borrowed_iterator_vars.contains(name)
                                                            || self
                                                                .inferred_borrowed_params
                                                                .contains(name);
                                                    if is_borrowed && !arg_str.ends_with(".clone()")
                                                    {
                                                        let is_copy = self
                                                            .infer_expression_type(arg_to_generate)
                                                            .as_ref()
                                                            .is_some_and(|t| self.is_type_copy(t));
                                                        if !is_copy {
                                                            arg_str =
                                                                format!("{}.clone()", arg_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // AUTO-CAST int → float: Call(FieldAccess) path
                                // Skip when signature has a collision (different types with same name).
                                let qualified_key = type_name.as_ref()
                                    .map(|tn| format!("{}::{}", tn, call_method));
                                let has_collision = qualified_key.as_ref()
                                    .is_some_and(|k| self.signature_registry.has_collision(k))
                                    || self.signature_registry.has_collision(&call_method);
                                if !has_collision {
                                    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                                        let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                        let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                        if param_is_f32 || param_is_f64 {
                                            let arg_ty = self.infer_expression_type(arg);
                                            let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                                matches!(t, Type::Int)
                                                    || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                                            });
                                            if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                                let target = if param_is_f32 { "f32" } else { "f64" };
                                                arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                    format!("({}) as {}", arg_str, target)
                                                } else {
                                                    format!("{} as {}", arg_str, target)
                                                };
                                            }
                                        }
                                    }
                                }

                                vec![arg_str]
                            })
                            .collect()
                    } else {
                        // No signature: still apply map-key strip + stdlib `should_add_ref` (parser uses Call+FieldAccess)
                        // Try to find signature by qualified or simple method name for string coercion.
                        // CRITICAL: For common stdlib methods (get, remove, contains, etc.),
                        // do NOT fall back to unqualified lookup — it can match the WRONG
                        // user-defined method (e.g., ComponentArray::get when we want
                        // HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                        // This mirrors the guard in the MethodCall handler.
                        let fallback_sig = type_name
                            .as_ref()
                            .map(|tn| format!("{}::{}", tn, call_method))
                            .and_then(|q| self.signature_registry.get_signature(&q).cloned())
                            .or_else(|| {
                                if super::stdlib_method_traits::is_common_stdlib_method(&call_method)
                                {
                                    None
                                } else {
                                    self.signature_registry.get_signature(&call_method).cloned()
                                }
                            });
                        arguments
                            .iter()
                            .enumerate()
                            .map(|(i, (_label, arg))| {
                                let arg_to_generate =
                                    expression_utilities::strip_unary_ref_for_collection_key_arg(&call_method, i, arg);
                                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                                self.coerce_string_literals_to_owned = false;
                                let prev_match_arm_str = self.in_match_arm_needing_string;
                                self.in_match_arm_needing_string = false;
                                let mut arg_str = self.generate_expression(arg_to_generate);
                                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                                self.in_match_arm_needing_string = prev_match_arm_str;

                                // Check if this argument needs .to_string() conversion
                                // This handles both string literals AND &str parameters
                                let is_string_literal = matches!(
                                    arg_to_generate,
                                    Expression::Literal { value: Literal::String(_), .. }
                                );
                                let is_str_param = matches!(
                                    arg_to_generate,
                                    Expression::Identifier { name, .. }
                                        if self.inferred_borrowed_params.contains(name)
                                            || self.current_function_params.iter().any(|p| {
                                                &p.name == name && matches!(
                                                    &p.type_,
                                                    Type::Reference(inner) if matches!(**inner, Type::Custom(ref s) if s == "str")
                                                )
                                            })
                                );
                                if is_string_literal || is_str_param {
                                    let needs_to_string = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                        i,
                                        call_method.as_str(),
                                        &fallback_sig,
                                    );
                                    if needs_to_string {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    }
                                }

                                let should_ref =
                                    crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                        arg_to_generate,
                                        &arg_str,
                                        call_method.as_str(),
                                        i,
                                        &fallback_sig,
                                        &self.usize_variables,
                                        &self.current_function_params,
                                        &self.borrowed_iterator_vars,
                                        &self.inferred_borrowed_params,
                                        arguments.len(),
                                        type_name.as_deref(),
                                        Some(&self.local_var_types),
                                        Some(&self.stdlib_method_signatures),
                                        Some(&self.method_signatures_by_type),
                                        &self.match_arm_bindings, // TDD FIX: E0308 fix
                                    );
                                if should_ref {
                                    arg_str = format!("&{}", arg_str);
                                }

                                let string_literal_converted_here = (is_string_literal || is_str_param)
                                    && arg_str.ends_with(".to_string()");
                                if let Some(fb_idx) = fallback_sig.as_ref().map(|s| {
                                    if s.has_self_receiver {
                                        i + 1
                                    } else {
                                        i
                                    }
                                }) {
                                    arg_str =
                                        self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                                            &fallback_sig,
                                            fb_idx,
                                            arg_to_generate,
                                            arg_str,
                                            string_literal_converted_here,
                                        );
                                }
                                arg_str
                            })
                            .collect()
                    };

                    let call_str = format!("{}.{}({})", obj_str, call_method, args.join(", "));

                    let is_extern_call = method_signature.as_ref().is_some_and(|sig| sig.is_extern)
                        || self
                            .signature_registry
                            .get_signature(call_method.as_str())
                            .is_some_and(|sig| sig.is_extern)
                        || self.extern_function_names.contains(call_method.as_str());

                    return if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {} }})", call_str)
                    } else {
                        call_str
                    };
                }

                let mut func_str = self.generate_expression(function);

                // Windjammer stdlib type mapping: Map::method → HashMap::method
                if func_str.starts_with("Map::") {
                    func_str = func_str.replacen("Map::", "HashMap::", 1);
                }

                // E0282 turbofish: Vec::new() / HashSet::new() → Vec::<T>::new() / HashSet::<T>::new()
                // when the function return type provides the element type.
                // Skip when suppress_collection_turbofish is set (let binding already has type ascription).
                if arguments.is_empty() && !self.suppress_collection_turbofish {
                    if func_str == "Vec::new" {
                        if let Some(Type::Vec(inner)) = &self.current_function_return_type {
                            func_str = format!("Vec::<{}>::new", self.type_to_rust(inner));
                        }
                    } else if func_str == "HashSet::new" {
                        if let Some(Type::Parameterized(base, args)) =
                            &self.current_function_return_type
                        {
                            if base == "HashSet" && args.len() == 1 {
                                func_str =
                                    format!("HashSet::<{}>::new", self.type_to_rust(&args[0]));
                            }
                        }
                    } else if func_str == "HashMap::new" {
                        if let Some(Type::Parameterized(base, args)) =
                            &self.current_function_return_type
                        {
                            if base == "HashMap" && args.len() == 2 {
                                func_str = format!(
                                    "HashMap::<{}, {}>::new",
                                    self.type_to_rust(&args[0]),
                                    self.type_to_rust(&args[1])
                                );
                            }
                        }
                    }
                }

                // In an impl block, bare function calls to sibling methods need qualified dispatch.
                // Instance methods (take self) → self.method(args)
                // Static methods → Self::method(args)
                if self.in_impl_block
                    && !func_name.contains("::")
                    && self.current_impl_methods.contains(&func_name)
                {
                    if self.current_impl_instance_methods.contains(&func_name) {
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
                    if let Some(Type::Option(inner)) = &self.current_function_return_type {
                        let inner_rust = self.type_to_rust(inner);
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
                let is_std_enum = matches!(func_name.as_str(), "Some" | "Ok" | "Err");
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
                    && self.tuple_struct_names.contains(&func_name);

                if is_std_enum || is_custom_enum || is_tuple_struct_constructor {
                    // Enum variant constructors need owned values (Some(T), Ok(T), Err(E)).
                    // Set owned context so index expressions use .clone() instead of &,
                    // BUT only for arguments that aren't already explicit references.
                    let prev_owned_context = self.in_owned_value_context;
                    let generated_args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| {
                            let is_explicit_ref = matches!(
                                arg,
                                Expression::Unary {
                                    op: crate::parser::UnaryOp::Ref
                                        | crate::parser::UnaryOp::MutRef,
                                    ..
                                }
                            );
                            if !is_explicit_ref {
                                self.in_owned_value_context = true;
                            }
                            let result = self.generate_expression(arg);
                            self.in_owned_value_context = prev_owned_context;
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
                                if arg_str.starts_with("format!(")
                                    || arg_str.starts_with("&format!(")
                                {
                                    // Strip leading & if present
                                    let format_expr = if arg_str.starts_with("&") {
                                        arg_str.strip_prefix("&").unwrap()
                                    } else {
                                        arg_str
                                    };
                                    // Extract to temp var
                                    let temp_name = format!("_temp{}", temp_counter);
                                    temp_counter += 1;
                                    temp_decls.push_str(&format!(
                                        "let {} = {}; ",
                                        temp_name, format_expr
                                    ));

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
                                let returns_option_ref = match &self.current_function_return_type {
                                    Some(Type::Option(inner_type)) => {
                                        matches!(
                                            **inner_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                    }
                                    _ => false,
                                };

                                // Check if return type is Result<&T, E> or Result<&mut T, E>
                                let returns_result_ref = match &self.current_function_return_type {
                                    Some(Type::Result(ok_type, _err_type)) => {
                                        matches!(
                                            **ok_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
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
                                    if self.str_ref_optimized_params.contains(name.as_str()) {
                                        format!("{}.to_string()", result)
                                    } else if self.borrowed_iterator_vars.contains(name)
                                        || self.inferred_borrowed_params.contains(name.as_str())
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
                let mut signature = if let Some(param) = self
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
                                    Type::Custom(name) if name == "string" => {
                                        OwnershipMode::Borrowed
                                    }
                                    // Explicit references - borrowed (types.rs:154)
                                    Type::Reference(_) | Type::MutableReference(_) => {
                                        OwnershipMode::Borrowed
                                    }
                                    // Copy types - owned (types.rs:156-157)
                                    Type::Int
                                    | Type::Int32
                                    | Type::Uint
                                    | Type::Float
                                    | Type::Bool => OwnershipMode::Owned,
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
                            name: func_name.clone(),
                            param_types: params.clone(),
                            param_ownership,
                            return_type: return_type.as_ref().map(|t| (**t).clone()),
                            return_ownership: OwnershipMode::Owned, // Functions return owned by default
                            has_self_receiver: false,
                            is_extern: false,
                        })
                    } else {
                        // Not a function pointer - try registry
                        self.signature_registry.get_signature(&func_name).cloned()
                    }
                } else {
                    // Not a parameter - try registry lookup
                    let direct = self.signature_registry.get_signature(&func_name).cloned();
                    direct.or_else(|| {
                        if let Some(pos) = func_name.rfind("::") {
                            let qualifier = &func_name[..pos];
                            let simple_name = &func_name[pos + 2..];
                            let is_type_qualifier =
                                qualifier.chars().next().is_some_and(|c| c.is_uppercase());
                            if is_type_qualifier {
                                self.signature_registry.get_signature(simple_name).cloned()
                            } else {
                                // For module-qualified calls (e.g., draw::draw_text),
                                // try progressively shorter qualified names.
                                // Do NOT fall back to simple name - it may collide
                                // with a different module's function with the same name.
                                let parts: Vec<&str> = func_name.split("::").collect();
                                let mut found = None;
                                for start in (0..parts.len().saturating_sub(1)).rev() {
                                    let candidate = parts[start..].join("::");
                                    if let Some(sig) =
                                        self.signature_registry.get_signature(&candidate)
                                    {
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
                    let simple = func_name.rsplit("::").next().unwrap_or(&func_name);

                    // Try resolving through module alias map first
                    if let Some(original_module) = self.module_alias_map.get(qualifier) {
                        let resolved_name = format!("{}::{}", original_module, simple);
                        if let Some(resolved_sig) =
                            self.signature_registry.get_signature(&resolved_name)
                        {
                            signature = Some(resolved_sig.clone());
                        }
                    }

                    // If alias resolution didn't work, try simple-name fallback
                    // with arg count validation to avoid name collisions.
                    if signature.is_none() {
                        if let Some(found) = self.signature_registry
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
                    let simple = func_name.rsplit("::").next().unwrap_or(&func_name);
                    self.extern_function_names.contains(simple)
                };

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (_label, arg))| {
                        // CRITICAL: Reset in_field_access_object for argument generation.
                        // Arguments are independent expressions, NOT part of a field/method/index chain.
                        // Without this, `process_property(prop.name, prop.value).as_str()` would
                        // leak in_field_access_object from the MethodCall handler into prop.name/prop.value,
                        // suppressing necessary .clone() calls.
                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;

                        // TDD FIX: Set call argument context to suppress premature .clone()
                        // The FieldAccess handler normally adds .clone() for borrowed iterator vars,
                        // but in call arguments, we need to let the ownership check below decide
                        let prev_in_call_arg = self.in_call_argument_generation;
                        self.in_call_argument_generation = true;

                        // Return/match contexts set `coerce_string_literals_to_owned` and
                        // `in_match_arm_needing_string` for the outer expression; nested call
                        // arguments must use only parameter-type conversion (below), not context
                        // coercion — avoids `"x".to_string().to_string()` and wrong `.to_string()`
                        // on &str params, and prevents format!("...".to_string(), ...) in match arms.
                        let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                        self.coerce_string_literals_to_owned = false;
                        let prev_match_arm_str = self.in_match_arm_needing_string;
                        self.in_match_arm_needing_string = false;
                        let mut arg_str = self.generate_expression(arg);
                        self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                        self.in_match_arm_needing_string = prev_match_arm_str;

                        self.in_call_argument_generation = prev_in_call_arg;
                        self.in_field_access_object = prev_field_access_obj;

                        // TDD FIX: Cast int arguments to usize for stdlib methods
                        // Vec::with_capacity(size) where size: int → Vec::with_capacity(size as usize)
                        // Vec::with_capacity(10) where 10: int literal → Vec::with_capacity(10_usize)
                        if i == 0 && (func_name == "Vec::with_capacity" || func_name == "HashMap::with_capacity" ||
                                      func_name == "String::with_capacity" || func_name == "Vec::reserve") {
                            match arg {
                                Expression::Identifier { .. } => {
                                    // Variables: add explicit cast
                                    arg_str = format!("{} as usize", arg_str);
                                }
                                Expression::Literal { value: Literal::Int(val), .. } => {
                                    // Literals: use usize suffix
                                    arg_str = format!("{}_usize", val);
                                }
                                _ => {
                                    // Other expressions (e.g., calculations): wrap in (expr) as usize
                                    if !arg_str.ends_with("_usize") && !arg_str.contains(" as usize") {
                                        arg_str = format!("({}) as usize", arg_str);
                                    }
                                }
                            }
                        }

                        // WINDJAMMER FFI: Convert string arguments for extern functions
                        if is_extern_call {
                            if let Some(ref sig) = signature {
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, Type::Custom(name) if name == "str") {
                                        // Expand str to (ptr, len)
                                        return vec![
                                            format!("{}.as_bytes().as_ptr()", arg_str),
                                            format!("{}.as_bytes().len()", arg_str),
                                        ];
                                    }
                                    // string/String params → FfiString via string_to_ffi
                                    // TDD FIX: Always use .to_string() - infer_expression_type returns
                                    // declared param type (Type::String), not actual Rust type. When
                                    // ownership infers Borrowed, param becomes &str in Rust, but we
                                    // thought it was String and passed directly → E0308.
                                    // .to_string() works for both &str and String (String::to_string = clone).
                                    //
                                    // TDD FIX: Strip redundant .to_string() before wrapping.
                                    // Bug: User writes render_text(label.to_string(), x, y). Expression
                                    // generation produces "label.to_string()", then we added another
                                    // → string_to_ffi(label.to_string().to_string()). Fix: If arg_str
                                    // already ends with .to_string(), don't add another.
                                    if matches!(param_type, Type::String)
                                        || matches!(param_type, Type::Custom(n) if n == "string" || n == "String")
                                    {
                                        let inner = if arg_str.ends_with(".to_string()") {
                                            arg_str.clone()
                                        } else {
                                            format!("{}.to_string()", arg_str)
                                        };
                                        return vec![format!(
                                            "windjammer_runtime::ffi::string_to_ffi({})",
                                            inner
                                        )];
                                    }
                                }
                            }
                        }

                        // Auto-convert string literals to String for functions expecting owned String
                        // THE WINDJAMMER WAY: Smart inference based on available information!
                        if matches!(
                            arg,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            // Check if the parameter expects an owned String
                            let should_convert = if let Some(ref sig) = signature {
                                if sig.is_extern {
                                    // Extern functions have explicit types; ownership inference
                                    // is meaningless (empty body defaults to Borrowed).
                                    // Convert if parameter type is String.
                                    sig.param_types.get(i).is_some_and(|ty| {
                                        matches!(ty, Type::String)
                                            || matches!(ty, Type::Custom(name) if name == "string" || name == "String")
                                    })
                                } else if sig.param_types.get(i).is_some_and(|ty| {
                                    matches!(ty, Type::Reference(inner) if
                                        matches!(**inner, Type::Custom(ref s) if s == "str"))
                                }) {
                                    // Parameter type is &str (string optimization inferred this).
                                    // String literals are already &str in Rust — no .to_string() needed.
                                    false
                                } else if signature_from_simple_fallback && {
                                    let qualifier = func_name.split("::").next().unwrap_or("");
                                    qualifier.chars().next().is_some_and(|c| c.is_lowercase())
                                } {
                                    // Fallback-resolved from module::function: the signature may
                                    // be from a different module. Don't trust ownership for
                                    // string coercion — the actual target may take &str.
                                    false
                                } else if let Some(&ownership) = sig.param_ownership.get(i) {
                                    // Convert if parameter expects owned String
                                    matches!(ownership, OwnershipMode::Owned)
                                } else {
                                    // No ownership info for this param
                                    // THE WINDJAMMER WAY: Heuristic for constructors
                                    // Functions named 'new' (or Type::new) taking string params likely expect String
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            } else {
                                // No signature found — check enum variant registry
                                // WINDJAMMER FIX: Enum variant constructors like GameEvent::ItemPickup("text")
                                // need .to_string() when the variant field is String type
                                if let Some(variant_types) = self.enum_variant_types.get(&func_name) {
                                    // TDD FIX: Check for both Type::String and Type::Custom("String")
                                    variant_types.get(i).is_some_and(|ty| {
                                        matches!(ty, Type::String)
                                            || matches!(ty, Type::Custom(name) if name == "String")
                                    })
                                } else {
                                    // Fallback heuristic for constructors
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            };

                            if should_convert {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }

                        // Check if this parameter expects a borrow
                        // Skip ownership inference for extern function calls - they have explicit types
                        if let Some(ref sig) = signature {
                            if sig.is_extern {
                                // Auto-convert mut locals to &mut when FFI param is *mut T
                                // This eliminates Rust leakage: users write `ffi_fn(x)` not `ffi_fn(&mut x)`
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, crate::parser::ast::types::Type::RawPointer { mutable: true, .. }) {
                                        return vec![format!("&mut {}", arg_str)];
                                    }
                                }
                                return vec![arg_str];
                            }

                            // COLLISION GUARD: When the signature was resolved via a
                            // simple-name fallback from a module-qualified call AND the
                            // simple name has a collision, skip auto-borrow/auto-mutborrow.
                            // The looked-up signature may be from the wrong module,
                            // so applying its ownership blindly can produce incorrect
                            // `&` or `&mut` prefixes.
                            //
                            // We only guard fallback-resolved signatures because:
                            // - Direct qualified lookups are unambiguous (right signature)
                            // - Bare-name calls within the same file are also unambiguous
                            // - Only fallback from module::fn → fn is risky (wrong module)
                            let simple_name = func_name.rsplit("::").next().unwrap_or(&func_name);
                            let has_ownership_collision = signature_from_simple_fallback
                                && (self.signature_registry.has_collision(&func_name)
                                    || self.signature_registry.has_collision(simple_name))
                                && {
                                    // Validate collision: if the found signature's arg count
                                    // matches the actual call, it's the right overload despite
                                    // the collision. Only suppress ownership when arg count
                                    // doesn't match (genuinely ambiguous signature).
                                    let sig_args = if sig.has_self_receiver {
                                        sig.param_ownership.len().saturating_sub(1)
                                    } else {
                                        sig.param_ownership.len()
                                    };
                                    sig_args != arguments.len()
                                };

                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed if !has_ownership_collision => {
                                        // PHASE 1: Generate &String parameters for correctness
                                        // String literals need conversion: "foo" → &"foo".to_string()
                                        let is_string_literal = matches!(
                                            arg,
                                            Expression::Literal {
                                                value: Literal::String(_),
                                                ..
                                            }
                                        );

                                        if is_string_literal {
                                            // PHASE 2 CALL-SITE OPTIMIZATION: Check if parameter is &String vs &str
                                            // In the AST, `string` parameters are Type::String (converted to &String by codegen)
                                            // Explicit `&str` parameters are Type::Reference(Custom("str"))
                                            let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                                matches!(t, Type::Reference(inner) if matches!(**inner, Type::Custom(ref name) if name == "str"))
                                            });

                                            if param_is_str_ref {
                                                // Parameter is explicitly &str - pass literal directly (already a &str)
                                                // "World" is already &str in Rust, no conversion needed!
                                                return vec![arg_str];
                                            } else {
                                                // Parameter is Type::String (becomes &String in Rust)
                                                // Check if it's actually a String type
                                                let param_is_string = sig.param_types.get(i).is_some_and(|t| {
                                                    matches!(t, Type::String) || matches!(t, Type::Custom(ref name) if name == "string")
                                                });
                                                if param_is_string {
                                                    // Convert &str literal to &String: "World" → &"World".to_string()
                                                    return vec![format!("&{}.to_string()", arg_str)];
                                                } else {
                                                    // Non-string type - pass directly
                                                    return vec![arg_str];
                                                }
                                            }
                                        }

                                        // TDD FIX: Check if parameter is already a reference type
                                        // If param is &string, don't add another & (would be &&string)
                                        let is_param_already_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::Reference(_)
                                                                | Type::MutableReference(_)
                                                        )
                                                })
                                            } else {
                                                false
                                            };

                                        // TDD FIX: Don't add & for Copy type parameters
                                        // When signature says Borrowed but param type is Copy,
                                        // codegen keeps it as owned (e.g., x: usize not x: &usize)
                                        // So the call site should NOT add &
                                        // BUT: Reference types (&Vec<T>, &[T]) are NOT treated as
                                        // Copy here - if param type is &T, caller still needs &
                                        let is_copy_param = sig.param_types.get(i)
                                            .map(|t| {
                                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                                    && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
                                            })
                                            .unwrap_or(false);

                                        // TDD FIX (Bug #16): Don't add & to temp variables!
                                        // Temp variables (like _temp0) hold OWNED values from format!()
                                        // format!() returns String, not &str, so _temp0 is String
                                        // If we add &, we get &String when we need String
                                        let is_temp_variable = arg_str.starts_with("_temp")
                                            && arg_str.chars().skip(5).all(|c| c.is_numeric());

                                        // TDD FIX: IDIOMATIC WINDJAMMER - Strip .clone() if present!
                                        // When destination wants Borrowed, pass &field, NOT &field.clone()
                                        // Example: has_item(ingredient.item_id) with has_item(item_id: string)
                                        // Should generate: has_item(&ingredient.item_id)
                                        // NOT: has_item(&ingredient.item_id.clone())
                                        // The .clone() may have been added by generate_expression for borrowed iterator vars
                                        if arg_str.ends_with(".clone()") {
                                            arg_str = arg_str[..arg_str.len() - 8].to_string();
                                        }

                                        // Insert & if not already a reference and not a string literal and not a temp var
                                        // THE WINDJAMMER WAY: Preserve user-written closure params
                                        let is_user_closure_param = if let Expression::Identifier { name, .. } = arg {
                                            self.in_user_written_closure && self.user_closure_params.contains(name)
                                        } else {
                                            false
                                        };

                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_param_already_ref
                                            && !is_copy_param
                                            && !is_temp_variable
                                            && !is_user_closure_param
                                        {
                                            return vec![format!("&{}", arg_str)];
                                        } else {
                                            return vec![arg_str];
                                        }
                                    }
                                    OwnershipMode::MutBorrowed if !has_ownership_collision => {
                                        // TDD FIX: Don't add &mut if arg is already a &mut parameter
                                        // Covers both explicitly declared &mut params AND
                                        // params inferred as &mut through ownership analysis
                                        let is_already_mut_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                // Check 1: Explicit &mut in AST type
                                                let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::MutableReference(_)
                                                        )
                                                });
                                                // Check 2: Inferred &mut through ownership analysis
                                                let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                                explicit_mut_ref || inferred_mut_ref
                                            } else {
                                                false
                                            };

                                        // Insert &mut if not already a reference
                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_already_mut_ref
                                        {
                                            // CRITICAL FIX: Remove .clone() if present - we want to mutate the original!
                                            // &mut counter.clone() → &mut counter
                                            // When passing &mut, we're giving mutable access to the original,
                                            // not a clone. The .clone() would break mutation semantics.
                                            let mut_arg_str = if arg_str.ends_with(".clone()") {
                                                arg_str[..arg_str.len() - 8].to_string()
                                            } else {
                                                arg_str
                                            };
                                            return vec![format!("&mut {}", mut_arg_str)];
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // String optimization override: param_types may say &str
                                        // while param_ownership is stale as Owned. Trust param_types.
                                        let param_is_str_ref = sig.param_types.get(i).is_some_and(|t| {
                                            matches!(t, Type::Reference(inner) if
                                                matches!(**inner, Type::Custom(ref s) if s == "str"))
                                        });
                                        if param_is_str_ref {
                                            return vec![arg_str];
                                        }

                                        if let Expression::Identifier { name, .. } = arg {
                                            // Find the parameter type
                                            let param_type = self
                                                .current_function_params
                                                .iter()
                                                .find(|p| &p.name == name)
                                                .map(|p| &p.type_);

                                            // Check if it's a reference parameter (&str, &String, &T, &mut T)
                                            let inner_from_ref = match param_type {
                                                Some(Type::Reference(inner)) => Some(inner.as_ref()),
                                                Some(Type::MutableReference(inner)) => Some(inner.as_ref()),
                                                _ => None,
                                            };
                                            if let Some(inner_type) = inner_from_ref {
                                                if matches!(inner_type, Type::String)
                                                    && !arg_str.ends_with(".to_string()")
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                } else if self.is_type_copy(inner_type)
                                                    && !arg_str.trim_start().starts_with('*')
                                                {
                                                    arg_str = format!("*{}", arg_str);
                                                } else if !arg_str.ends_with(".clone()")
                                                    && !arg_str.trim_start().starts_with('*')
                                                {
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            } else {
                                                // TDD FIX: Check if it's from a borrowed iterator (for loop)
                                                // Example: for npc_id in npc_ids { Member::new(npc_id) }
                                                // npc_id is &String from iterator, needs .clone() for owned String
                                                //
                                                // CRITICAL: We're in OwnershipMode::Owned block, which means
                                                // the DESTINATION parameter wants an owned value (String, not &String).
                                                //
                                                // Windjammer `string` parameters lower to `&str`: `.clone()` keeps
                                                // `&str` (E0308). Use `.to_string()` for text types instead.
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(name);

                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(name);

                                                let is_inferred_mut_borrowed =
                                                    self.inferred_mut_borrowed_params.contains(name);

                                                if (is_borrowed_iterator_var
                                                    || is_inferred_borrowed
                                                    || is_inferred_mut_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    // `*ident` = owned Copy from &/&mut (see Identifier
                                                    // in_owned_value_context); do not append .clone().
                                                    if !arg_str.trim_start().starts_with('*') {
                                                        let is_text = self
                                                            .infer_expression_type(arg)
                                                            .as_ref()
                                                            .is_some_and(|t| {
                                                            crate::codegen::rust::types::is_windjammer_text_type(t)
                                                        });
                                                        let is_phase2_str_param = self
                                                            .str_ref_optimized_params
                                                            .contains(name.as_str());
                                                        if is_text && !is_phase2_str_param {
                                                            arg_str =
                                                                format!("{}.to_string()", arg_str);
                                                        } else if !is_text {
                                                            // Borrowed from iterator or inferred - use .clone()
                                                            // This handles &T → T for non-text types
                                                            arg_str = format!("{}.clone()", arg_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // TDD FIX: AUTO-CLONE for borrowed_param.field
                                        // When passing ingredient.item_id where ingredient is borrowed,
                                        // we need to clone() IF destination wants Owned.
                                        //
                                        // We're ALREADY in OwnershipMode::Owned block,
                                        // so destination wants owned. Safe to add .clone().
                                        //
                                        // This handles: for ingredient in &vec { func(ingredient.field) }
                                        // where func(field: String) expects owned.
                                        if let Expression::FieldAccess { .. } = arg {
                                            // Trace through nested field accesses to find the root identifier
                                            // Handles: stack.field, stack.item.id, stack.item.nested.deep
                                            let root_name = self.extract_root_identifier(arg);
                                            if let Some(name) = root_name {
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(&name);
                                                let is_explicitly_borrowed =
                                                    self.current_function_params.iter().any(|p| {
                                                        p.name == name
                                                            && matches!(
                                                                p.ownership,
                                                                crate::parser::OwnershipHint::Ref
                                                            )
                                                    });
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(&name);

                                                if (is_borrowed_iterator_var
                                                    || is_explicitly_borrowed
                                                    || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    let is_copy = self.infer_expression_type(arg)
                                                        .as_ref()
                                                        .is_some_and(|t| self.is_type_copy(t));
                                                    if !is_copy {
                                                        arg_str = format!("{}.clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }
                                        // DOGFOODING FIX: Vec indexing &vec[idx] passed to owned param
                                        // e.g. enterable.push(self.buildings[i]) → need (.clone())
                                        if let Expression::Index { .. } = arg {
                                            if arg_str.starts_with("&")
                                                && !arg_str.ends_with(".clone()")
                                            {
                                                if let Some(inner) = self.infer_expression_type(arg)
                                                {
                                                    if !self.is_type_copy(&inner) {
                                                        arg_str =
                                                            format!("({}).clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // Collision guard triggered: Borrowed or MutBorrowed
                                        // with a signature collision. Don't apply auto-borrow;
                                        // pass the argument as-is and let downstream Rust
                                        // compilation determine the correct behavior.
                                    }
                                }
                            }
                        } else {
                            // No signature found - don't auto-clone!
                            // Without signature info, we can't know if destination wants Owned or Borrowed
                            // Better to let Rust compiler catch the error than guess wrong
                        }

                        // AUTO-CAST int → float: regular Call path
                        // Skip when the signature key has a collision (different types registered
                        // the same function name with different param types). The auto-cast
                        // cannot be trusted when the looked-up signature may be from a different
                        // type in another module.
                        if let Some(ref sig) = signature {
                            let has_collision = self.signature_registry.has_collision(&func_name)
                                || self.signature_registry.has_collision(&func_str);
                            if !has_collision {
                                if let Some(param_ty) = sig.param_types.get(i) {
                                    let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                    let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                    if param_is_f32 || param_is_f64 {
                                        let arg_ty = self.infer_expression_type(arg);
                                        let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                            matches!(t, Type::Int)
                                                || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                                        });
                                        if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                            let target = if param_is_f32 { "f32" } else { "f64" };
                                            arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                format!("({}) as {}", arg_str, target)
                                            } else {
                                                format!("{} as {}", arg_str, target)
                                            };
                                        }
                                    }
                                }
                            }
                        }

                        vec![arg_str]
                    })
                    .collect();

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
                                temp_decls
                                    .push_str(&format!("let {} = {}; ", temp_name, format_expr));

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
                    if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {}{}  }})", temp_decls, call_expr)
                    } else {
                        format!("{{ {}{} }}", temp_decls, call_expr)
                    }
                } else {
                    // No format!() args - generate normally with optional unsafe wrapper
                    let call_str = format!("{}({})", func_str, args.join(", "));
                    if is_extern_call && !self.in_unsafe_block {
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
}

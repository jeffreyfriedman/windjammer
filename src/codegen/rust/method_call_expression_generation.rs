//! Method call expression generation
//!
//! Handles code generation for method calls including:
//! - Regular method calls on objects
//! - Auto-borrow and auto-deref insertion
//! - String literal → String conversion
//! - Redundant .as_str() elimination
//! - Method signature-based ownership inference

use crate::analyzer::OwnershipMode;
use crate::parser::*;

use super::{ast_utilities, expression_helpers, expression_utilities, string_analysis, type_classification_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a method call expression
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn generate_method_call_expression(
        &mut self,
        object: &Expression<'ast>,
        method: &str,
        type_args: &Option<Vec<Type>>,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
    ) -> String {
        // TDD FIX: Strip redundant .as_str() on &str parameters
        // If method is .as_str() and object is already inferred as &str, just return object
        if method == "as_str" && arguments.is_empty() {
                    if let Expression::Identifier { name, .. } = object {
                        let is_borrowed = self.inferred_borrowed_params.contains(name.as_str());
                        if is_borrowed {
                            // Parameter is already &str, .as_str() is redundant
                            return self.generate_expression(object);
                        }
                    }
                }

                // METHOD CALL CONTEXT: Suppress Vec index auto-clone when generating the
                // object of a method call. Methods take &self or &mut self, so Rust allows
                // calling methods on &T returned by Vec indexing without cloning.
                // e.g., self.lights[i].is_enabled() → no need to clone the whole Light2D
                let prev_field_access = self.in_field_access_object;
                self.in_field_access_object = true;
                // DOUBLE-CLONE FIX: When the source has explicit .clone(), suppress auto-clone
                // on the object to prevent .clone().clone(). The explicit clone IS the clone.
                let prev_explicit_clone = self.in_explicit_clone_call;
                if method == "clone" {
                    self.in_explicit_clone_call = true;
                }
                let mut obj_str = self.generate_expression_with_precedence(object);
                self.in_field_access_object = prev_field_access;
                self.in_explicit_clone_call = prev_explicit_clone;
                // E0507: `collection[i].method(args)` when the method consumes `self` (owned receiver)
                // must clone the element: `self.tracks[i].clone().sample(t)` (otherwise move out of &Vec).
                if matches!(object, Expression::Index { .. }) {
                    if let Some(recv_ty) = self.infer_expression_type(object) {
                        if !self.is_type_copy(&recv_ty) {
                            if let Some(tn) = Self::type_to_name(&recv_ty) {
                                let qualified = format!("{}::{}", tn, method);
                                let sig_opt = self
                                    .signature_registry
                                    .get_signature(&qualified)
                                    .or_else(|| self.signature_registry.get_signature(method));
                                if let Some(sig) = sig_opt {
                                    if sig.has_self_receiver
                                        && sig.param_ownership.first()
                                            == Some(&crate::analyzer::OwnershipMode::Owned)
                                        && !obj_str.ends_with(".clone()")
                                    {
                                        obj_str = format!("{}.clone()", obj_str);
                                    }
                                }
                            }
                        }
                    }
                }

                // E0507: `borrowed_var.method(args)` when the method consumes `self` (owned receiver)
                // and the variable is a borrowed iterator variable (from `for x in &collection`).
                // Must clone: `condition.clone().evaluate(state)` instead of `condition.evaluate(state)`.
                if let Expression::Identifier { name, .. } = object {
                    if self.borrowed_iterator_vars.contains(name) && method != "clone" {
                        if let Some(recv_ty) = self.infer_expression_type(object) {
                            if !self.is_type_copy(&recv_ty) {
                                if let Some(tn) = Self::type_to_name(&recv_ty) {
                                    let qualified = format!("{}::{}", tn, method);
                                    let sig_opt = self
                                        .signature_registry
                                        .get_signature(&qualified)
                                        .or_else(|| self.signature_registry.get_signature(method));
                                    if let Some(sig) = sig_opt {
                                        if sig.has_self_receiver
                                            && sig.param_ownership.first()
                                                == Some(&crate::analyzer::OwnershipMode::Owned)
                                            && !obj_str.ends_with(".clone()")
                                        {
                                            obj_str = format!("{}.clone()", obj_str);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // DOUBLE-CLONE SAFETY NET: If the object was auto-cloned by the FieldAccess
                // handler and this IS a .clone() call, strip the redundant auto-clone.
                // e.g., "stack.item.clone()" from auto-clone + ".clone()" from source
                //     → should be "stack.item.clone()", not "stack.item.clone().clone()"
                if method == "clone" && obj_str.ends_with(".clone()") {
                    obj_str = obj_str[..obj_str.len() - 8].to_string();
                }

                // TDD FIX: Option::unwrap() move error prevention
                // TDD FIX: AUTO-CLONE Option::unwrap() on borrowed fields
                // When calling .unwrap() on a borrowed Option field, we must clone before unwrap:
                //   node.children.unwrap() where node is &Node → ERROR: cannot move from &Option
                //   node.children.clone().unwrap() → ✅ OK
                // THE WINDJAMMER WAY: Users write .unwrap() naturally, compiler handles ownership
                if method == "unwrap" {
                    // Check if object is a field access (node.children) that needs clone
                    let needs_clone = if let Expression::FieldAccess {
                        object: field_obj, ..
                    } = object
                    {
                        // Is this accessing a field on a borrowed parameter?
                        if let Expression::Identifier { ref name, .. } = **field_obj {
                            // Check if the identifier is an inferred borrowed parameter
                            self.inferred_borrowed_params.contains(name)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if needs_clone && !obj_str.contains(".clone()") {
                        obj_str = format!("{}.clone()", obj_str);
                    }
                }

                // E0507 fix: Option::map on self.field with &self must use .as_ref().map(...)
                // self.children.map(|c| ...) with &self → self.children.as_ref().map(|c| ...)
                if method == "map"
                    && self.inferred_borrowed_params.contains("self")
                    && self.codegen_expression_traces_to_self(object)
                {
                    if !obj_str.contains(".as_ref()") {
                        obj_str = format!("{}.as_ref()", obj_str);
                    }
                }

                // BUG #8 FIX: Look up method signature with qualified name (Type::method)
                // First try to infer the type from the object expression
                let type_name = self.infer_type_name(object);
                let method_signature = if let Some(ref type_name) = type_name {
                    let qualified_name = format!("{}::{}", type_name, method);
                    let mut sig = self
                        .signature_registry
                        .get_signature(&qualified_name)
                        .cloned();
                    // Validate: if the signature's param count doesn't match the call's
                    // argument count, it's a name collision (e.g., two different types
                    // both named Ability with different activate methods). In that case,
                    // try module-qualified alternatives from the registry.
                    if let Some(ref found_sig) = sig {
                        let expected_args = if found_sig.has_self_receiver {
                            found_sig.param_ownership.len().saturating_sub(1)
                        } else {
                            found_sig.param_ownership.len()
                        };
                        if expected_args != arguments.len() {
                            // Wrong signature due to name collision; try alternatives
                            sig = None;
                            for (key, alt_sig) in &self.signature_registry.signatures {
                                if key.ends_with(&format!("::{}", qualified_name))
                                    && key != &qualified_name
                                {
                                    let alt_args = if alt_sig.has_self_receiver {
                                        alt_sig.param_ownership.len().saturating_sub(1)
                                    } else {
                                        alt_sig.param_ownership.len()
                                    };
                                    if alt_args == arguments.len() {
                                        sig = Some(alt_sig.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    sig
                    // CRITICAL: Do NOT fall back to unqualified method name lookup!
                    // Unqualified lookup for common names like "get", "remove", "contains"
                    // can match WRONG user-defined methods (e.g., ComponentArray::get when
                    // we want HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                    // When the qualified name isn't found, method_signature stays None and
                    // the stdlib heuristics in should_add_ref handle common patterns correctly.
                } else {
                    if super::stdlib_method_traits::is_common_stdlib_method(method) {
                        None
                    } else {
                        self.signature_registry
                            .get_signature(method)
                            .cloned()
                            .or_else(|| {
                                let suffix_sig = self
                                    .signature_registry
                                    .find_signature_ending_with(method)
                                    .cloned();
                                if let Some(ref sig) = suffix_sig {
                                    let expected_args = if sig.has_self_receiver {
                                        sig.param_ownership.len().saturating_sub(1)
                                    } else {
                                        sig.param_ownership.len()
                                    };
                                    if expected_args == arguments.len() {
                                        return suffix_sig;
                                    }
                                }
                                None
                            })
                    }
                };

                // Float method argument context: for methods like clamp/max/min on float
                // receivers, arguments should use the same float type as the receiver.
                let prev_float_target = self.assignment_float_target_type.clone();
                let receiver_float_type = self.infer_expression_type(object);
                let is_float_method = crate::type_classification::is_float_receiver_method(method);
                if is_float_method {
                    if let Some(ref rft) = receiver_float_type {
                        match rft {
                            Type::Custom(n) if n == "f64" => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f64".to_string()));
                            }
                            Type::Custom(n) if n == "f32" => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f32".to_string()));
                            }
                            Type::Float => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f64".to_string()));
                            }
                            _ => {}
                        }
                    }
                }

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                        // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                        // Fix: Suppress clone when param expects Borrowed -> just add & to field
                        let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                        let param_expects_borrowed = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                            .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Borrowed));

                        const AUTO_BORROW_METHODS: &[&str] = &["push_str", "extend_from_slice"];
                        let is_auto_borrow_target = AUTO_BORROW_METHODS.contains(&method) && i == 0;

                        let prev_suppress = self.suppress_borrowed_clone;
                        if (param_expects_borrowed || is_auto_borrow_target)
                            && matches!(arg, Expression::FieldAccess { .. } | Expression::Identifier { .. })
                        {
                            self.suppress_borrowed_clone = true;
                        }

                        // CRITICAL: Reset in_field_access_object for method argument generation.
                        // Same rationale as function call arguments — method arguments are
                        // independent expressions, not part of a field/method/index chain.
                        // TDD FIX: STRIP explicit &ref when parameter expects owned value.
                        // WINDJAMMER PHILOSOPHY: The developer shouldn't need to think about &.
                        // If the user writes `&object.transform` but the method takes `Transform` (owned),
                        // the compiler strips the & and passes by value (Copy types) or moves.
                        // Example: self.render_transform(&object.transform) → self.render_transform(object.transform)
                        //
                        // TDD FIX: ALSO strip explicit & for HashMap/BTreeMap key methods with &String arguments.
                        // HashMap<String, V>.contains_key() expects &str, not &&String.
                        // User writes: map.contains_key(&key) where key is inferred as &String
                        // Compiler generates: map.contains_key(key) which auto-derefs &String to &str ✅
                        let arg_to_generate = if let Expression::Unary {
                            op: crate::parser::UnaryOp::Ref,
                            operand,
                            ..
                        } = arg
                        {
                            let is_hashmap_key_method =
                                super::stdlib_method_traits::is_map_key_method(method) && i == 0;

                            if is_hashmap_key_method {
                                // Strip explicit `&ident` for map keys: `should_add_ref` will add `&` back when the
                                // Rust type is owned or a Copy `K` that still needs `&K`. For `key: &str` / `&String`
                                // parameters, `should_add_ref` stays false → we emit `get(key)` not `get(&key)` (E0277).
                                if let Expression::Identifier { .. } = &**operand {
                                    operand
                                } else {
                                    arg
                                }
                            } else if let Some(ref sig) = method_signature {
                                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                                let param_is_owned = sig
                                    .param_ownership
                                    .get(sig_param_idx)
                                    .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Owned));
                                if param_is_owned {
                                    operand // Strip & — generate the inner expression
                                } else {
                                    arg // Keep the & — parameter expects a reference
                                }
                            } else {
                                arg // No signature info — keep as-is
                            }
                        } else {
                            arg // Not a & expression — keep as-is
                        };

                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;
                        let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                        self.coerce_string_literals_to_owned = false;
                        let prev_match_arm_str = self.in_match_arm_needing_string;
                        self.in_match_arm_needing_string = false;
                        let mut arg_str = self.generate_expression(arg_to_generate);
                        self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                        self.in_match_arm_needing_string = prev_match_arm_str;
                        self.in_field_access_object = prev_field_access_obj;

                        // TDD FIX: PHASE 2 CALL-SITE OPTIMIZATION
                        // Strip unnecessary .to_string() when parameter was optimized to &str
                        // Example: User writes `loader.load("name".to_string())` but Phase 2 optimized
                        // the signature from `fn load(self, name: String)` to `fn load(self, name: &str)`.
                        // Result: Call site should be `loader.load("name")` not `loader.load("name".to_string())`
                        //
                        // IMPORTANT: Only strip for &str parameters, NOT &String parameters!
                        // &String parameters still need .to_string() (creates String, then borrows it)
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                                // Check if parameter is specifically &str (not &String!)
                                let param_is_str_slice_ref = if let Type::Reference(inner) = param_type {
                                    matches!(&**inner, Type::Custom(name) if name == "str")
                                } else {
                                    false
                                };
                                if param_is_str_slice_ref && arg_str.ends_with(".to_string()") {
                                    // Strip .to_string() - &str accepts string literals directly
                                    arg_str = arg_str[..arg_str.len() - 12].to_string();
                                }
                            }
                        }

                        // TDD FIX: Vec index methods require usize arguments.
                        // Int inference may resolve the literal to i32/u32/i64/u64 due to
                        // conflicting constraints. Fix at codegen level: rewrite any
                        // integer suffix to _usize for the first argument of known
                        // index-taking methods.
                        if i == 0
                            && super::stdlib_method_traits::is_index_taking_method(method)
                        {
                            let is_int_literal = matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                                    ..
                                }
                            );
                            if is_int_literal {
                                let int_suffixes =
                                    ["_i32", "_i64", "_u32", "_u64", "_i16", "_u16", "_i8", "_u8"];
                                for suffix in &int_suffixes {
                                    if arg_str.ends_with(suffix) {
                                        arg_str = format!(
                                            "{}_usize",
                                            &arg_str[..arg_str.len() - suffix.len()]
                                        );
                                        break;
                                    }
                                }
                            }
                        }

                        // TDD FIX: AUTO-WRAP function pointers in iterator adapter methods.
                        // Rust's .filter()/.any()/.find() on iter() yield &&T, expecting FnMut(&&T) -> bool,
                        // but bare function pointers fn(&T) -> bool don't auto-deref.
                        // THE WINDJAMMER WAY: Users write the natural `filter(predicate)` and the
                        // compiler generates `filter(|__e| predicate(__e))`.
                        if i == 0
                            && super::stdlib_method_traits::is_closure_taking_method(method)
                            && matches!(arg, Expression::Identifier { .. })
                        {
                            // Bare identifier (function pointer) passed to iterator adapter -
                            // wrap in closure so Rust's auto-deref handles &&T -> &T.
                            arg_str = format!("|__e| {}(__e)", arg_str);
                        }

                        // TDD FIX: String literal ownership conversion
                        // Windjammer philosophy: "sword" should work whether parameter wants String or &String
                        // CRITICAL: Do NOT convert for explicit &str parameters! Only for inferred &String.
                        let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                        let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                        let param_ownership = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx));
                        let string_literal_converted = if is_string_literal {
                            // Check what the parameter wants

                            // CRITICAL: Check if parameter is explicitly &str (not inferred &String)
                            // Explicit &str parameters should NOT get .to_string() conversion
                            let param_type = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_types.get(sig_param_idx));
                            let is_explicit_str_ref = if let Some(Type::Reference(inner)) = param_type {
                                matches!(**inner, Type::String) ||
                                matches!(**inner, Type::Custom(ref s) if s == "str")
                            } else {
                                false
                            };

                            if is_explicit_str_ref {
                                // Explicit &str parameter - no conversion needed
                                false
                            } else {
                                match param_ownership {
                                    Some(&OwnershipMode::Owned) | Some(&OwnershipMode::Borrowed) => {
                                        // TDD FIX: Both Owned and Borrowed string params need .to_string()
                                        // Owned → String needs .to_string()
                                        // Borrowed → &String needs .to_string() (then & is added later)
                                        // String literals are &str, must allocate to get String/&String
                                        arg_str = format!("{}.to_string()", arg_str);
                                        true // Mark that we converted
                                    }
                                    _ => {
                                        // No signature info - use heuristic (fallback to old logic)
                                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(i, method, &method_signature) {
                                            arg_str = format!("{}.to_string()", arg_str);
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                }
                            }
                        } else {
                            false
                        };

                        // TDD FIX: If we converted string literal for Borrowed parameter,
                        // we need to add & since .to_string() produces String but param wants &String
                        if string_literal_converted {
                            if let Some(&OwnershipMode::Borrowed) = param_ownership {
                                // .to_string() produces String, but Borrowed param wants &String
                                // So we need to add &
                                arg_str = format!("&{}", arg_str);
                            }
                        }

                        // TDD FIX: AUTO-CONVERT &str → String for method calls
                        // When passing a Phase 2 optimized &str parameter to a method expecting owned String, convert it
                        // This handles cases like: HashMap::insert(key, value) where key is &str but insert expects String
                        if let Expression::Identifier { name, .. } = arg_to_generate {
                            let is_str_ref_optimized =
                                self.str_ref_optimized_params.contains(name.as_str());

                            if is_str_ref_optimized {
                                let sig_param_idx = if method_signature
                                    .as_ref()
                                    .is_some_and(|s| s.has_self_receiver)
                                {
                                    i + 1
                                } else {
                                    i
                                };
                                if !crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::callee_param_is_rust_str_slice(
                                    &method_signature,
                                    sig_param_idx,
                                ) {
                                    let expects_owned = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                        i,
                                        method,
                                        &method_signature,
                                    );

                                    if expects_owned
                                        && !arg_str.ends_with(".to_string()")
                                        && !arg_str.ends_with(".clone()")
                                    {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    }
                                }
                            }
                        }

                        // AUTO .clone(): Add .clone() when needed for borrowed values
                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_clone(
                            arg,
                            &arg_str,
                            method,
                            i,
                            &method_signature,
                            &self.borrowed_iterator_vars,
                            &self.current_function_params,
                            &self.inferred_borrowed_params,
                            &self.current_function_return_type,
                        ) {
                            arg_str = format!("{}.clone()", arg_str);
                        }

                        // DOGFOODING FIX: Vec indexing vec[idx] passed to owned param (e.g. push)
                        // should_add_clone handles Identifier/FieldAccess; Index needs explicit check
                        // Vec::push uses stdlib heuristics (method_signature=None) - param 0 expects Owned
                        if let Expression::Index { .. } = arg {
                            let sig_param_idx = method_signature
                                .as_ref()
                                .map(|s| if s.has_self_receiver { i + 1 } else { i })
                                .unwrap_or(i);
                            let param_expects_owned = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                                .is_some_and(|&o| matches!(o, OwnershipMode::Owned))
                                || (method == "push" && i == 0);
                            if param_expects_owned && !arg_str.ends_with(".clone()") {
                                let inferred = self.infer_expression_type(arg);
                                let is_copy = inferred.as_ref().is_some_and(|t| self.is_type_copy(t));
                                if is_copy {
                                    if arg_str.starts_with("&") {
                                        arg_str = arg_str
                                            .strip_prefix('&')
                                            .unwrap_or(&arg_str)
                                            .to_string();
                                    }
                                } else {
                                    // Non-Copy or unknown type: clone to prevent E0507
                                    if arg_str.starts_with("&") {
                                        arg_str = format!("({}).clone()", arg_str);
                                    } else {
                                        arg_str = format!("{}.clone()", arg_str);
                                    }
                                }
                            }
                        }

                        // TDD FIX: Strip unnecessary .clone() when method param is Borrowed
                        // When a field like `ingredient.item_id` is auto-cloned by the
                        // FieldAccess handler (because owner is borrowed), but the method
                        // expects &String (Borrowed), the clone is wasteful:
                        //   &ingredient.item_id.clone()  ← clones then borrows (wasteful)
                        //   &ingredient.item_id          ← borrows directly (correct)
                        // Strip the .clone() so should_add_ref can add & cleanly.
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                            if param_is_borrowed && arg_str.ends_with(".clone()") {
                                arg_str = arg_str[..arg_str.len() - 8].to_string();
                            }
                        }

                        // AUTO-MUT-BORROW: Add &mut when parameter expects MutBorrowed
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_mut_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
                            if param_is_mut_borrowed {
                                let is_already_mut_ref =
                                    if let Expression::Identifier { name, .. } = arg {
                                        let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                            param.name == *name
                                                && matches!(&param.type_, Type::MutableReference(_))
                                        });
                                        let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                        explicit_mut_ref || inferred_mut_ref
                                    } else {
                                        false
                                    };
                                if !expression_helpers::is_reference_expression(arg)
                                    && !is_already_mut_ref
                                {
                                    if arg_str.ends_with(".clone()") {
                                        arg_str = arg_str[..arg_str.len() - 8].to_string();
                                    }
                                    if arg_str.starts_with("&") && !arg_str.starts_with("&mut ") {
                                        arg_str = arg_str[1..].to_string();
                                    }
                                    arg_str = format!("&mut {}", arg_str);
                                }
                            }
                        }

                        // AUTO-REF: Add & when parameter expects reference but arg is owned
                        if !string_literal_converted {
                            // Use `arg_to_generate` (after stripping explicit `&` for map keys / owned params)
                            // so `should_add_ref` sees `key` not `&key` — otherwise the Unary(Ref) early-return
                            // skips HashMap `str` key handling and we emit `get(&key)` for `key: &str` (E0277).
                            let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                arg_to_generate,
                                &arg_str,
                                method,
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
                                &self.match_arm_bindings, // TDD FIX: Pass match arm bindings for E0308 fix
                            );
                            if should_ref {
                                if let Expression::Cast { .. } = arg_to_generate {
                                    arg_str = format!("&({})", arg_str);
                                } else {
                                    arg_str = format!("&{}", arg_str);
                                }
                            }
                        }

                        let sig_param_idx_str_field = method_signature.as_ref().map(|sig| {
                            if sig.has_self_receiver {
                                i + 1
                            } else {
                                i
                            }
                        });
                        if let Some(idx) = sig_param_idx_str_field {
                            arg_str = self.ensure_ref_for_owned_string_field_when_callee_expects_str(
                                &method_signature,
                                idx,
                                arg_to_generate,
                                arg_str,
                                string_literal_converted,
                            );
                        }

                        // AUTO-BORROW: Methods that take &T or &[T] should auto-borrow
                        // when given owned values. Eliminates Rust leakage in .wj files.
                        let auto_borrow_methods = ["push_str", "extend_from_slice"];
                        let map_key_methods = ["remove", "get", "contains_key", "entry"];
                        let is_map_method = map_key_methods.contains(&method)
                            && i == 0
                            && {
                                let obj_ty = self.infer_expression_type(object);
                                obj_ty.as_ref().is_some_and(|t| matches!(t,
                                    Type::Parameterized(base, _) if base == "HashMap" || base == "BTreeMap" || base == "Map"
                                ))
                            };
                        if (auto_borrow_methods.contains(&method) || is_map_method) && i == 0 {
                            let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                            let arg_already_ref = {
                                let arg_ty = self.infer_expression_type(arg);
                                let ty_is_ref = arg_ty.as_ref().is_some_and(|t| matches!(t,
                                    Type::Reference(_) | Type::MutableReference(_)
                                ) || matches!(t, Type::Custom(n) if n == "&str"));
                                let param_is_borrowed = match arg {
                                    Expression::Identifier { name, .. } =>
                                        self.inferred_borrowed_params.contains(&name.to_string()),
                                    _ => false,
                                };
                                ty_is_ref || param_is_borrowed
                            };
                            if !is_string_literal && !arg_str.starts_with('&') && !arg_already_ref {
                                let needs_borrow = matches!(arg,
                                    Expression::Identifier { .. } |
                                    Expression::FieldAccess { .. } |
                                    Expression::MethodCall { .. }
                                );
                                if needs_borrow {
                                    arg_str = format!("&{}", arg_str);
                                }
                            }
                        }

                        // AUTO-CAST int → float: when parameter expects f32/f64 but argument is int
                        // Skip when signature has a collision (different types with same name).
                        {
                            let effective_sig = method_signature.as_ref()
                                .or_else(|| self.signature_registry.get_signature(method));
                            let has_collision = self.signature_registry.has_collision(method);
                            if let Some(sig) = effective_sig {
                                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
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
                            }
                        }

                        // Restore suppress flag
                        self.suppress_borrowed_clone = prev_suppress;

                        arg_str
                    })
                    .collect();

                // E0499 FIX: Extract temporaries when receiver and arguments both borrow self.
                // Pattern: self.field.method(self.other_method()) generates two &mut self borrows.
                // Fix: { let __wj_tmp0 = self.other_method(); self.field.method(__wj_tmp0) }
                let receiver_borrows_self = self.codegen_expression_traces_to_self(object);
                let mut self_borrow_temps: Vec<(String, String)> = Vec::new();
                let args = if receiver_borrows_self {
                    let needs_extraction = arguments.iter().any(|(_label, arg)| self.expression_borrows_self(arg));
                    if needs_extraction {
                        args.into_iter()
                            .enumerate()
                            .map(|(i, arg_str)| {
                                let (_label, arg_expr) = &arguments[i];
                                if self.expression_borrows_self(arg_expr) {
                                    let temp_name = format!("__wj_tmp{}", i);
                                    self_borrow_temps.push((temp_name.clone(), arg_str));
                                    temp_name
                                } else {
                                    arg_str
                                }
                            })
                            .collect()
                    } else {
                        args
                    }
                } else {
                    args
                };

                // Restore float target type after argument generation
                self.assignment_float_target_type = prev_float_target;

                // Generate turbofish if present, or infer for collect() from return type
                let turbofish = if let Some(types) = type_args {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    format!("::<{}>", type_strs.join(", "))
                } else if method == "collect" {
                    if let Some(target_ty) = &self.collect_target_type {
                        format!("::<{}>", self.type_to_rust(target_ty))
                    } else if let Some(ret_ty) = &self.current_function_return_type {
                        format!("::<{}>", self.type_to_rust(ret_ty))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                // Special case: empty method name means turbofish on a function call (func::<T>())
                if method.is_empty() {
                    return format!("{}{}({})", obj_str, turbofish, args.join(", "));
                }

                // Special case: substring(start, end) -> &text[start..end]
                if method == "substring" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // Special case: contains() with String argument needs .as_str()
                // String::contains() expects &str, not String
                if method == "contains" && args.len() == 1 {
                    // Check if argument is a method call that returns String (like to_lowercase())
                    if let Some((_label, arg)) = arguments.first() {
                        if matches!(arg, Expression::MethodCall { method: m, .. } if
                            m == "to_lowercase" || m == "to_uppercase" ||
                            m == "to_string" || m == "trim" || m == "clone")
                        {
                            // The argument is String, needs .as_str()
                            return format!("{}.{}({}.as_str())", obj_str, method, args[0]);
                        }
                    }
                }

                // Determine separator: :: for static calls, . for instance methods
                // - Type/Module (starts with uppercase): use ::
                // - Variable (starts with lowercase): use .
                let separator = match object {
                    Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
                    Expression::Identifier { name, .. } => {
                        // Check for known module/crate names that should use ::
                        // Note: Avoid common variable names like "path", "config" which are used as variables
                        let known_modules = [
                            "std",
                            "serde_json",
                            "serde",
                            "tokio",
                            "reqwest",
                            "sqlx",
                            "chrono",
                            "sha2",
                            "bcrypt",
                            "base64",
                            "rand",
                            "Vec",
                            "String",
                            "Option",
                            "Result",
                            "Box",
                            "Arc",
                            "Mutex",
                            "Utc",
                            "Local",
                            "DEFAULT_COST",
                            // Stdlib modules (avoid common variable names)
                            "mime",
                            "http",
                            "fs",
                            "strings",
                            // NOTE: "json" removed - it's a common variable name!
                            // Use "serde_json" for the module instead
                            "regex",
                            "cli",
                            "log",
                            "crypto",
                            "io",
                            "env",
                            "time",
                            "sync",
                            "thread",
                            "collections",
                            "cmp",
                        ];

                        // Type or module (uppercase) vs variable (lowercase)
                        if name.chars().next().is_some_and(|c| c.is_uppercase())
                            || name.contains('.')
                            || known_modules.contains(&name.as_str())
                        {
                            "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                        } else {
                            "." // x.abs(), value.method()
                        }
                    }
                    Expression::FieldAccess { ref object, .. } => {
                        // Check if this is a module path (e.g., std::fs) or a field access (e.g., self.count)
                        // If the object is an identifier that looks like a module, use ::
                        // Otherwise, use . for instance methods on fields
                        match object {
                            Expression::Identifier { name, .. } => {
                                if name.chars().next().is_some_and(|c| c.is_uppercase())
                                    || name == "std"
                                {
                                    "::" // Module::path::method() -> static method
                                } else {
                                    "." // self.field.method() or variable.field.method() -> instance method
                                }
                            }
                            _ => ".", // Default to instance method
                        }
                    }
                    _ => ".", // Instance method on expressions
                };

                // SPECIAL CASE: .slice() method is our desugared slice syntax [start..end]
                // Convert it back to proper Rust slice syntax
                // For strings, we need to add & to get &str (a reference)
                if method == "slice" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // E0308: Borrowed Windjammer `string` parameters lower to `&str`. `.clone()` on `&str`
                // is still `&str`, but users mean an owned copy → emit `.to_string()`.
                if method == "clone" && arguments.is_empty() {
                    if let Expression::Identifier { name, .. } = object {
                        if self.inferred_borrowed_params.contains(name.as_str())
                            && self
                                .current_function_params
                                .iter()
                                .find(|p| p.name == *name)
                                .is_some_and(|p| {
                                    crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                })
                        {
                            return format!("{}.to_string()", obj_str);
                        }
                    }
                }

                // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
                // DISABLED: This optimization was too aggressive and removed needed clones
                // TODO: Make this more conservative - only remove clone when we can prove
                // the value is Copy or when it's the last use
                // if method == "clone" && arguments.is_empty() {
                //     if let Expression::Identifier { name: ref var_name, location: None } = **object {
                //         if self.clone_optimizations.contains(var_name) {
                //             // Skip the .clone(), just return the variable (or borrow if needed)
                //             return obj_str;
                //         }
                //     }
                // }

                // UI FRAMEWORK: Check if we need to add .to_vnode() for .child() methods
                // DISABLED: Too aggressive - needs type checking to determine if parameter expects VNode
                // TODO: Re-enable with proper type checking when VNode type bindings are implemented
                let processed_args = args;

                // WINDJAMMER STDLIB → RUST TRANSLATION
                // Some Windjammer methods don't exist in Rust and need translation.
                //
                // reversed() → into_iter().rev().collect::<Vec<_>>()
                if method == "reversed" && processed_args.is_empty() {
                    return format!("{}.into_iter().rev().collect::<Vec<_>>()", obj_str);
                }
                // enumerate() → iter().enumerate()
                // Rust Vec doesn't have .enumerate() — only iterators do.
                // But if the object already ends with .iter(), .iter_mut(), or
                // .into_iter(), don't add a redundant .iter() prefix.
                if method == "enumerate" && processed_args.is_empty() {
                    let already_iterator = obj_str.ends_with(".iter()")
                        || obj_str.ends_with(".iter_mut()")
                        || obj_str.ends_with(".into_iter()");
                    if already_iterator {
                        return format!("{}.enumerate()", obj_str);
                    } else {
                        return format!("{}.iter().enumerate()", obj_str);
                    }
                }

                // TDD FIX (Bug #3): Extract format!() macros in method arguments too
                let has_format_arg = processed_args
                    .iter()
                    .any(|arg_str| arg_str.contains("format!("));

                let base_expr = if has_format_arg {
                    // Extract format!() macros to temp variables
                    let mut temp_decls = String::new();
                    let mut temp_counter = 0;
                    let fixed_args: Vec<String> = processed_args
                        .iter()
                        .map(|arg_str| {
                            if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                // Strip leading & if present (was added by argument processing)
                                let format_expr = if arg_str.starts_with("&") {
                                    arg_str.strip_prefix("&").unwrap()
                                } else {
                                    arg_str
                                };
                                // Extract to temp var
                                let temp_name = format!("_temp{}", temp_counter);
                                temp_counter += 1;
                                temp_decls
                                    .push_str(&format!("let {} = {}; ", temp_name, format_expr));

                                // When the method expects &str (push_str, extend_from_slice),
                                // add & to pass borrowed temp. Otherwise, pass owned value.
                                let method_needs_borrow =
                                    matches!(method, "push_str" | "extend_from_slice");
                                if arg_str.starts_with("&") || method_needs_borrow {
                                    format!("&{}", temp_name)
                                } else {
                                    temp_name
                                }
                            } else {
                                arg_str.clone()
                            }
                        })
                        .collect();

                    // Wrap in block: { let _temp0 = format!(...); obj.method(&_temp0, ...) }
                    format!(
                        "{{ {}{}{}{}{}({}) }}",
                        temp_decls,
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        fixed_args.join(", ")
                    )
                } else {
                    format!(
                        "{}{}{}{}({})",
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        processed_args.join(", ")
                    )
                };

                // E0499 FIX: Wrap in block with temporaries if self-borrow extraction was needed
                let base_expr = if !self_borrow_temps.is_empty() {
                    let mut temp_decls = String::new();
                    for (name, value) in &self_borrow_temps {
                        temp_decls.push_str(&format!("let {} = {}; ", name, value));
                    }
                    format!("{{ {}{} }}", temp_decls, base_expr)
                } else {
                    base_expr
                };

                base_expr
    }
}

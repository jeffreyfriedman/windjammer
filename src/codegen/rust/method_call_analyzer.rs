#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]

/// Method Call Analyzer - Smart auto-conversions for method arguments
///
/// The Windjammer Philosophy: The compiler should handle mechanical transformations
/// automatically, letting developers focus on logic, not syntax.
///
/// This module determines when to automatically add:
/// - & (reference) for borrowed parameters
/// - .clone() for moved values that are used again
/// - .to_string() for string literals passed to owned String parameters
/// - .cloned() for Option<&T> -> Option<T> conversions
///
/// These transformations are based on:
/// 1. Method signatures (when available)
/// 2. Stdlib method patterns (fallback heuristics)
/// 3. Type analysis (Copy types, etc.)
use crate::analyzer::OwnershipMode;
use crate::parser::{Expression, Literal, OwnershipHint, Parameter, Type};
use std::collections::HashSet;

/// Analyzes method calls to determine what automatic conversions are needed
pub struct MethodCallAnalyzer;

impl MethodCallAnalyzer {
    /// Determine if we should add & to this argument
    /// 
    /// NEW ARCHITECTURE: Uses type-based signature lookup to make decisions
    /// Replaces all hard-coded method name heuristics with proper type analysis
    #[allow(clippy::too_many_arguments)]
    pub fn should_add_ref(
        arg: &Expression,
        arg_str: &str,
        method: &str,
        param_idx: usize,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        usize_variables: &HashSet<String>,
        current_function_params: &[Parameter],
        borrowed_iterator_vars: &HashSet<String>,
        inferred_borrowed_params: &HashSet<String>,
        arg_count: usize,
        receiver_type_name: Option<&str>,
        local_var_types: Option<&std::collections::HashMap<String, Type>>,
        // NEW: Signature registries for type-based method resolution
        stdlib_signatures: Option<&std::collections::HashMap<
            String,
            std::collections::HashMap<String, crate::codegen::rust::generator::MethodSignature>,
        >>,
        user_signatures: Option<&std::collections::HashMap<
            String,
            std::collections::HashMap<String, crate::codegen::rust::generator::MethodSignature>,
        >>,
    ) -> bool {
        // String literals are ALREADY &str - never add &
        let is_string_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        );
        if is_string_literal {
            return false;
        }

        // Integer literals are Copy types - never add &
        // Example: vec.remove(0) should stay as-is, NOT become vec.remove(&0)
        // Integer literals are passed by value, not by reference
        let is_integer_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                ..
            }
        );
        if is_integer_literal {
            return false;
        }

        // Float literals are also Copy types - never add &
        let is_float_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::Float(_),
                ..
            }
        );
        if is_float_literal {
            return false;
        }

        // Boolean literals are Copy types - never add &
        let is_bool_literal = matches!(
            arg,
            Expression::Literal {
                value: Literal::Bool(_),
                ..
            }
        );
        if is_bool_literal {
            return false;
        }

        // Struct literals are always rvalues — don't auto-borrow them.
        // The callee's generated parameter type determines whether
        // the struct is passed owned or by reference, and we should
        // trust the generated signature rather than the analyzer's
        // potentially stale ownership inference.
        if matches!(arg, Expression::StructLiteral { .. }) {
            return false;
        }

        // Already has & - don't add another
        if arg_str.starts_with('&') {
            return false;
        }

        // Already an explicit reference
        if matches!(
            arg,
            Expression::Unary {
                op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef,
                ..
            }
        ) {
            return false;
        }

        // NEW ARCHITECTURE: Try signature-based lookup FIRST
        // This replaces ALL hard-coded heuristics with proper type-based decisions
        if let Some(receiver_type) = receiver_type_name {
            // Try stdlib signatures first (Vec, String, HashMap, etc.)
            if let Some(stdlib_sigs) = stdlib_signatures {
                // Strip generic params (Vec<String> → Vec)
                let base_type = receiver_type.split('<').next().unwrap_or(receiver_type);
                
                if let Some(methods) = stdlib_sigs.get(base_type) {
                    if let Some(sig) = methods.get(method) {
                        // Found signature! Use it to make decision
                        if let Some(param_type) = sig.param_types.get(param_idx) {
                            // Check if parameter expects &str and argument is String
                            let param_is_str_ref = matches!(
                                param_type,
                                Type::Reference(inner) if matches!(&**inner, Type::Custom(s) if s == "str")
                            );
                            
                            if param_is_str_ref {
                                // Parameter wants &str - check if argument is owned String
                                if let Expression::Identifier { name, .. } = arg {
                                    // Check local_var_types
                                    if let Some(local_types) = local_var_types {
                                        if let Some(var_type) = local_types.get(name.as_str()) {
                                            if crate::codegen::rust::types::is_windjammer_text_type(var_type) {
                                                return true; // String → &str
                                            }
                                        }
                                    }
                                    
                                    // Check function parameters
                                    let is_owned_string = current_function_params.iter().any(|p| {
                                        p.name == *name
                                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                            && !inferred_borrowed_params.contains(name.as_str())
                                    });
                                    if is_owned_string {
                                        return true;
                                    }
                                }
                            }
                            
                            // Use the signature's ownership mode
                            // BUT: Don't add & if the argument is already borrowed (e.g., &str param)
                            if let Some(&ownership) = sig.param_ownership.get(param_idx) {
                                if matches!(ownership, crate::analyzer::OwnershipMode::Borrowed) {
                                    // Check if argument is already a borrowed parameter
                                    if let Expression::Identifier { name, .. } = arg {
                                        // If it's in inferred_borrowed_params, it's already &T
                                        if inferred_borrowed_params.contains(name.as_str()) {
                                            return false; // Already &T, don't add another &
                                        }
                                    }
                                    return true; // Needs &
                                }
                            }
                        }
                    }
                }
            }
            
            // Try user-defined signatures
            if let Some(user_sigs) = user_signatures {
                if let Some(methods) = user_sigs.get(receiver_type) {
                    if let Some(sig) = methods.get(method) {
                        // Found user signature! Use it
                        if let Some(param_type) = sig.param_types.get(param_idx) {
                            let param_is_str_ref = matches!(
                                param_type,
                                Type::Reference(inner) if matches!(&**inner, Type::Custom(s) if s == "str")
                            );
                            
                            if param_is_str_ref {
                                if let Expression::Identifier { name, .. } = arg {
                                    if let Some(local_types) = local_var_types {
                                        if let Some(var_type) = local_types.get(name.as_str()) {
                                            if crate::codegen::rust::types::is_windjammer_text_type(var_type) {
                                                return true;
                                            }
                                        }
                                    }
                                    
                                    let is_owned_string = current_function_params.iter().any(|p| {
                                        p.name == *name
                                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                            && !inferred_borrowed_params.contains(name.as_str())
                                    });
                                    if is_owned_string {
                                        return true;
                                    }
                                }
                            }
                            
                            if let Some(&ownership) = sig.param_ownership.get(param_idx) {
                                if matches!(ownership, crate::analyzer::OwnershipMode::Borrowed) {
                                    // Check if argument is already a borrowed parameter
                                    if let Expression::Identifier { name, .. } = arg {
                                        if inferred_borrowed_params.contains(name.as_str()) {
                                            return false; // Already &T, don't add another &
                                        }
                                    }
                                    return true; // Needs &
                                }
                            }
                        }
                    }
                }
            }
        }

        // Method call results (like input.is_key_down()) generally shouldn't be auto-borrowed.
        // Exception 1: when we have a user-defined signature that says the parameter is Borrowed
        // AND the param type is non-Copy (e.g., `path: &String`), then .to_string() results
        // DO need & added. The signature check downstream handles this.
        // Exception 2: HashMap/BTreeMap key methods ALWAYS need &key, even for method call results.
        // e.g., self.animations.contains_key(state.animation_name()) needs &state.animation_name()
        if matches!(arg, Expression::MethodCall { .. }) {
            let is_map_key_method = matches!(
                method,
                "contains_key" | "get" | "get_mut" | "remove" | "get_key_value"
            ) && param_idx == 0;
            let is_known_map = receiver_type_name.is_some_and(|n| {
                let base = n.split('<').next().unwrap_or(n);
                matches!(base, "HashMap" | "BTreeMap" | "IndexMap")
            });
            if is_map_key_method && is_known_map {
                return true;
            }
            // Allow through ONLY if we have a user-defined method signature that says Borrowed
            if let Some(sig) = method_signature {
                let sig_param_idx = if sig.has_self_receiver {
                    param_idx + 1
                } else {
                    param_idx
                };
                let is_borrowed = sig
                    .param_ownership
                    .get(sig_param_idx)
                    .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                if !is_borrowed {
                    return false;
                }
                // Fall through to let the downstream Copy-type check handle it
            } else {
                return false;
            }
        }

        // BORROWED ITERATOR VARIABLES: Variables from borrowed iterators (.keys(), .values(), .iter())
        // are already borrowed (e.g., &String from .keys()). Don't add another &.
        // Example: for key in map.keys() { map.get(key) }  // key is already &String
        if let Expression::Identifier { name, .. } = arg {
            if borrowed_iterator_vars.contains(name) {
                return false;
            }
        }

        // TDD FIX: HashMap/BTreeMap key methods with &String arguments
        // HashMap<String, V>.contains_key() expects &str, not &String or &&String
        // When passing a &String parameter to these methods, don't add another &
        // Example: fn check(map: HashMap<String, int>, key: string) { map.contains_key(&key) }
        //   Generated: fn check(map: &HashMap<String, i64>, key: &String)
        //   Need: map.contains_key(key) NOT map.contains_key(&key)
        //   Result: &String auto-derefs to &str ✅
        let is_hashmap_key_method = matches!(
            method,
            "contains_key" | "get" | "get_mut" | "remove" | "get_key_value"
        ) && param_idx == 0; // Key is always first argument

        if is_hashmap_key_method {
            if let Expression::Identifier { name, .. } = arg {
                let is_string_type = |t: &Type| {
                    crate::codegen::rust::types::is_windjammer_text_type(t)
                };
                // `key: str` → Rust `key: &str` always; do not emit `&key` (&&str / E0277).
                let is_wj_str_param = current_function_params.iter().any(|param| {
                    param.name == *name && matches!(&param.type_, Type::Custom(s) if s == "str")
                });
                let is_borrowed_string_param = current_function_params
                    .iter()
                    .any(|param| param.name == *name && is_string_type(&param.type_))
                    && inferred_borrowed_params.contains(name);

                if is_wj_str_param || is_borrowed_string_param {
                    return false; // Rust param is already &str / string ref — no extra &
                }
            }
        }

        // TDD FIX: PARAMETERS THAT ARE ALREADY REFERENCE TYPES
        // If a function parameter is declared as &T or &mut T, the identifier
        // itself is already a reference. Don't add another &.
        // Example: fn remove(&mut self, key: &str) { self.items.remove(key) }
        // key is already &str, so we pass it directly, not &key (which would be &&str)
        if let Expression::Identifier { name, .. } = arg {
            if current_function_params.iter().any(|param| {
                param.name == *name
                    && matches!(&param.type_, Type::Reference(_) | Type::MutableReference(_))
            }) {
                return false;
            }
        }

        // SPECIAL CASE: Dereference of Copy types should NOT get &
        // Example: `*entity` (where entity: &Entity and Entity is Copy) should stay as-is,
        // NOT become `&*entity` which is redundant and wrong
        // The dereference produces an owned Copy value, which can be passed directly
        if matches!(
            arg,
            Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                ..
            }
        ) {
            // If we're dereferencing, we're explicitly getting an owned value
            // Don't add & back to it (especially for Copy types)
            return false;
        }

        // SPECIAL CASE: Cast expressions to Copy types should NOT get &
        // Example: `index as usize` should stay as-is, not become `&index as usize`
        // EXCEPTION: HashMap key methods need &(expr as Type) even for Copy casts
        // Example: `names.get(entity_id as i64)` → `names.get(&(entity_id as i64))`
        if let Expression::Cast { type_, .. } = arg {
            if Self::is_copy_type_annotation(type_) {
                let is_known_map = receiver_type_name.is_some_and(|n| {
                    let base = n.split('<').next().unwrap_or(n);
                    matches!(base, "HashMap" | "BTreeMap" | "IndexMap")
                });
                let is_map_key_method = matches!(
                    method,
                    "get" | "get_mut" | "remove" | "contains_key" | "get_key_value"
                ) && param_idx == 0;
                if !(is_known_map && is_map_key_method) {
                    return false;
                }
            }
        }

        // REMOVED: Hard-coded heuristics replaced with type-based signature lookup above
        
        // User-defined methods with names like "remove" should use their actual signature,
        // not stdlib HashMap assumptions.
        // Example: ComponentArray<T>.remove(entity: Entity) takes Entity by value, not &Entity
        if let Some(sig) = method_signature {
            let sig_param_idx = if sig.has_self_receiver {
                param_idx + 1
            } else {
                param_idx
            };
            if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                if matches!(ownership, OwnershipMode::Borrowed) {
                    // CRITICAL FIX: For Copy types, the codegen generates `param: Type`
                    // (not `param: &Type`) even when ownership is Borrowed, because Copy
                    // types are efficient to pass by value. We must NOT add & at the call
                    // site, or we'd pass &i32 to a parameter expecting i32.
                    // Example: fn matches(&self, current_state: i32) — analyzer says Borrowed
                    // but codegen generates `current_state: i32`, not `current_state: &i32`
                    //
                    // TDD FIX: BUT Reference types (&str, &T) are NOT treated as Copy here.
                    // If param type is &str, the generated code has `pattern: &str`,
                    // and the caller passing String needs `&text` to auto-deref to &str.
                    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                        if !matches!(param_type, Type::Reference(_) | Type::MutableReference(_))
                            && Self::is_copy_type_annotation(param_type)
                        {
                            return false; // Copy type — pass by value
                        }
                    }
                    
                    // TDD FIX for E0308: Auto-convert String → &str for match arm bindings
                    // This check applies to ALL expressions, not just identifiers
                    // When parameter expects &str and argument is owned String, add & to auto-deref
                    if let Some(param_type) = sig.param_types.get(sig_param_idx) {
                        let param_is_str_ref = match param_type {
                            Type::Reference(inner) => matches!(&**inner, Type::Custom(s) if s == "str"),
                            _ => false,
                        };
                        
                        if param_is_str_ref {
                            // Parameter expects &str - check if argument is owned String
                            if let Expression::Identifier { name: arg_name, .. } = arg {
                                // Check if this identifier is already a &str parameter
                                let already_rust_str = current_function_params.iter().any(|p| {
                                    p.name == *arg_name
                                        && (matches!(&p.type_, Type::Custom(s) if s == "str")
                                            || (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                                && inferred_borrowed_params.contains(arg_name)))
                                });
                                if already_rust_str {
                                    return false; // Already &str, don't add another &
                                }
                                
                                // Check if it's an owned String parameter
                                let is_owned_string_param = current_function_params.iter().any(|p| {
                                    p.name == *arg_name
                                        && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                        && !inferred_borrowed_params.contains(arg_name)
                                });
                                
                                // Check if it's a local variable with String type (match arm binding, let binding)
                                let is_owned_string_local = local_var_types
                                    .as_ref()
                                    .and_then(|vars| vars.get(arg_name))
                                    .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t));
                                
                                if is_owned_string_param || is_owned_string_local {
                                    return true; // Add & to convert String → &str
                                }
                            }
                            // For non-identifier expressions, if param expects &str, likely need &
                            // (e.g., method calls returning String)
                            // Let the general Borrowed logic below handle it
                        }
                    }
                    
                    // WJ `str` / inferred-borrowed `string` params are already `&str` in Rust;
                    // callee `Borrowed` here means `&str` — passing `&name` would be `&&str` (E0277).
                    if let Expression::Identifier { name: arg_name, .. } = arg {
                        let already_rust_str = current_function_params.iter().any(|p| {
                            p.name == *arg_name
                                && (matches!(&p.type_, Type::Custom(s) if s == "str")
                                    || (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                        && inferred_borrowed_params.contains(arg_name)))
                        });
                        if already_rust_str {
                            return false;
                        }
                        // Note: We intentionally do NOT skip auto-borrow for owned
                        // caller args here. The callee signature's Borrowed ownership
                        // accurately reflects the generated Rust parameter type (&T)
                        // in most cases. Trait impl mismatches are handled upstream.
                    }
                    return true; // Non-Copy Borrowed type needs &
                }
                return false; // Owned or MutBorrowed — don't add &
            }
        }

        // No signature available - fall back to stdlib heuristics
        let is_stdlib_method = matches!(
            method,
            "remove"
                | "get"
                | "contains_key"
                | "get_mut"
                | "contains"
                | "binary_search"
                | "starts_with"
                | "ends_with"
        );

        if is_stdlib_method {
            return Self::needs_stdlib_ref(
                method,
                arg,
                usize_variables,
                current_function_params,
                borrowed_iterator_vars,
                inferred_borrowed_params,
                arg_count,
                receiver_type_name,
            );
        }

        // Final fallback
        false
    }

    /// Determine if we should add .clone() to this argument
    #[allow(clippy::too_many_arguments)]
    pub fn should_add_clone(
        arg: &Expression,
        arg_str: &str,
        method: &str,
        param_idx: usize,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
        borrowed_iterator_vars: &HashSet<String>,
        current_function_params: &[Parameter],
        inferred_borrowed_params: &HashSet<String>,
        current_function_return_type: &Option<Type>,
    ) -> bool {
        // TDD FIX: METHOD CALL EXPRESSIONS NEVER NEED .clone() (CHECKED FIRST!)
        // Method calls like input.is_key_down(Key::W) return owned values (often Copy types)
        // They never need .clone(), EVEN IF the method signature says the parameter should be owned.
        // Example: paddle.update(delta, input.is_key_down(Key::W), input.is_key_down(Key::S))
        // should NOT become paddle.update(delta, input.is_key_down(Key::W).clone(), ...)
        // This check MUST come before any signature checks to prevent unnecessary clones.
        if matches!(arg, Expression::MethodCall { .. }) {
            return false;
        }

        // THE WINDJAMMER WAY: Auto-clone borrowed iterator vars when pushing to Vec<T>
        //
        // When we have:
        //   for item in self.items { new_vec.push(item) }
        //
        // The compiler adds & automatically, making it:
        //   for item in &self.items { ... }
        //
        // So `item` is &T, but Vec::push() expects T (owned).
        // We need to automatically insert .clone() in this case.
        //
        // EXCEPTION: If the function returns Vec<&T>, don't clone!
        // Example: fn get_quests(&self) -> Vec<&Quest>
        // In this case, we want to push &Quest, not Quest.

        // Check if arg is a borrowed iterator variable
        if let Expression::Identifier { name, .. } = arg {
            if borrowed_iterator_vars.contains(name) && !arg_str.ends_with(".clone()") {
                // For push(), check if we're building a Vec<&T>
                if method == "push" {
                    // Check if the function returns Vec<&T>
                    if let Some(Type::Vec(inner_type)) = current_function_return_type {
                        // Check if the Vec's element type is a reference
                        if matches!(**inner_type, Type::Reference(_) | Type::MutableReference(_)) {
                            // Function returns Vec<&T>, so don't clone
                            return false;
                        }
                    }

                    // Not returning Vec<&T>, so clone is needed
                    return true;
                }

                // For other methods, check the signature
                if let Some(sig) = method_signature {
                    let sig_param_idx = if sig.has_self_receiver {
                        param_idx + 1
                    } else {
                        param_idx
                    };
                    if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                        if matches!(ownership, OwnershipMode::Owned) {
                            return true;
                        }
                    }
                }
            }
        }

        // TDD FIX: Check if method expects borrowed value from borrowed struct field
        // Pattern: borrowed_struct.owned_field passed to method expecting &T
        // Example: ingredient.item_id where ingredient: &Ingredient, item_id: String
        //          passed to has_item(item_id: &String)
        // Solution: Pass &borrowed_struct.owned_field (NO clone needed)
        // Wrong: &ingredient.item_id.clone() (wasteful - creates String then borrows it)
        // Right: &ingredient.item_id (just borrow the field)
        if let Some(sig) = method_signature {
            let sig_param_idx = if sig.has_self_receiver {
                param_idx + 1
            } else {
                param_idx
            };
            if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                // If method expects Borrowed, don't clone - should_add_ref will add &
                if matches!(ownership, OwnershipMode::Borrowed) {
                    return false; // Will be borrowed, no clone needed
                }

                // Method expects Owned - check if we need to clone
                if matches!(ownership, OwnershipMode::Owned) {
                    if let Expression::FieldAccess { object, .. } = arg {
                        if let Expression::Identifier { name, .. } = &**object {
                            let is_explicitly_borrowed = current_function_params.iter().any(|p| {
                                &p.name == name && matches!(p.ownership, OwnershipHint::Ref)
                            });
                            let is_inferred_borrowed = inferred_borrowed_params.contains(name);
                            if (is_explicitly_borrowed || is_inferred_borrowed)
                                && !arg_str.ends_with(".clone()")
                            {
                                // WINDJAMMER PHILOSOPHY: Don't clone Copy types.
                                // Even when moving out of a borrow, Copy types are implicitly copied.
                                // Checking `is_copy_type` prevents noise like self.mouse_x.clone()
                                if !Self::is_copy_type(
                                    arg,
                                    &HashSet::new(), // no usize_variables in this context
                                    current_function_params,
                                ) {
                                    // Also check if the method parameter type itself is Copy
                                    // (f32, i32, bool etc. don't need cloning regardless of source)
                                    let param_is_copy =
                                        sig.param_types.get(sig_param_idx).is_some_and(|t| {
                                            crate::codegen::rust::type_analysis::is_copy_type(t)
                                        });
                                    if !param_is_copy {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Determine if we should add .to_string() to this string literal
    pub fn should_add_to_string(
        param_idx: usize,
        method: &str,
        method_signature: &Option<crate::analyzer::FunctionSignature>,
    ) -> bool {
        // Check stdlib methods FIRST - these have well-known signatures that must be respected
        // even if we have a different signature in the registry (might be a user-defined method
        // with the same name)
        let is_stdlib_method = matches!(
            method,
            "push"
                | "insert"
                | "draw_text"
                | "set_title"
                | "set_text"
                | "set_label"
                | "log"
                | "print"
        );

        if is_stdlib_method {
            // Known stdlib methods that expect owned String
            // Vec<String>::push(String) - param 0 is owned
            // HashMap<String, V>::insert(String, V) - param 0 (key) is owned
            //
            // THE WINDJAMMER WAY: For external crate methods, we don't have signatures.
            // Add heuristics for common method patterns that typically take owned strings.
            return match (method, param_idx) {
                ("push", 0) => true,   // Vec<String>::push(item: String)
                ("insert", 0) => true, // HashMap<String, V>::insert(key: String, ...)
                // UI/Game framework methods that typically take owned String for display text
                ("draw_text", 0) => true, // RenderContext::draw_text(text: String, ...)
                ("set_title", 0) => true, // Window::set_title(title: String)
                ("set_text", 0) => true,  // Label::set_text(text: String)
                ("set_label", 0) => true, // Button::set_label(label: String)
                ("log", 0) => true,       // Logger::log(message: String)
                ("print", 0) => true,     // Custom print(message: String)
                _ => false,
            };
        }

        // Check signature for non-stdlib methods
        if let Some(sig) = method_signature {
            let sig_param_idx = if sig.has_self_receiver {
                param_idx + 1
            } else {
                param_idx
            };
            if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                // Convert if parameter expects owned String
                return matches!(ownership, OwnershipMode::Owned);
            }
        }

        // TDD FIX: Heuristic fallback for methods without signatures
        // Common patterns that typically expect owned String for string-like params
        // - add_*: add_ingredient, add_item, add_member, add_choice, etc.
        // - set_*: set_name, set_description, set_value, etc.
        // - new: Constructor pattern often stores owned strings
        // For first parameter (param_idx 0), these usually expect owned String
        if param_idx == 0 {
            if method.starts_with("add_")
                || method.starts_with("set_")
                || method == "new"
                || method.ends_with("_new")
            {
                return true;
            }
        }

        // Final fallback
        false
    }

    /// Determine if we should add .cloned() for Option<&T> -> Option<T>
    pub fn should_add_cloned(method: &str, _return_type: &Option<Type>) -> bool {
        super::stdlib_method_traits::is_map_key_method(method)
            || matches!(method, "first" | "last")
    }

    /// Check if expression represents a Copy type
    pub fn is_copy_type(
        arg: &Expression,
        usize_variables: &HashSet<String>,
        current_function_params: &[Parameter],
    ) -> bool {
        match arg {
            Expression::Identifier { name, .. } => {
                // Check if it's a known usize variable
                if usize_variables.contains(name) {
                    return true;
                }

                // Heuristics for Copy type variable names
                // IMPORTANT: Only use heuristics for clearly numeric types
                // DO NOT assume "entity" is Copy - Entity structs are usually NOT Copy!
                if name.contains("usize") || name.contains("index") {
                    return true;
                }

                if matches!(name.as_str(), "i" | "j" | "k" | "idx" | "pos" | "position") {
                    return true;
                }

                // Check if parameter has a Copy type (integers, floats, bool, char)
                if current_function_params.iter().any(|p| {
                    if &p.name == name {
                        if let Type::Custom(t) = &p.type_ {
                            return matches!(
                                t.as_str(),
                                "i8" | "i16"
                                    | "i32"
                                    | "i64"
                                    | "i128"
                                    | "isize"
                                    | "u8"
                                    | "u16"
                                    | "u32"
                                    | "u64"
                                    | "u128"
                                    | "usize"
                                    | "f32"
                                    | "f64"
                                    | "bool"
                                    | "char"
                            );
                        }
                    }
                    false
                }) {
                    return true;
                }

                false
            }
            Expression::FieldAccess { field, .. } => {
                // Field access like entity.id or (*entity_ref).id
                // Heuristics for Copy type field names
                matches!(
                    field.as_str(),
                    "id" | "idx"
                        | "index"
                        | "count"
                        | "size"
                        | "len"
                        | "width"
                        | "height"
                        | "x"
                        | "y"
                        | "z"
                        | "w"
                        | "r"
                        | "g"
                        | "b"
                        | "a"
                )
            }
            _ => false,
        }
    }

    /// Check if a Type annotation is a Copy type
    /// Copy types in Rust: integers, floats, bool, char, and some small tuples
    /// Public wrapper for is_copy_type_annotation
    /// Used by the Call expression handler in generator.rs
    pub fn is_copy_type_annotation_pub(type_: &Type) -> bool {
        Self::is_copy_type_annotation(type_)
    }

    fn is_copy_type_annotation(type_: &Type) -> bool {
        match type_ {
            Type::Custom(name) => {
                // Primitive Copy types
                matches!(
                    name.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "isize"
                        | "usize"
                        | "f32"
                        | "f64"
                        | "bool"
                        | "char"
                        | "int" // Windjammer's int type maps to i64
                )
            }
            // References are also Copy, but we don't add & to them anyway
            Type::Reference(_) | Type::MutableReference(_) => true,
            // Most other types are not Copy by default
            _ => false,
        }
    }

    /// Check if this method call needs & based on stdlib patterns
    fn needs_stdlib_ref(
        method: &str,
        arg: &Expression,
        usize_variables: &HashSet<String>,
        current_function_params: &[Parameter],
        borrowed_iterator_vars: &HashSet<String>,
        inferred_borrowed_params: &HashSet<String>,
        arg_count: usize,
        receiver_type_name: Option<&str>,
    ) -> bool {
        // Check if argument is already a reference (parameter or iterator variable)
        let arg_is_already_borrowed = if let Expression::Identifier { name, .. } = arg {
            // Check if it's a reference parameter
            let is_ref_param = current_function_params.iter().any(|p| {
                &p.name == name && matches!(p.ownership, OwnershipHint::Ref | OwnershipHint::Mut)
            });
            // Check if it's from a borrowed iterator (.keys(), .iter(), etc.)
            let is_borrowed_iter_var = borrowed_iterator_vars.contains(name);
            // `str` and inferred-borrowed `string` parameters are emitted as `&str` — never prefix `&`
            // again for HashSet::contains / String::contains (would be `&&str`, E0277).
            let is_rust_str_param = current_function_params.iter().any(|p| {
                p.name == *name
                    && (matches!(&p.type_, Type::Custom(s) if s == "str")
                        || (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                            && inferred_borrowed_params.contains(name)))
            });

            is_ref_param || is_borrowed_iter_var || is_rust_str_param
        } else {
            false
        };

        if arg_is_already_borrowed {
            return false;
        }

        // TDD FIX: Multi-argument methods are NOT stdlib collection methods
        // HashMap::get, HashMap::remove, Vec::remove, etc. all take exactly 1 argument.
        // If a method named "get" or "remove" takes 2+ arguments, it's a user-defined method
        // (e.g., Heightmap::get(x, z)) and we should NOT add & based on stdlib assumptions.
        if arg_count > 1 && super::stdlib_method_traits::is_map_key_method(method) {
            return false;
        }

        // TDD FIX: HashMap/BTreeMap methods that expect &K
        // HashMap methods like contains_key(&K), get(&K) ALWAYS need &, even for Copy types.
        // Vec methods like get(usize), remove(usize) take by value.
        if super::stdlib_method_traits::is_map_key_method(method) {
            let is_known_map = receiver_type_name.is_some_and(|n| {
                let base = n.split('<').next().unwrap_or(n);
                matches!(base, "HashMap" | "BTreeMap" | "IndexMap")
            });
            let is_known_vec = receiver_type_name
                .is_some_and(|n| n.split('<').next().unwrap_or(n) == "Vec");




            // When receiver type is KNOWN, use it definitively
            if is_known_vec {
                return false; // Vec methods take index by value
            }
            if is_known_map {
                // HashMap key methods ALWAYS need &, even for Copy types
                // But check for already-borrowed params first (would create &&)
                if let Expression::Identifier { name, .. } = arg {
                    let is_already_ref = current_function_params.iter().any(|p| {
                        p.name == *name
                            && (matches!(&p.type_, Type::Custom(s) if s == "str")
                                || matches!(&p.type_, Type::Reference(_) | Type::MutableReference(_))
                                || (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                    && inferred_borrowed_params.contains(name)))
                    });
                    if is_already_ref {
                        return false;
                    }
                }
                return true; // HashMap always needs &key
            }

            // Receiver type UNKNOWN — fall back to heuristics
            if super::stdlib_method_traits::is_map_key_method(method)
                && Self::is_copy_type(arg, usize_variables, current_function_params)
            {
                let arg_name = if let Expression::Identifier { name, .. } = arg {
                    Some(name.as_str())
                } else {
                    None
                };

                let looks_like_hashmap_key = arg_name.is_some_and(|name| {
                    name == "id"
                        || name == "key"
                        || name == "entity"
                        || name.ends_with("_id")
                        || name.ends_with("_key")
                });

                if looks_like_hashmap_key {
                    if let Some(name) = arg_name {
                        let is_already_map_key_ref = current_function_params.iter().any(|p| {
                            p.name == *name
                                && (matches!(&p.type_, Type::Custom(s) if s == "str")
                                    || (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                        && inferred_borrowed_params.contains(name)))
                        });
                        if is_already_map_key_ref {
                            return false;
                        }
                    }
                    return true;
                }

                return false; // Copy type, unknown receiver — assume Vec index
            }

            // Cast expressions with unknown receiver — assume Vec index
            if super::stdlib_method_traits::is_map_key_method(method) {
                if let Expression::Cast { type_, .. } = arg {
                    if Self::is_copy_type_annotation(type_) {
                        return false;
                    }
                }
            }

            return true; // Non-Copy key, assume HashMap
        }

        // General Copy type check (for non-HashMap methods)
        // Copy types generally don't need & (passed by value)
        if Self::is_copy_type(arg, usize_variables, current_function_params) {
            return false;
        }

        // Vec/slice methods that expect &T (not usize index)
        if matches!(method, "contains" | "binary_search") {
            return true;
        }

        // String methods that expect &str
        if matches!(method, "contains" | "starts_with" | "ends_with") {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_type_detection() {
        let usize_vars = HashSet::new();
        let params = vec![];

        // Test usize variable detection
        let expr = Expression::Identifier {
            name: "sparse_idx_usize".to_string(),
            location: Default::default(),
        };
        assert!(
            MethodCallAnalyzer::is_copy_type(&expr, &usize_vars, &params),
            "should detect usize variable"
        );

        // Test that "entity" is NOT assumed to be Copy
        // Entity structs are usually NOT Copy!
        let expr = Expression::Identifier {
            name: "entity".to_string(),
            location: Default::default(),
        };
        assert!(
            !MethodCallAnalyzer::is_copy_type(&expr, &usize_vars, &params),
            "should NOT assume entity is Copy"
        );

        // Test index variable detection
        let expr = Expression::Identifier {
            name: "index".to_string(),
            location: Default::default(),
        };
        assert!(
            MethodCallAnalyzer::is_copy_type(&expr, &usize_vars, &params),
            "should detect index variable"
        );
    }
}

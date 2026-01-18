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
                value: Literal::Int(_),
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

        // BORROWED ITERATOR VARIABLES: Variables from borrowed iterators (.keys(), .values(), .iter())
        // are already borrowed (e.g., &String from .keys()). Don't add another &.
        // Example: for key in map.keys() { map.get(key) }  // key is already &String
        if let Expression::Identifier { name, .. } = arg {
            if borrowed_iterator_vars.contains(name) {
                return false;
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
        // This is because:
        // 1. The cast result (usize) is a Copy type
        // 2. Adding & to the operand creates invalid syntax: `&i64 as usize` (can't cast &i64 to usize)
        if let Expression::Cast { type_, .. } = arg {
            if Self::is_copy_type_annotation(type_) {
                return false;
            }
        }

        // BUGFIX: Check method signature FIRST if available!
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
                // Signature is available - use it!
                return matches!(ownership, OwnershipMode::Borrowed);
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

        // Check if method expects owned value and arg is a borrowed field
        if let Some(sig) = method_signature {
            let sig_param_idx = if sig.has_self_receiver {
                param_idx + 1
            } else {
                param_idx
            };
            if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
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
                                return true;
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

        // Final fallback
        false
    }

    /// Determine if we should add .cloned() for Option<&T> -> Option<T>
    pub fn should_add_cloned(method: &str, _return_type: &Option<Type>) -> bool {
        // Methods that return Option<&T> from collections
        matches!(method, "get" | "first" | "last")
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
    ) -> bool {
        // Check if argument is already a reference (parameter or iterator variable)
        let arg_is_already_borrowed = if let Expression::Identifier { name, .. } = arg {
            // Check if it's a reference parameter
            let is_ref_param = current_function_params.iter().any(|p| {
                &p.name == name && matches!(p.ownership, OwnershipHint::Ref | OwnershipHint::Mut)
            });
            // Check if it's from a borrowed iterator (.keys(), .iter(), etc.)
            let is_borrowed_iter_var = borrowed_iterator_vars.contains(name);

            is_ref_param || is_borrowed_iter_var
        } else {
            false
        };

        if arg_is_already_borrowed {
            return false;
        }

        // TDD FIX: HashMap/BTreeMap methods that expect &K
        // IMPORTANT: Check HashMap methods BEFORE the general Copy type check!
        // HashMap methods like contains_key(&K), get(&K) ALWAYS need &, even for Copy types.
        // Example: HashMap<i64, String>.contains_key(&user_id) where user_id: i64
        // BUT: Vec.remove(index) takes usize by value, not &usize!
        if matches!(method, "remove" | "get" | "contains_key" | "get_mut") {
            // Special case: Vec.remove(index) where index is usize (Copy type)
            // Vec.remove wants owned usize, not &usize
            if method == "remove"
                && Self::is_copy_type(arg, usize_variables, current_function_params)
            {
                return false; // Don't add & for Vec.remove(usize_index)
            }
            // For all other cases (HashMap methods), add &
            // This includes Copy types like i64, u32, etc.
            return true; // Add & for HashMap.get(&key), HashMap.contains_key(&key)
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

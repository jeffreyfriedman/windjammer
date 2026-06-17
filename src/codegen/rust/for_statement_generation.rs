//! For loop statement generation
//!
//! Handles code generation for for loops including:
//! - Iterator-based loops (for x in collection)
//! - Range loops (for i in 0..10)
//! - Mutable reference tracking for borrowed iterators
//! - Pattern matching in loop variables

use crate::parser::*;

use super::{pattern_analysis, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a for loop statement
    pub(in crate::codegen::rust) fn generate_for_statement(
        &mut self,
        pattern: &Pattern<'ast>,
        iterable: &'ast Expression<'ast>,
        body: &[&'ast Statement<'ast>],
        location: &crate::parser::ast::SourceLocation,
    ) -> String {
        if let Some(simd_block) =
            super::simd_transform::try_emit_simd_for_loop(self, pattern, iterable, body)
        {
            return simd_block;
        }

        let mut output = self.indent();
        output.push_str("for ");

        let pattern_str = self.pattern_to_rust(pattern);
        let loop_var = pattern_analysis::extract_pattern_identifier(pattern);

        // TDD FIX: Check if ANY binding in the pattern is mutated (not just simple identifier)
        // For tuple patterns like (id, val), extract ALL bindings and check each one
        let mut all_pattern_bindings = std::collections::HashSet::new();
        self.extract_pattern_bindings(pattern, &mut all_pattern_bindings);

        let needs_mut = if let Some(var) = loop_var.as_ref() {
            // Simple identifier pattern: check if it's mutated
            self.loop_body_modifies_variable(body, var)
                || self.loop_body_calls_mut_dispatch_method(iterable, body, var)
        } else {
            // Tuple or complex pattern: check if ANY binding is mutated
            all_pattern_bindings.iter().any(|var| {
                self.loop_body_modifies_variable(body, var)
                    || self.loop_body_calls_mut_dispatch_method(iterable, body, var)
            })
        };

        let is_self_field_on_mut_self = self.inferred_mut_borrowed_params.contains("self")
            && matches!(
                iterable,
                Expression::FieldAccess { object, .. }
                    if matches!(&**object, Expression::Identifier { name, .. } if name == "self")
            );

        let mut needs_borrow = self.should_borrow_for_iteration(iterable)
            || self.self_field_iterable_needs_borrow(iterable, body);
        if is_self_field_on_mut_self {
            if needs_mut {
                // `for mut x in self.field` on &mut self: borrow mutably, never move the field.
                needs_borrow = true;
            } else if Self::variable_used_in_statements(body, "self") {
                // Body also uses `self` — clone for by-value iteration (E0505).
            } else {
                needs_borrow = true;
            }
        }
        let needs_mut_borrow = needs_mut && needs_borrow;

        let iterable_already_mut_ref = matches!(
            iterable,
            Expression::Unary {
                op: UnaryOp::MutRef,
                ..
            }
        );
        if needs_mut && !needs_mut_borrow && !iterable_already_mut_ref {
            output.push_str("mut ");
        }

        let is_unused_loop_var = location
            .as_ref()
            .is_some_and(|loc| self.unused_let_bindings.contains(&(loc.line, loc.column)));
        let display_pattern = if is_unused_loop_var {
            format!("_{}", pattern_str)
        } else {
            pattern_str
        };
        output.push_str(&display_pattern);
        output.push_str(" in ");

        let mut is_borrowed_iterator = needs_borrow || self.is_iterating_over_borrowed(iterable);

        if needs_mut_borrow {
            output.push_str("&mut ");
        } else if needs_borrow {
            output.push('&');
        }

        let iterable_to_generate = if let Expression::Unary {
            op: crate::parser::UnaryOp::Ref,
            operand,
            ..
        } = iterable
        {
            if let Expression::Identifier { name, .. } = &**operand {
                if self.inferred_borrowed_params.contains(name) {
                    operand
                } else {
                    iterable
                }
            } else {
                iterable
            }
        } else {
            iterable
        };

        // Suppress auto-clone on the iterable: for-loops iterate by reference
        // when `&` is prepended, so cloning a Vec<Box<dyn Trait>> or Vec<T>
        // is unnecessary and fails when T doesn't implement Clone.
        let prev_field_access = self.in_field_access_object;
        self.in_field_access_object = true;
        let mut iter_expr = self.generate_expression(iterable_to_generate);
        self.in_field_access_object = prev_field_access;

        // `for pass in self.field` on `&mut self` when the body calls `self.update_*()`:
        // cannot use `&self.field` (borrow conflict) or bare `self.field` (partial move).
        // Clone the collection for by-value iteration instead.
        let needs_owned_self_field_clone = self.inferred_mut_borrowed_params.contains("self")
            && self.codegen_expression_traces_to_self(iterable)
            && Self::variable_used_in_statements(body, "self")
            && !needs_borrow
            && !needs_mut_borrow
            && !iter_expr.ends_with(".clone()");
        if needs_owned_self_field_clone {
            iter_expr = format!("{}.clone()", iter_expr);
            // Owned collection snapshot — iterate by value, not `&self.field`.
            is_borrowed_iterator = false;
        }
        output.push_str(&iter_expr);
        output.push_str(" {\n");

        self.indent_level += 1;

        // TDD FIX: Track bound variables in tuple patterns for explicit deref fix.
        // IMPORTANT: When the iterable uses .enumerate(), the index variable (first
        // tuple element) is always `usize` (owned, Copy) — NOT a reference.
        // Only the value variable inherits the borrowed status from the iterator.
        let mut borrowed_bindings_added: Vec<String> = Vec::new();
        if is_borrowed_iterator {
            let enumerate_index_var = Self::extract_enumerate_index_var(iterable, pattern);
            let mut all_bindings = std::collections::HashSet::new();
            self.extract_pattern_bindings(pattern, &mut all_bindings);
            for var in all_bindings {
                if Some(&var) == enumerate_index_var.as_ref() {
                    continue;
                }
                self.borrowed_iterator_vars.insert(var.clone());
                borrowed_bindings_added.push(var.clone());
                if needs_mut_borrow {
                    self.mut_borrowed_iterator_vars.insert(var);
                }
            }
        }

        let is_owned_string_iterator = !is_borrowed_iterator;
        if is_owned_string_iterator {
            if let Some(var) = &loop_var {
                self.owned_string_iterator_vars.insert(var.clone());
            }
        }

        if let Some(var) = &loop_var {
            if let Expression::Range { end, .. } = iterable {
                if self.expression_produces_usize(end) {
                    self.usize_variables.insert(var.clone());
                }
            }
        }

        // TDD FIX: Track types for ALL bound variables (simple and tuple patterns)
        if let Some(iterable_type) = self.infer_expression_type(iterable) {
            if let Some(elem_type) = Self::extract_iterator_element_type(&iterable_type) {
                match pattern {
                    Pattern::Identifier(var) => {
                        self.local_var_types.insert(var.clone(), elem_type);
                    }
                    Pattern::Tuple(patterns) => {
                        // elem_type should be Tuple with matching arity
                        if let Type::Tuple(tuple_types) = &elem_type {
                            for (pat, ty) in patterns.iter().zip(tuple_types.iter()) {
                                if let Pattern::Identifier(var) = pat {
                                    self.local_var_types.insert(var.clone(), ty.clone());
                                }
                            }
                        }
                    }
                    _ => {
                        // For other patterns, use the old loop_var approach
                        if let Some(var) = &loop_var {
                            self.local_var_types.insert(var.clone(), elem_type);
                        }
                    }
                }
            }
        }

        let saved_body = self.current_function_body.clone();
        let saved_idx = self.current_statement_idx;
        let saved_local_idx = self.current_block_local_idx;
        self.current_function_body = body.to_vec();
        for (i, stmt) in body.iter().enumerate() {
            self.current_statement_idx = self.auto_clone_counter;
            self.current_block_local_idx = i;
            self.auto_clone_counter += 1;
            output.push_str(&self.generate_statement(stmt));
        }
        self.current_function_body = saved_body;
        self.current_statement_idx = saved_idx;
        self.current_block_local_idx = saved_local_idx;

        if is_borrowed_iterator {
            for var in &borrowed_bindings_added {
                self.borrowed_iterator_vars.remove(var);
                self.mut_borrowed_iterator_vars.remove(var);
            }
        }
        if is_owned_string_iterator {
            if let Some(var) = &loop_var {
                self.owned_string_iterator_vars.remove(var);
            }
        }
        // Clean up local_var_types for all pattern bindings, not just simple loop_var
        for var in &borrowed_bindings_added {
            self.local_var_types.remove(var);
        }
        if let Some(var) = &loop_var {
            self.local_var_types.remove(var);
        }

        self.indent_level -= 1;

        output.push_str(&self.indent());
        output.push_str("}\n");
        output
    }

    /// If the iterable is `.enumerate()` and the pattern is a tuple, return
    /// the first binding name (the index variable) which is always `usize`.
    fn extract_enumerate_index_var(
        iterable: &Expression<'ast>,
        pattern: &Pattern<'ast>,
    ) -> Option<String> {
        let is_enumerate = matches!(
            iterable,
            Expression::MethodCall { method, .. } if method == "enumerate"
        );
        if !is_enumerate {
            return None;
        }
        if let Pattern::Tuple(elements) = pattern {
            if let Some(Pattern::Identifier(name)) = elements.first() {
                return Some(name.clone());
            }
        }
        None
    }
}

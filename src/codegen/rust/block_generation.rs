//! Block expression generation
//!
//! Handles generation of:
//! - Block expressions ({ ... })
//! - Unsafe blocks
//! - Match expression optimization (single-statement match -> match expression)
//! - Implicit returns in blocks

use crate::parser::{Expression, Literal, Pattern, Statement};

use super::{
    arm_string_analysis, pattern_analysis, string_analysis, string_utilities, CodeGenerator,
};

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_block_expr(
        &mut self,
        stmts: &[&Statement<'ast>],
        is_unsafe: bool,
    ) -> String {
        let old_in_unsafe = self.in_unsafe_block;
        if is_unsafe {
            self.in_unsafe_block = true;
        }
        let block_open = if is_unsafe { "unsafe {\n" } else { "{\n" };

        // Special case: if the block contains only a match statement, generate it as a match expression
        // BUT: Skip this optimization when the match is an if-let pattern (2 arms, last is wildcard with empty body)
        // In that case, fall through to normal block generation which will generate `if let` via Statement::Match handler
        if stmts.len() == 1 {
            if let Statement::Match { value, arms, .. } = &stmts[0] {
                // Check if this is an if-let pattern that should be generated as `if let`
                let is_if_let_pattern = arms.len() == 2
                    && matches!(arms[1].pattern, Pattern::Wildcard)
                    && arms[1].guard.is_none()
                    && matches!(arms[1].body, Expression::Block { statements, .. } if statements.is_empty());

                if is_if_let_pattern {
                    // Fall through to normal block generation — generate_statement will emit `if let`
                    let mut output = String::from(block_open);
                    self.indent_level += 1;
                    for stmt in stmts {
                        output.push_str(&self.generate_statement(stmt));
                    }
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                    self.in_unsafe_block = old_in_unsafe;
                    return output;
                }

                let mut output = String::from("match ");

                // Check if any arm has a string literal pattern
                // BUT: Don't add .as_str() if the match value is a tuple
                let has_string_literal = arms
                    .iter()
                    .any(|arm| pattern_analysis::pattern_has_string_literal(&arm.pattern));

                let is_tuple_match = arms
                    .iter()
                    .any(|arm| matches!(arm.pattern, Pattern::Tuple(_)));

                // CRITICAL: Check if matching on self.field to avoid partial move
                let needs_clone_for_match = self.match_needs_clone_for_self_field(value, arms);

                let value_str = self.generate_expression(value);

                // E0507 fix: when matching on a field of a borrowed
                // parameter, add & prefix to prevent move-out errors.
                let scrutinee_needs_ref = {
                    let root = self.root_identifier_of_field_or_index_chain(value);
                    if let Some(root_name) = root {
                        let has_enum_binding = arms.iter().any(|arm| {
                            matches!(
                                &arm.pattern,
                                Pattern::EnumVariant(_, binding)
                                    if !matches!(binding, crate::parser::EnumPatternBinding::None)
                            )
                        });
                        has_enum_binding
                            && (self.inferred_borrowed_params.contains(root_name)
                                || self.inferred_mut_borrowed_params.contains(root_name))
                    } else {
                        false
                    }
                };

                if has_string_literal && !is_tuple_match {
                    if !value_str.ends_with(".as_str()") {
                        let is_already_str_ref = self.inferred_borrowed_params.contains(&value_str)
                            || self.current_function_params.iter().any(|p| {
                                p.name == value_str
                                    && (matches!(p.type_, crate::parser::Type::String)
                                        || matches!(p.type_, crate::parser::Type::Custom(ref n) if n == "str" || n == "string" || n == "&str"))
                            });
                        if is_already_str_ref {
                            output.push_str(&value_str);
                        } else {
                            output.push_str(&format!("{}.as_str()", value_str));
                        }
                    } else {
                        output.push_str(&value_str);
                    }
                } else if scrutinee_needs_ref && !value_str.ends_with(".clone()") {
                    output.push_str(&format!("&{}", value_str));
                } else if needs_clone_for_match && !value_str.ends_with(".clone()") {
                    output.push_str(&format!("{}.clone()", value_str));
                } else {
                    output.push_str(&value_str);
                }

                output.push_str(" {\n");

                self.indent_level += 1;

                // WINDJAMMER PHILOSOPHY: Detect if any arm returns String and convert all arms
                let needs_string_conversion_from_type =
                    string_utilities::return_type_expects_owned_string(
                        &self.current_function_return_type,
                    ) || arms.iter().any(|arm| {
                        string_analysis::expression_produces_string(arm.body)
                            || arm_string_analysis::arm_returns_converted_string(arm.body)
                    });

                // Set context flag BEFORE generating arms
                let old_in_match_arm = self.in_match_arm_needing_string;
                if needs_string_conversion_from_type {
                    self.in_match_arm_needing_string = true;
                }

                // Generate all arms with the flag set
                let mut arm_strings: Vec<(String, bool)> = Vec::with_capacity(arms.len());
                let match_binds_refs_flag = scrutinee_needs_ref
                    || self.match_expression_binds_refs(value)
                    || self.expression_type_contains_reference(value);

                for arm in arms.iter() {
                    // When the scrutinee has a & prefix (or clones from a
                    // borrowed param), enum struct bindings become references.
                    // Track them so for-loops iterating over these bindings
                    // correctly identify the loop variable as borrowed.
                    let mut added_borrowed: Vec<String> = Vec::new();
                    if match_binds_refs_flag {
                        let mut bound_vars = std::collections::HashSet::new();
                        self.extract_pattern_bindings(&arm.pattern, &mut bound_vars);
                        for var in &bound_vars {
                            self.borrowed_iterator_vars.insert(var.clone());
                            added_borrowed.push(var.clone());
                        }
                    }
                    // Also try infer_match_bound_types for richer type info
                    let match_bound_type_entries =
                        self.infer_match_bound_types(value, &arm.pattern);
                    for (var_name, var_type) in &match_bound_type_entries {
                        self.local_var_types
                            .insert(var_name.clone(), var_type.clone());
                    }

                    let body_str = self.generate_expression(arm.body);

                    for (var_name, _) in &match_bound_type_entries {
                        self.local_var_types.remove(var_name);
                    }
                    for var in &added_borrowed {
                        self.borrowed_iterator_vars.remove(var);
                    }

                    let is_string_literal = matches!(
                        &arm.body,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    );
                    arm_strings.push((body_str, is_string_literal));
                }

                // Restore flag
                self.in_match_arm_needing_string = old_in_match_arm;

                // For direct string literals, we still need to apply .to_string()
                let any_arm_produces_string = needs_string_conversion_from_type;

                for (arm, (arm_str, is_string_literal)) in arms.iter().zip(arm_strings.iter()) {
                    output.push_str(&self.indent());
                    output.push_str(&self.generate_pattern(&arm.pattern));

                    // Add guard if present
                    if let Some(guard) = &arm.guard {
                        output.push_str(" if ");
                        output.push_str(&self.generate_expression(guard));
                    }

                    output.push_str(" => ");

                    let mut final_arm_str = arm_str.clone();

                    // E0308 FIX: When scrutinee yields reference bindings
                    // (e.g., match &self.field, or method returning Option<&T>),
                    // simple binding returns (Some(x) => x) produce &T, but other arms
                    // may produce owned T. Clone/deref the binding to fix the mismatch.
                    let scrutinee_type_has_ref = self.expression_type_contains_reference(value);
                    let match_binds_refs = scrutinee_needs_ref
                        || self.match_expression_binds_refs(value)
                        || scrutinee_type_has_ref;
                    if match_binds_refs && !final_arm_str.ends_with(".clone()") {
                        let mut bound_vars = std::collections::HashSet::new();
                        self.extract_pattern_bindings(&arm.pattern, &mut bound_vars);
                        let binding_name: Option<&str> =
                            if let Expression::Identifier { name, .. } = arm.body {
                                Some(name)
                            } else if let Expression::Block { statements, .. } = arm.body {
                                if let Some(Statement::Expression { expr, .. }) = statements.last()
                                {
                                    if statements.len() == 1 {
                                        if let Expression::Identifier { name, .. } = expr {
                                            Some(name)
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                        if let Some(name) = binding_name {
                            if bound_vars.contains(name) {
                                let bound_type = self
                                    .infer_match_bound_types(value, &arm.pattern)
                                    .into_iter()
                                    .find(|(n, _)| n == name)
                                    .map(|(_, t)| t);
                                let is_copy =
                                    bound_type.as_ref().is_some_and(|t| self.is_type_copy(t));
                                if is_copy {
                                    if final_arm_str.trim() == name {
                                        final_arm_str = format!("*{}", name);
                                    } else {
                                        let old_str = format!("{}\n", name);
                                        let new_str = format!("*{}\n", name);
                                        final_arm_str = final_arm_str.replacen(&old_str, &new_str, 1);
                                    }
                                } else {
                                    if final_arm_str.trim() == name {
                                        final_arm_str = format!("{}.clone()", name);
                                    } else {
                                        let old_str = format!("{}\n", name);
                                        let new_str = format!("{}.clone()\n", name);
                                        final_arm_str = final_arm_str.replacen(&old_str, &new_str, 1);
                                    }
                                }
                            }
                        }
                    }

                    // Auto-convert string literals to String when other arms return String
                    if any_arm_produces_string
                        && *is_string_literal
                        && !final_arm_str.ends_with(".to_string()")
                    {
                        output.push_str(&format!("{}.to_string()", final_arm_str));
                    } else {
                        output.push_str(&final_arm_str);
                    }
                    output.push_str(",\n");
                }
                self.indent_level -= 1;

                output.push_str(&self.indent());
                output.push('}');
                self.in_unsafe_block = old_in_unsafe;
                return output;
            }
        }

        // Regular block - must handle last expression correctly
        let mut output = String::from(block_open);
        self.indent_level += 1;

        // Unsafe blocks are always value-producing (e.g., `if unsafe { call() } { ... }`),
        // so reset in_void_block to allow implicit returns.
        let saved_void_block = self.in_void_block;
        if is_unsafe {
            self.in_void_block = false;
        }

        // Save and restore auto_clone_counter so that Expression::Block
        // uses a local counter scope, matching the analysis behavior in
        // collect_usages_from_expression which creates block_counter = idx + 1.
        let saved_auto_clone = self.auto_clone_counter;

        let len = stmts.len();
        for (i, stmt) in stmts.iter().enumerate() {
            self.current_statement_idx = self.auto_clone_counter;
            self.auto_clone_counter += 1;

            let is_last = i == len - 1;
            if is_last
                && !self.in_void_block
                && matches!(
                    stmt,
                    Statement::Expression { .. }
                        | Statement::Thread { .. }
                        | Statement::Async { .. }
                )
            {
                // Last statement is an expression, thread/async block - generate as implicit return
                match stmt {
                    Statement::Expression { expr, .. } => {
                        output.push_str(&self.indent());
                        let mut expr_str = self.generate_expression(expr);

                        // If in a match arm needing string conversion, convert string literals
                        if self.in_match_arm_needing_string {
                            let is_string_literal = matches!(
                                expr,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            );
                            if is_string_literal && !expr_str.ends_with(".to_string()") {
                                expr_str = format!("{}.to_string()", expr_str);
                            }
                        }

                        output.push_str(&expr_str);

                        // TDD FIX: In statement-context matches, add semicolons to all statements
                        if self.in_statement_match {
                            output.push_str(";\n");
                        } else {
                            output.push('\n');
                        }
                    }
                    Statement::Thread { body, .. } => {
                        output.push_str(&self.indent());
                        output.push_str("std::thread::spawn(move || {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    Statement::Async { body, .. } => {
                        output.push_str(&self.indent());
                        output.push_str("tokio::spawn(async move {\n");
                        self.indent_level += 1;
                        for stmt in body {
                            output.push_str(&self.generate_statement(stmt));
                        }
                        self.indent_level -= 1;
                        output.push_str(&self.indent());
                        output.push_str("})\n");
                    }
                    _ => unreachable!(),
                }
            } else if !is_last {
                let old_expr_ctx = self.in_expression_context;
                self.in_expression_context = false;
                output.push_str(&self.generate_statement(stmt));
                self.in_expression_context = old_expr_ctx;
            } else {
                output.push_str(&self.generate_statement(stmt));
            }
        }

        self.auto_clone_counter = saved_auto_clone;

        self.indent_level -= 1;
        output.push_str(&self.indent());
        output.push('}');
        self.in_void_block = saved_void_block;
        self.in_unsafe_block = old_in_unsafe;
        output
    }
}

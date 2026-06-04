//! Match statement generation
//!
//! Handles code generation for match expressions including:
//! - Pattern matching with exhaustiveness
//! - Guard clauses
//! - Arm generation with proper scoping
//! - Reference vs owned scrutinee handling
//! - Option pattern special casing

use crate::parser::*;

use super::{pattern_analysis, string_analysis, string_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a match statement
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn generate_match_statement(
        &mut self,
        value: &'ast Expression<'ast>,
        arms: &[crate::parser::MatchArm<'ast>],
    ) -> String {
        use super::arm_string_analysis;

        // TDD FIX: Optimize boolean match expressions to matches! macro
        if arms.len() == 2 && arms[0].guard.is_none() && arms[1].guard.is_none() {
            let arm0_is_true = matches!(
                arms[0].body,
                Expression::Literal {
                    value: Literal::Bool(true),
                    ..
                }
            );
            let arm0_is_false = matches!(
                arms[0].body,
                Expression::Literal {
                    value: Literal::Bool(false),
                    ..
                }
            );
            let arm1_is_true = matches!(
                arms[1].body,
                Expression::Literal {
                    value: Literal::Bool(true),
                    ..
                }
            );
            let arm1_is_false = matches!(
                arms[1].body,
                Expression::Literal {
                    value: Literal::Bool(false),
                    ..
                }
            );

            if arm0_is_true && arm1_is_false {
                let value_str = self.generate_expression(value);
                let pattern_str = self.generate_pattern(&arms[0].pattern);
                let mut output = self.indent();
                output.push_str(&format!("matches!({}, {})\n", value_str, pattern_str));
                return output;
            }

            if arm0_is_false && arm1_is_true {
                let value_str = self.generate_expression(value);
                let pattern_str = self.generate_pattern(&arms[0].pattern);
                let mut output = self.indent();
                output.push_str(&format!("!matches!({}, {})\n", value_str, pattern_str));
                return output;
            }
        }

        // TDD FIX: Detect `if let` pattern and generate `if let` instead of `match`
        // Guards require full `match` syntax (if-let doesn't support guards in Rust)
        if arms.len() == 2
            && matches!(arms[1].pattern, Pattern::Wildcard)
            && arms[1].guard.is_none()
            && arms[0].guard.is_none()
        {
            let wildcard_body_is_empty = if let Expression::Block { statements, .. } = arms[1].body
            {
                statements.is_empty()
            } else {
                false
            };

            let wildcard_body_stmts: Option<&[&Statement]> =
                if let Expression::Block { statements, .. } = arms[1].body {
                    if statements.is_empty() {
                        None
                    } else {
                        Some(statements)
                    }
                } else {
                    None
                };

            let match_binds_refs_early_check = self.match_expression_binds_refs(value)
                || self.expression_type_contains_reference(value);
            let needs_borrow_break_check = match_binds_refs_early_check
                && self.match_scrutinee_is_self_method_call(value)
                && self.match_arms_mutate_self(arms);

            if !needs_borrow_break_check
                && (wildcard_body_is_empty || wildcard_body_stmts.is_some())
            {
                let value_str = if let Expression::MethodCall {
                    object,
                    method,
                    arguments,
                    ..
                } = value
                {
                    if super::rust_stdlib_annotations::is_strip_redundant(method)
                        && arguments.is_empty()
                        && self.expression_produces_str_ref(object)
                    {
                        self.generate_expression(object)
                    } else {
                        self.generate_expression(value)
                    }
                } else {
                    self.generate_expression(value)
                };
                // E0507 fix: if let Some(x) / EnumVar(x) = borrowed.field must use & / &mut
                let scrutinee_ref_prefix = if matches!(
                    &arms[0].pattern,
                    Pattern::EnumVariant(_, binding)
                        if !matches!(binding, crate::parser::EnumPatternBinding::None)
                ) {
                    let is_some = matches!(
                        &arms[0].pattern,
                        Pattern::EnumVariant(name, _) if name == "Some" || name.ends_with("::Some")
                    );
                    if is_some {
                        self.effective_option_scrutinee_ref_prefix(value, Some(&arms[0]))
                    } else {
                        self.option_scrutinee_ref_prefix(value)
                    }
                } else {
                    ""
                };
                let value_str = if scrutinee_ref_prefix.is_empty() {
                    value_str
                } else if scrutinee_ref_prefix == "&mut "
                    && value_str.starts_with('&')
                    && !value_str.starts_with("&mut")
                {
                    format!("&mut {}", &value_str[1..])
                } else {
                    let base = if value_str.ends_with(".clone()") {
                        value_str[..value_str.len() - 8].to_string()
                    } else {
                        value_str
                    };
                    format!("{}{}", scrutinee_ref_prefix, base)
                };
                let main_arm = &arms[0];

                let mut bound_vars = std::collections::HashSet::new();
                self.extract_pattern_bindings(&main_arm.pattern, &mut bound_vars);

                let added_borrowed: Vec<String> = if match_binds_refs_early_check {
                    bound_vars.iter().cloned().collect()
                } else {
                    Vec::new()
                };
                for var in &added_borrowed {
                    self.borrowed_iterator_vars.insert(var.clone());
                }

                self.local_variable_scopes.push(bound_vars);

                let mut match_bound_type_entries: Vec<(String, Type)> =
                    self.infer_match_bound_types(value, &main_arm.pattern);
                // The codegen prepends & or &mut to the scrutinee, but
                // `infer_match_bound_types` only sees the AST expression
                // (without the ref prefix).  Wrap the inferred binding types
                // so downstream `let x = binding` can trigger `.clone()`.
                if scrutinee_ref_prefix == "&mut " {
                    for entry in &mut match_bound_type_entries {
                        if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                            entry.1 = Type::MutableReference(Box::new(entry.1.clone()));
                        }
                    }
                } else if scrutinee_ref_prefix == "& " || scrutinee_ref_prefix == "&" {
                    for entry in &mut match_bound_type_entries {
                        if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                            entry.1 = Type::Reference(Box::new(entry.1.clone()));
                        }
                    }
                }
                for (var_name, var_type) in &match_bound_type_entries {
                    self.local_var_types
                        .insert(var_name.clone(), var_type.clone());
                }

                // Upgrade pattern bindings to `mut` when the body mutates them.
                // Only use ref mut when the scrutinee is &mut (mutable borrow).
                // When &self (immutable), ref mut is invalid.
                let scrutinee_is_mut_ref = scrutinee_ref_prefix.contains("mut");
                let upgraded_pattern = if let Expression::Block { statements, .. } = main_arm.body {
                    self.upgrade_pattern_mut_bindings(
                        &main_arm.pattern,
                        statements.as_slice(),
                        scrutinee_is_mut_ref,
                    )
                } else {
                    main_arm.pattern.clone()
                };

                let mut output = self.indent();
                output.push_str("if let ");
                output.push_str(&self.generate_pattern(&upgraded_pattern));

                if let Some(guard) = &main_arm.guard {
                    output.push_str(" if ");
                    output.push_str(&self.generate_expression(guard));
                }

                output.push_str(" = ");
                output.push_str(&value_str);
                output.push_str(" {\n");

                let has_else = wildcard_body_stmts.is_some();
                let old_in_func_body = self.in_function_body;
                let old_in_void_block = self.in_void_block;
                if !has_else {
                    self.in_function_body = false;
                    self.in_void_block = true;
                }

                self.indent_level += 1;
                if let Expression::Block { statements, .. } = main_arm.body {
                    // Check the last statement for simple binding return needing deref
                    if match_binds_refs_early_check {
                        if let Some(last_stmt) = statements.last() {
                            if let crate::parser::Statement::Expression { expr, .. } = last_stmt {
                                if let Expression::Identifier { name, .. } = expr {
                                    if added_borrowed.contains(name) {
                                        let binding_type = match_bound_type_entries
                                            .iter()
                                            .find(|(n, _)| n == name)
                                            .map(|(_, t)| t);
                                        let is_copy =
                                            binding_type.is_some_and(|t| self.is_type_copy(t));
                                        // Generate all but last, then the derefed last
                                        let all_but_last = &statements[..statements.len() - 1];
                                        output.push_str(&self.generate_block(all_but_last));
                                        output.push_str(&self.indent());
                                        let expr_str = self.generate_expression(expr);
                                        if is_copy {
                                            output.push_str(&format!("*{}\n", expr_str));
                                        } else {
                                            output.push_str(&format!("{}.clone()\n", expr_str));
                                        }
                                        self.indent_level -= 1;
                                        self.in_void_block = old_in_void_block;

                                        output.push_str(&self.indent());
                                        output.push('}');

                                        if let Some(else_stmts) = wildcard_body_stmts {
                                            output.push_str(" else {\n");
                                            self.indent_level += 1;
                                            output.push_str(&self.generate_block(else_stmts));
                                            self.indent_level -= 1;
                                            output.push_str(&self.indent());
                                            output.push('}');
                                        }
                                        self.in_function_body = old_in_func_body;
                                        for var in &added_borrowed {
                                            self.borrowed_iterator_vars.remove(var);
                                        }
                                        self.local_variable_scopes.pop();
                                        for (var_name, _) in &match_bound_type_entries {
                                            self.local_var_types.remove(var_name);
                                        }
                                        return output;
                                    }
                                }
                            }
                        }
                    }
                    output.push_str(&self.generate_block(statements));
                } else {
                    // Simple expression body — check for deref
                    let mut body_str = self.generate_expression(main_arm.body);
                    if match_binds_refs_early_check {
                        if let Expression::Identifier { name, .. } = main_arm.body {
                            if added_borrowed.contains(name) {
                                let binding_type = match_bound_type_entries
                                    .iter()
                                    .find(|(n, _)| n == name)
                                    .map(|(_, t)| t);
                                let is_copy = binding_type.is_some_and(|t| self.is_type_copy(t));
                                if is_copy {
                                    body_str = format!("*{}", body_str);
                                } else {
                                    body_str = format!("{}.clone()", body_str);
                                }
                            }
                        }
                    }
                    output.push_str(&self.indent());
                    output.push_str(&body_str);
                    output.push_str(";\n");
                }
                self.indent_level -= 1;
                self.in_void_block = old_in_void_block;

                output.push_str(&self.indent());
                output.push('}');

                if let Some(else_stmts) = wildcard_body_stmts {
                    output.push_str(" else {\n");
                    self.indent_level += 1;
                    output.push_str(&self.generate_block(else_stmts));
                    self.indent_level -= 1;
                    output.push_str(&self.indent());
                    output.push('}');
                }

                self.in_function_body = old_in_func_body;

                output.push('\n');

                self.local_variable_scopes.pop();
                for (var_name, _) in &match_bound_type_entries {
                    self.local_var_types.remove(var_name);
                }
                for var in &added_borrowed {
                    self.borrowed_iterator_vars.remove(var);
                }

                return output;
            }
        }

        let has_string_literal = arms
            .iter()
            .any(|arm| pattern_analysis::pattern_has_string_literal(&arm.pattern));

        let is_tuple_match = arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Tuple(_)));

        let value_str = if let Expression::MethodCall {
            object,
            method,
            arguments,
            ..
        } = value
        {
            if super::rust_stdlib_annotations::is_strip_redundant(method)
                && arguments.is_empty()
                && self.expression_produces_str_ref(object)
            {
                self.generate_expression(object)
            } else {
                self.generate_expression(value)
            }
        } else if let Expression::Call {
            function,
            arguments,
            ..
        } = value
        {
            if let Expression::FieldAccess { object, field, .. } = &**function {
                if super::rust_stdlib_annotations::is_strip_redundant(field)
                    && arguments.is_empty()
                    && self.expression_produces_str_ref(object)
                {
                    self.generate_expression(object)
                } else {
                    self.generate_expression(value)
                }
            } else {
                self.generate_expression(value)
            }
        } else {
            self.generate_expression(value)
        };

        // E0507 fix: match on Option behind a borrow needs & / &mut scrutinee
        let some_arm = arms.iter().find(|arm| {
            matches!(&arm.pattern, Pattern::EnumVariant(name, _) if name == "Some" || name.ends_with("::Some"))
        });
        let match_scrutinee_ref_prefix: &str;
        let value_str = if let Some(arm) = some_arm {
            let p = self.effective_option_scrutinee_ref_prefix(value, Some(arm));
            match_scrutinee_ref_prefix = p;
            if p.is_empty() {
                value_str
            } else if p == "&mut " && value_str.starts_with('&') && !value_str.starts_with("&mut") {
                format!("&mut {}", &value_str[1..])
            } else {
                let base = if value_str.ends_with(".clone()") {
                    value_str[..value_str.len() - 8].to_string()
                } else {
                    value_str
                };
                format!("{}{}", p, base)
            }
        } else {
            match_scrutinee_ref_prefix = "";
            value_str
        };

        // E0507 fix (generalized): non-Option enum patterns with bindings
        // behind a borrowed parameter also need & prefix.
        let value_str = if some_arm.is_none() {
            let has_non_option_binding = arms.iter().any(|arm| {
                matches!(
                    &arm.pattern,
                    Pattern::EnumVariant(name, binding)
                        if !matches!(binding, crate::parser::EnumPatternBinding::None)
                           && name != "Some" && !name.ends_with("::Some")
                           && name != "None" && !name.ends_with("::None")
                )
            });
            if has_non_option_binding {
                let root = self.root_identifier_of_field_or_index_chain(value);
                if let Some(root_name) = root {
                    let already_owned = value_str.ends_with(".clone()");
                    if already_owned {
                        value_str
                    } else if self.inferred_mut_borrowed_params.contains(root_name) {
                        format!("&mut {}", value_str)
                    } else if self.inferred_borrowed_params.contains(root_name) {
                        // Check if the underlying value type (not the reference) is Copy.
                        // For `e: &E` the expression type is `&E` (Copy since refs are Copy),
                        // but we need to know if `E` itself is Copy to decide the prefix:
                        //   - Copy inner type: use `*e` (deref to get owned Copy value)
                        //   - Non-Copy inner type: use `e` (let match ergonomics handle it)
                        let inner_type_is_copy = if root_name == "self" {
                            self.current_struct_name
                                .as_ref()
                                .is_some_and(|sn| self.is_type_copy(&Type::Custom(sn.clone())))
                        } else {
                            self.infer_expression_type(value)
                                .map(|t| match &t {
                                    Type::Reference(inner) | Type::MutableReference(inner) => {
                                        self.is_type_copy(inner)
                                    }
                                    _ => self.is_type_copy(&t),
                                })
                                .unwrap_or(false)
                        };
                        if inner_type_is_copy {
                            if matches!(value, Expression::FieldAccess { .. }) {
                                value_str
                            } else {
                                format!("*{}", value_str)
                            }
                        } else if root_name == "self" {
                            // self is already &Self — no extra & needed.
                            value_str
                        } else {
                            // Non-Copy type behind shared ref: need & prefix to prevent
                            // moving out of the borrow. Match ergonomics will auto-ref bindings.
                            format!("&{}", value_str)
                        }
                    } else {
                        value_str
                    }
                } else {
                    value_str
                }
            } else {
                value_str
            }
        } else {
            value_str
        };

        let match_binds_refs_early = self.match_expression_binds_refs(value);
        let needs_borrow_break = match_binds_refs_early
            && self.match_scrutinee_is_self_method_call(value)
            && self.match_arms_mutate_self(arms);

        let mut output = self.indent();

        if needs_borrow_break {
            output.push_str(&format!(
                "let __match_borrow_break = {}.map(|__v| __v.to_owned());\n",
                value_str
            ));
            output.push_str(&self.indent());
            output.push_str("match __match_borrow_break.as_ref()");
        } else {
            output.push_str("match ");
            if has_string_literal && !is_tuple_match {
                let scrutinee = crate::codegen::rust::string_utilities::maybe_append_as_str_for_match(
                    &value_str,
                    &self.inferred_borrowed_params,
                    &self.current_function_params,
                );
                output.push_str(&scrutinee);
            } else {
                output.push_str(&value_str);
            }
        }

        output.push_str(" {\n");

        self.indent_level += 1;

        let match_binds_refs = self.match_expression_binds_refs(value);

        let needs_string_conversion =
            string_utilities::return_type_expects_owned_string(&self.current_function_return_type)
                || arms.iter().any(|arm| {
                    string_analysis::expression_produces_string(arm.body)
                        || arm_string_analysis::arm_returns_converted_string(arm.body)
                });

        let old_in_statement_match = self.in_statement_match;
        let match_is_statement = self.current_function_return_type.is_none();
        if match_is_statement {
            self.in_statement_match = true;
        }

        // If any arm has an empty body (returns ()), treat all arms as void
        // to prevent type mismatches between () and non-() return values.
        let has_void_arm = arms.iter().any(
            |arm| matches!(arm.body, Expression::Block { statements, .. } if statements.is_empty()),
        );

        let scrutinee_type_has_ref = self.expression_type_contains_reference(value);
        // When the scrutinee has been dereferenced (`*self`, `*e`, etc.) for a Copy type,
        // the match operates on an owned value and pattern bindings are owned — NOT refs.
        // Generalized from the original `value_str == "*self"` to handle all Copy params.
        let owned_bindings_from_copy_deref = if let Some(deref_name) = value_str.strip_prefix('*') {
            if deref_name == "self" {
                self.current_struct_name
                    .as_ref()
                    .is_some_and(|sn| self.is_type_copy(&Type::Custom(sn.clone())))
            } else if let Some(ty) = self.infer_expression_type(value) {
                let inner = match &ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                self.is_type_copy(inner) && !Self::type_contains_reference(inner)
            } else {
                false
            }
        } else {
            false
        };

        // When codegen prepends `&` / `&mut` on the scrutinee (`match &node.children`),
        // pattern bindings are reference types even if `match_expression_binds_refs` is false.
        let scrutinee_prefix_binds_refs = !match_scrutinee_ref_prefix.is_empty();

        for arm in arms {
            // Upgrade pattern bindings to `mut` when the arm body mutates them
            let body_stmts: &[&Statement<'ast>] =
                if let Expression::Block { statements, .. } = arm.body {
                    statements.as_slice()
                } else {
                    &[]
                };
            let upgraded_pattern = self.upgrade_pattern_mut_bindings(
                &arm.pattern,
                body_stmts,
                match_scrutinee_ref_prefix.contains("mut"),
            );

            output.push_str(&self.indent());
            output.push_str(&self.generate_pattern(&upgraded_pattern));

            if let Some(guard) = &arm.guard {
                output.push_str(" if ");
                output.push_str(&self.generate_expression(guard));
            }

            output.push_str(" => ");

            let mut bound_vars = std::collections::HashSet::new();
            self.extract_pattern_bindings(&arm.pattern, &mut bound_vars);

            // TDD FIX for E0614: Track match arm bindings as OWNED values
            // Match arm bindings extract owned values from enums, NOT references
            // This prevents incorrectly adding * to Copy types like i32 in comparisons
            for var in &bound_vars {
                self.match_arm_bindings.insert(var.clone());
            }

            let added_borrowed: Vec<String> =
                if (match_binds_refs || scrutinee_type_has_ref || scrutinee_prefix_binds_refs)
                    && !owned_bindings_from_copy_deref
                {
                    bound_vars.iter().cloned().collect()
                } else {
                    Vec::new()
                };
            for var in &added_borrowed {
                self.borrowed_iterator_vars.insert(var.clone());
            }

            // Clone bound_vars before moving it, so we can clean up match_arm_bindings later
            let bound_vars_for_cleanup = bound_vars.clone();

            self.local_variable_scopes.push(bound_vars);

            let mut match_bound_type_entries: Vec<(String, Type)> =
                self.infer_match_bound_types(value, &arm.pattern);
            // Wrap binding types with the ref kind matching the
            // generated scrutinee prefix (see if-let equivalent above).
            if match_scrutinee_ref_prefix == "&mut " {
                for entry in &mut match_bound_type_entries {
                    if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                        entry.1 = Type::MutableReference(Box::new(entry.1.clone()));
                    }
                }
            } else if match_scrutinee_ref_prefix == "& " || match_scrutinee_ref_prefix == "&" {
                for entry in &mut match_bound_type_entries {
                    if !matches!(entry.1, Type::Reference(_) | Type::MutableReference(_)) {
                        entry.1 = Type::Reference(Box::new(entry.1.clone()));
                    }
                }
            }
            for (var_name, var_type) in &match_bound_type_entries {
                self.local_var_types
                    .insert(var_name.clone(), var_type.clone());
            }

            let old_in_match_arm = self.in_match_arm_needing_string;
            if needs_string_conversion {
                self.in_match_arm_needing_string = true;
            }

            let old_void_block = self.in_void_block;
            if has_void_arm {
                self.in_void_block = true;
            }
            let mut arm_str = self.generate_expression(arm.body);
            self.in_void_block = old_void_block;

            self.in_match_arm_needing_string = old_in_match_arm;

            if (match_binds_refs || scrutinee_type_has_ref || scrutinee_prefix_binds_refs)
                && !arm_str.ends_with(".clone()")
            {
                // Extract the binding name from either a direct identifier
                // or a block whose only/last statement is an expression identifier
                let binding_name: Option<&str> =
                    if let Expression::Identifier { name, .. } = arm.body {
                        Some(name)
                    } else if let Expression::Block { statements, .. } = arm.body {
                        if let Some(Statement::Expression { expr, .. }) = statements.last() {
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
                    };
                let is_simple_binding_return =
                    binding_name.is_some_and(|n| added_borrowed.contains(&n.to_string()));
                if is_simple_binding_return {
                    let bname = binding_name.unwrap();
                    let binding_type = match_bound_type_entries
                        .iter()
                        .find(|(n, _)| n == bname)
                        .map(|(_, t)| t);
                    let inner_type = match binding_type {
                        Some(Type::Reference(inner)) | Some(Type::MutableReference(inner)) => {
                            Some(inner.as_ref())
                        }
                        other => other,
                    };
                    let is_copy = inner_type.is_some_and(|t| self.is_type_copy(t));
                    // For block bodies, replace the binding name inside the
                    // generated string since we can't prefix the whole block.
                    let deref_expr = if is_copy {
                        format!("*{}", bname)
                    } else {
                        format!("{}.clone()", bname)
                    };
                    if let Expression::Identifier { .. } = arm.body {
                        arm_str = deref_expr;
                    } else {
                        // Block body: replace the last occurrence of the
                        // bare binding with its dereffed version
                        if let Some(pos) = arm_str.rfind(bname) {
                            let after = pos + bname.len();
                            if after >= arm_str.len()
                                || !arm_str[after..after + 1]
                                    .chars()
                                    .next()
                                    .unwrap_or(' ')
                                    .is_alphanumeric()
                            {
                                arm_str.replace_range(pos..after, &deref_expr);
                            }
                        }
                    }
                } else if let Some(rewritten) = self.rewrite_some_wrapper_for_ref_match_binding(
                    arm.body,
                    &match_bound_type_entries,
                    &added_borrowed,
                ) {
                    arm_str = rewritten;
                } else if let Some(rewritten) = self.rewrite_some_ident_arm_string(
                    arm_str.trim(),
                    &match_bound_type_entries,
                    &added_borrowed,
                ) {
                    arm_str = rewritten;
                }
            }

            self.local_variable_scopes.pop();

            for (var_name, _) in &match_bound_type_entries {
                self.local_var_types.remove(var_name);
            }

            for var in &added_borrowed {
                self.borrowed_iterator_vars.remove(var);
            }

            // TDD FIX: Clean up match arm bindings after each arm
            for var in &bound_vars_for_cleanup {
                self.match_arm_bindings.remove(var);
            }
            let is_string_literal = matches!(
                &arm.body,
                Expression::Literal {
                    value: Literal::String(_),
                    ..
                }
            );

            if needs_string_conversion
                && is_string_literal
                && !string_utilities::already_owned_string_expr(&arm_str)
            {
                arm_str = string_utilities::coerce_expr_to_owned_string(&arm_str);
            }

            output.push_str(&arm_str);
            output.push_str(",\n");
        }

        self.in_statement_match = old_in_statement_match;

        self.indent_level -= 1;

        output.push_str(&self.indent());
        output.push_str("}\n");
        output
    }

    /// `Some(x)` (or a block containing only that expression) where `x` is a match binding
    /// introduced under ref ergonomics; used to upgrade to owned `Option` for the arm value.
    fn match_arm_some_call_single_ident<'e>(body: &'e Expression<'e>) -> Option<&'e str> {
        let expr = match body {
            Expression::Block { statements, .. } => {
                if statements.len() != 1 {
                    return None;
                }
                match statements[0] {
                    Statement::Expression { expr, .. } => expr,
                    _ => return None,
                }
            }
            other => other,
        };
        let Expression::Call {
            function,
            arguments,
            ..
        } = expr
        else {
            return None;
        };
        if arguments.len() != 1 {
            return None;
        }
        let Expression::Identifier { name: fname, .. } = &**function else {
            return None;
        };
        if fname != "Some" && !fname.ends_with("::Some") {
            return None;
        }
        let (_, arg) = &arguments[0];
        let Expression::Identifier { name: inner, .. } = &**arg else {
            return None;
        };
        Some(inner.as_str())
    }

    /// When match ergonomics bind `x` as `&T` but the arm returns `Some(x)` expecting
    /// `Option<T>`, emit `Some(x.clone())` or `Some(*x)` for `Copy` `T`.
    pub(in crate::codegen::rust) fn rewrite_some_wrapper_for_ref_match_binding(
        &self,
        arm_body: &'ast Expression<'ast>,
        match_bound_type_entries: &[(String, Type)],
        added_borrowed: &[String],
    ) -> Option<String> {
        let inner = Self::match_arm_some_call_single_ident(arm_body)?;
        if !added_borrowed.iter().any(|n| n == inner) {
            return None;
        }
        let binding_type = match_bound_type_entries
            .iter()
            .find(|(n, _)| n == inner)
            .map(|(_, t)| t);
        let inner_type = binding_type.map(|bt| match bt {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        });
        let is_copy = inner_type.is_some_and(|t| self.is_type_copy(t));
        let inner_expr = if is_copy {
            format!("*{}", inner)
        } else {
            format!("{}.clone()", inner)
        };
        Some(format!("Some({})", inner_expr))
    }

    /// Fallback when `infer_match_bound_types` is empty / AST shape misses: `Some(x)` lowering
    /// is always plain text; salvage from generated Rust substring.
    pub(in crate::codegen::rust) fn rewrite_some_ident_arm_string(
        &self,
        arm_str: &str,
        match_bound_type_entries: &[(String, Type)],
        added_borrowed: &[String],
    ) -> Option<String> {
        let s = arm_str.trim();
        const PREFIX: &str = "Some(";
        if !s.starts_with(PREFIX) || !s.ends_with(')') {
            return None;
        }
        let inner = s[PREFIX.len()..s.len().saturating_sub(1)].trim();
        if !Self::looks_like_simple_binding_ident(inner)
            || !added_borrowed.iter().any(|n| n == inner)
        {
            return None;
        }
        let binding_type = match_bound_type_entries
            .iter()
            .find(|(n, _)| n == inner)
            .map(|(_, t)| t);
        let inner_ty = binding_type.map(|bt| match bt {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        });
        let is_copy = inner_ty.is_some_and(|t| self.is_type_copy(t));
        if is_copy {
            Some(format!("Some(*{})", inner))
        } else {
            Some(format!("Some({}.clone())", inner))
        }
    }

    fn looks_like_simple_binding_ident(inner: &str) -> bool {
        let mut ch = inner.chars();
        let Some(c0) = ch.next() else {
            return false;
        };
        if !(c0.is_ascii_alphabetic() || c0 == '_') {
            return false;
        }
        ch.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}

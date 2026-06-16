//! Block generation for sequences of statements
//!
//! Handles code generation for blocks including:
//! - Statement sequencing and tracking
//! - Implicit return optimization for last expressions
//! - Thread/async block handling
//! - Auto-clone tracking and optimization
//! - String literal coercion for return types
//! - Iterator variable dereferencing

use crate::parser::*;

use super::{codegen_helpers, pattern_analysis, string_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a block of statements
    pub(crate) fn generate_block(&mut self, stmts: &[&'ast Statement<'ast>]) -> String {
        let mut output = String::new();
        let len = stmts.len();
        let saved_body = self.current_function_body.clone();
        let saved_idx = self.current_statement_idx;
        let saved_local_idx = self.current_block_local_idx;
        let saved_skips = std::mem::take(&mut self.skip_block_indices);
        self.current_function_body = stmts.to_vec();
        for (i, stmt) in stmts.iter().enumerate() {
            self.current_statement_idx = self.auto_clone_counter;
            self.current_block_local_idx = i;
            self.auto_clone_counter += 1;

            // Skip statements folded into a preceding .take() or .replace()
            if self.skip_block_indices.contains(&i) {
                continue;
            }

            let is_last = i == len - 1;
            // TDD: Track if this is the last statement (used by If handler)
            self.current_is_last_statement = is_last;
            // TDD FIX: Only optimize return statements in function body (not nested blocks)
            let should_optimize_return =
                self.in_function_body && matches!(stmt, Statement::Return { .. });
            if is_last
                && !self.in_void_block
                && (should_optimize_return
                    || matches!(
                        stmt,
                        Statement::Expression { .. }
                            | Statement::Thread { .. }
                            | Statement::Async { .. }
                    ))
            {
                // Last statement is an expression, thread/async block, or return - generate as implicit return
                match stmt {
                    Statement::Expression { expr, .. } => {
                        output.push_str(&self.indent());
                        let old_coerce_lit = self.coerce_string_literals_to_owned;
                        if self.in_function_body
                            && string_utilities::return_type_expects_owned_string(
                                &self.current_function_return_type,
                            )
                        {
                            self.coerce_string_literals_to_owned = true;
                        }
                        let mut expr_str = self.generate_expression(expr);
                        self.coerce_string_literals_to_owned = old_coerce_lit;

                        // TDD FIX: Borrowed iterator vars need deref when returned as Copy types
                        // For `for (_, val) in &vec` where val: &i32, implicit return `val` needs `*val`
                        if let Expression::Identifier { name, .. } = expr {
                            if self.borrowed_iterator_vars.contains(name) {
                                let return_type_is_copy = self
                                    .current_function_return_type
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if return_type_is_copy && !expr_str.starts_with('*') {
                                    expr_str = format!("*{}", expr_str);
                                }
                            }
                        }

                        // Deref local vars with reference types when returning Copy
                        // e.g., `let (id, name) = &items[0]; id` → id is &i32, return i32 → *id
                        // Also handles &mut refs: `let x = n; x` where n: &mut i32, return i32
                        if let Expression::Identifier { .. } = expr {
                            let expects_owned = !matches!(
                                &self.current_function_return_type,
                                Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                            );
                            if expects_owned {
                                if let Some(
                                    Type::Reference(inner) | Type::MutableReference(inner),
                                ) = self.infer_expression_type(expr)
                                {
                                    if self.is_type_copy(inner.as_ref())
                                        && !expr_str.starts_with('*')
                                    {
                                        expr_str = format!("*{}", expr_str);
                                    }
                                }
                            }
                        }

                        self.coerce_return_ref_to_owned_copy(&mut expr_str, expr);

                        self.apply_owned_string_tail_coercion(&mut expr_str, expr, true);

                        // DOGFOODING FIX: Vec indexing &vec[idx] for non-Copy needs .clone() when implicit return
                        // Applies to all return types (SaveSlot, Option<String>, etc.), not just String
                        // Use parentheses: (&vec[idx]).clone() - . has higher precedence than &
                        if expr_str.starts_with("&")
                            && !expr_str.starts_with("&mut")
                            && !expr_str.ends_with(".clone()")
                        {
                            let expects_owned = !matches!(
                                &self.current_function_return_type,
                                Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                            );
                            if expects_owned {
                                if let Some(inner) = self.infer_expression_type(expr) {
                                    if !self.is_type_copy(&inner) {
                                        expr_str = format!("({}).clone()", expr_str);
                                    }
                                }
                            }
                        } else if matches!(expr, Expression::Index { .. })
                            && !expr_str.ends_with(".clone()")
                            && !expr_str.starts_with('&')
                        {
                            let expects_owned = !matches!(
                                &self.current_function_return_type,
                                Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                            );
                            if expects_owned {
                                if let Some(inner) = self.infer_expression_type(expr) {
                                    if !self.is_type_copy(&inner) {
                                        expr_str = format!("{}.clone()", expr_str);
                                    }
                                }
                            }
                        }

                        {
                            let target = match &self.current_function_return_type {
                                Some(Type::Int) => Some("int"),
                                Some(Type::Custom(name)) if name == "i64" || name == "int" => {
                                    Some("int")
                                }
                                _ => None,
                            };
                            self.maybe_cast_usize_to_int_target(&mut expr_str, expr, target);
                        }

                        let returns_option_owned = self.returns_option_owned_type();
                        if returns_option_owned
                            && self.expression_type_contains_reference(expr)
                            && !expr_str.ends_with(".cloned()")
                            && !expr_str.ends_with(".clone()")
                        {
                            if self
                                .infer_expression_type(expr)
                                .as_ref()
                                .is_some_and(Self::type_contains_mut_reference_static)
                            {
                                expr_str = format!("{}.map(|v| v.clone())", expr_str);
                            } else {
                                expr_str = format!("{}.cloned()", expr_str);
                            }
                        }

                        output.push_str(&expr_str);

                        // BUGFIX: Only add semicolon if:
                        // 1. Function returns ()
                        // 2. AND we're not in an expression context (value is not being used)
                        // This prevents adding semicolons to if-else branches when used as values
                        let returns_unit = self.current_function_return_type.is_none()
                            || matches!(self.current_function_return_type, Some(Type::Tuple(ref types)) if types.is_empty());
                        if returns_unit && !self.in_expression_context {
                            output.push(';');
                        }
                        output.push('\n');
                    }
                    Statement::Thread { body, .. } => {
                        // Generate as expression (returns JoinHandle)
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
                        // Generate as expression (returns JoinHandle)
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
                    Statement::Return { value, .. } => {
                        // TDD FIX: Convert explicit return to implicit return when last statement
                        // Avoids Clippy warning: "unneeded `return` statement"
                        if let Some(expr) = value {
                            output.push_str(&self.indent());
                            let old_coerce_lit = self.coerce_string_literals_to_owned;
                            if string_utilities::return_type_expects_owned_string(
                                &self.current_function_return_type,
                            ) {
                                self.coerce_string_literals_to_owned = true;
                            }
                            let mut expr_str = self.generate_expression(expr);
                            self.coerce_string_literals_to_owned = old_coerce_lit;

                            self.coerce_return_ref_to_owned_copy(&mut expr_str, expr);

                            if expr_str.starts_with("&")
                                && !expr_str.starts_with("&mut")
                                && !expr_str.ends_with(".clone()")
                            {
                                let expects_owned = !matches!(
                                    &self.current_function_return_type,
                                    Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                                );
                                if expects_owned {
                                    if let Some(inner) = self.infer_expression_type(expr) {
                                        if !self.is_type_copy(&inner) {
                                            expr_str = format!("({}).clone()", expr_str);
                                        }
                                    }
                                }
                            } else if matches!(expr, Expression::Index { .. })
                                && !expr_str.ends_with(".clone()")
                                && !expr_str.starts_with('&')
                            {
                                let expects_owned = !matches!(
                                    &self.current_function_return_type,
                                    Some(Type::Reference(_)) | Some(Type::MutableReference(_))
                                );
                                if expects_owned {
                                    if let Some(inner) = self.infer_expression_type(expr) {
                                        if !self.is_type_copy(&inner) {
                                            expr_str = format!("{}.clone()", expr_str);
                                        }
                                    }
                                }
                            }

                            // TDD FIX: Borrowed iterator vars need deref when returned as Copy types
                            // For `for (_, val) in &vec` where val: &i32, `return val` needs `return *val`
                            if let Expression::Identifier { name, .. } = expr {
                                if self.borrowed_iterator_vars.contains(name) {
                                    let return_type_is_copy = self
                                        .current_function_return_type
                                        .as_ref()
                                        .is_some_and(|t| self.is_type_copy(t));
                                    if return_type_is_copy && !expr_str.starts_with('*') {
                                        expr_str = format!("*{}", expr_str);
                                    }
                                }
                            }

                            self.apply_owned_string_tail_coercion(&mut expr_str, expr, true);

                            {
                                let target = match &self.current_function_return_type {
                                    Some(Type::Int) => Some("int"),
                                    Some(Type::Custom(name)) if name == "i64" || name == "int" => {
                                        Some("int")
                                    }
                                    _ => None,
                                };
                                self.maybe_cast_usize_to_int_target(&mut expr_str, expr, target);
                            }

                            let returns_option_owned = self.returns_option_owned_type();
                            if returns_option_owned
                                && self.expression_type_contains_reference(expr)
                                && !expr_str.ends_with(".cloned()")
                                && !expr_str.ends_with(".clone()")
                            {
                                if self
                                    .infer_expression_type(expr)
                                    .as_ref()
                                    .is_some_and(Self::type_contains_mut_reference_static)
                                {
                                    expr_str = format!("{}.map(|v| v.clone())", expr_str);
                                } else {
                                    expr_str = format!("{}.cloned()", expr_str);
                                }
                            }

                            output.push_str(&expr_str);
                            output.push('\n');
                        }
                        // Void return as last statement is omitted (block returns () implicitly)
                    }
                    _ => unreachable!(),
                }
            } else if !is_last {
                // TDD FIX: Non-last statements in a block ALWAYS need semicolons,
                // even when the block is used in an expression context (e.g., match arm body
                // inside `let _ = match ... { Arm => { expr1; expr2 } }`).
                // Temporarily clear in_expression_context so intermediate expression
                // statements get their semicolons.
                let old_expr_ctx = self.in_expression_context;
                self.in_expression_context = false;
                output.push_str(&self.generate_statement(stmt));
                self.in_expression_context = old_expr_ctx;
            } else {
                // Last statement of a non-Expression type (e.g., Statement::If used as block value):
                // Preserve in_expression_context so inner branches retain correct semicolon behavior
                output.push_str(&self.generate_statement(stmt));
            }
        }
        self.current_function_body = saved_body;
        self.current_statement_idx = saved_idx;
        self.current_block_local_idx = saved_local_idx;
        self.skip_block_indices = saved_skips;
        output
    }

    pub(crate) fn generate_statement(&mut self, stmt: &Statement<'ast>) -> String {
        // RECURSION GUARD: Check depth before processing statement
        if let Err(e) = self.enter_recursion("generate_statement") {
            eprintln!("{}", e);
            return format!("/* {} */", e);
        }

        // Record source mapping if location info is available
        if let Some(location) = codegen_helpers::get_statement_location(stmt) {
            self.record_mapping(&location);
        }

        let result = self.generate_statement_impl(stmt);
        self.exit_recursion();
        result
    }

    pub(in crate::codegen::rust) fn generate_block_expr(
        &mut self,
        stmts: &[&'ast Statement<'ast>],
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
                    let scrutinee = string_utilities::maybe_append_as_str_for_match(
                        &value_str,
                        &self.inferred_borrowed_params,
                        &self.current_function_params,
                    );
                    output.push_str(&scrutinee);
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
                let needs_string_conversion_from_type = self.coerce_string_literals_to_owned
                    || string_utilities::return_type_expects_owned_string(
                        &self.current_function_return_type,
                    )
                    || arms
                        .iter()
                        .any(|arm| string_utilities::match_arm_needs_string_ascription(arm.body));

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
                        let match_bound_entries = self.infer_match_bound_types(value, &arm.pattern);
                        let added_borrowed: Vec<String> = bound_vars.iter().cloned().collect();
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
                                let bound_type = match_bound_entries
                                    .iter()
                                    .find(|(n, _)| n == name)
                                    .map(|(_, t)| t);
                                let is_copy =
                                    bound_type.as_ref().is_some_and(|t| self.is_copy_pointee(t));
                                if is_copy {
                                    if final_arm_str.trim() == name {
                                        final_arm_str = format!("*{}", name);
                                    } else {
                                        let old_str = format!("{}\n", name);
                                        let new_str = format!("*{}\n", name);
                                        final_arm_str =
                                            final_arm_str.replacen(&old_str, &new_str, 1);
                                    }
                                } else {
                                    if final_arm_str.trim() == name {
                                        final_arm_str = format!("{}.clone()", name);
                                    } else {
                                        let old_str = format!("{}\n", name);
                                        let new_str = format!("{}.clone()\n", name);
                                        final_arm_str =
                                            final_arm_str.replacen(&old_str, &new_str, 1);
                                    }
                                }
                            }
                        } else if let Some(rewritten) = self
                            .rewrite_some_wrapper_for_ref_match_binding(
                                arm.body,
                                &match_bound_entries,
                                &added_borrowed,
                            )
                        {
                            final_arm_str = rewritten;
                        } else if let Some(rewritten) = self.rewrite_some_ident_arm_string(
                            final_arm_str.trim(),
                            &match_bound_entries,
                            &added_borrowed,
                        ) {
                            final_arm_str = rewritten;
                        }
                    }

                    // Auto-convert string literals to String when other arms return String
                    if any_arm_produces_string
                        && *is_string_literal
                        && !crate::codegen::rust::string_utilities::already_owned_string_expr(
                            &final_arm_str,
                        )
                    {
                        output.push_str(
                            &crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                                &final_arm_str,
                            ),
                        );
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
                            if is_string_literal
                                && !crate::codegen::rust::string_utilities::already_owned_string_expr(&expr_str)
                            {
                                expr_str = crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(
                                    &expr_str,
                                );
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

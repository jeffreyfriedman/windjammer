//! Let statement generation
//!
//! Handles code generation for variable declarations including:
//! - Simple let bindings
//! - Tuple destructuring
//! - Type inference from expressions
//! - Mutability inference
//! - Unused binding suppression with _ prefix

use crate::parser::*;

use super::{string_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a let statement
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn generate_let_statement(
        &mut self,
        pattern: &Pattern<'ast>,
        mutable: bool,
        type_: &Option<Type>,
        value: &'ast Expression<'ast>,
        location: &Option<crate::source_map::Location>,
    ) -> String {
        let mut output = self.indent();
        output.push_str("let ");

        // Check if we need &mut for index access on borrowed fields
        // e.g., let enemy = self.enemies[i] should be let enemy = &mut self.enemies[i]
        let needs_mut_ref = self.should_mut_borrow_index_access(value);

        // Extract variable name for optimizations (only works for simple identifiers)
        let var_name = match pattern {
            Pattern::Identifier(name) => Some(name.as_str()),
            _ => None,
        };

        // TDD FIX (E0596): When `let x = self.field.get(key)` and downstream code
        // mutates the value obtained from x (via match/if-let), upgrade get→get_mut.
        if let Some(vn) = var_name {
            if super::self_analysis::is_self_field_get_call(value)
                && self.let_binding_value_is_mutated_downstream(vn)
            {
                self.upgrade_get_to_get_mut = true;
            }
        }

        // Mutability: explicit via `let mut`, or auto-inferred when the
        // variable is later used with a &mut self method call.
        let auto_needs_mut =
            !mutable && !needs_mut_ref && var_name.is_some_and(|v| self.variable_needs_mut(v));
        if needs_mut_ref {
            // Don't add mut keyword, but we'll add &mut to the value
        } else if mutable || auto_needs_mut {
            output.push_str("mut ");
        }

        // TDD FIX: Prefix unused let bindings with `_` to suppress warnings
        let is_unused_binding = location
            .as_ref()
            .is_some_and(|loc| self.unused_let_bindings.contains(&(loc.line, loc.column)));

        // Generate pattern (could be simple name or tuple)
        let pattern_str = if is_unused_binding {
            match pattern {
                Pattern::Identifier(name) => format!("_{}", name),
                other => self.generate_pattern(other),
            }
        } else {
            self.generate_pattern(pattern)
        };
        output.push_str(&pattern_str);

        // LOCAL VARIABLE TRACKING: Add this variable to the current scope
        // This enables proper shadowing of field names
        if let Some(name) = var_name {
            if let Some(current_scope) = self.local_variable_scopes.last_mut() {
                current_scope.insert(name.to_string());
            }
        } else if matches!(pattern, Pattern::Tuple(_)) {
            let mut bound = std::collections::HashSet::new();
            self.extract_pattern_bindings(pattern, &mut bound);
            if let Some(current_scope) = self.local_variable_scopes.last_mut() {
                for n in bound {
                    current_scope.insert(n);
                }
            }
        }

        // LOCAL VARIABLE TYPE TRACKING: Infer type from value expression or annotation
        // This enables qualified method signature lookup (e.g., stack.remove() → Stack::remove)
        if let Some(name) = var_name {
            let inferred_type: Option<Type> = if let Some(type_) = type_ {
                // Explicit type annotation: let x: Foo = ...
                Some((*type_).clone())
            } else {
                // Infer from value expression
                match value {
                    Expression::StructLiteral {
                        name: struct_name, ..
                    } => Some(Type::Custom(struct_name.to_string())),
                    // Literal types: let x = 25 → i32, let y = 3.14 → f32, let b = true → bool
                    Expression::Literal {
                        value: crate::parser::Literal::Int(_),
                        ..
                    } => Some(Type::Int),
                    Expression::Literal {
                        value: crate::parser::Literal::Float(_),
                        ..
                    } => Some(Type::Float),
                    Expression::Literal {
                        value: crate::parser::Literal::Bool(_),
                        ..
                    } => Some(Type::Bool),
                    Expression::Literal {
                        value: crate::parser::Literal::String(_),
                        ..
                    } => Some(Type::String),
                    Expression::Call { function, .. } => {
                        // Type::method() pattern (e.g., Foo::new())
                        if let Expression::FieldAccess { object, field, .. } = *function {
                            if let Expression::Identifier {
                                name: type_name, ..
                            } = *object
                            {
                                if type_name.chars().next().is_some_and(|c| c.is_uppercase())
                                    && (field == "new"
                                        || field.starts_with("from_")
                                        || field.starts_with("with_")
                                        || field == "default")
                                {
                                    // E0282: For Vec::new() / HashSet::new(), infer element type.
                                    // Priority: 1) return type  2) forward-scan .push()/.insert()
                                    if (type_name == "Vec" || type_name == "HashSet")
                                        && field == "new"
                                    {
                                        let elem_from_return =
                                            match &self.current_function_return_type {
                                                Some(Type::Vec(inner)) if type_name == "Vec" => {
                                                    Some(inner.as_ref().clone())
                                                }
                                                Some(Type::Parameterized(base, args))
                                                    if base == type_name && !args.is_empty() =>
                                                {
                                                    Some(args[0].clone())
                                                }
                                                _ => None,
                                            };
                                        let elem_from_push = if elem_from_return.is_none() {
                                            var_name.and_then(|vn| {
                                                self.infer_collection_element_type_from_usage(vn)
                                            })
                                        } else {
                                            None
                                        };
                                        let elem_type = elem_from_return.or(elem_from_push);
                                        if let Some(inner) = elem_type {
                                            if type_name == "Vec" {
                                                Some(Type::Vec(Box::new(inner)))
                                            } else {
                                                Some(Type::Parameterized(
                                                    type_name.to_string(),
                                                    vec![inner],
                                                ))
                                            }
                                        } else {
                                            Some(Type::Custom(type_name.to_string()))
                                        }
                                    } else {
                                        Some(Type::Custom(type_name.to_string()))
                                    }
                                } else {
                                    // Not a constructor — look up return type from signature registry
                                    // e.g., MathHelper::fade(x) → return type is f32
                                    let qualified = format!("{}::{}", type_name, field);
                                    self.signature_registry
                                        .get_signature(&qualified)
                                        .and_then(|sig| sig.return_type.clone())
                                }
                            } else {
                                None
                            }
                        } else if let Expression::Identifier { name: fn_name, .. } = *function {
                            // Handle Identifier("Vec::new") path (parser emits this form)
                            if fn_name == "Vec::new" || fn_name == "HashSet::new" {
                                let collection_name = if fn_name.starts_with("Vec") {
                                    "Vec"
                                } else {
                                    "HashSet"
                                };
                                let elem_from_return = match &self.current_function_return_type {
                                    Some(Type::Vec(inner)) if collection_name == "Vec" => {
                                        Some(inner.as_ref().clone())
                                    }
                                    Some(Type::Parameterized(base, args))
                                        if base == collection_name && !args.is_empty() =>
                                    {
                                        Some(args[0].clone())
                                    }
                                    _ => None,
                                };
                                let elem_from_push = if elem_from_return.is_none() {
                                    var_name.and_then(|vn| {
                                        self.infer_collection_element_type_from_usage(vn)
                                    })
                                } else {
                                    None
                                };
                                let elem_type = elem_from_return.or(elem_from_push);
                                if let Some(inner) = elem_type {
                                    if collection_name == "Vec" {
                                        Some(Type::Vec(Box::new(inner)))
                                    } else {
                                        Some(Type::Parameterized(
                                            collection_name.to_string(),
                                            vec![inner],
                                        ))
                                    }
                                } else {
                                    Some(Type::Custom(collection_name.to_string()))
                                }
                            } else {
                                self.infer_expression_type(value)
                            }
                        } else {
                            // Simple function call: look up in signature registry
                            self.infer_expression_type(value)
                        }
                    }
                    _ => {
                        // Fall back to general expression type inference
                        // Handles if/else, binary ops, method calls, etc.
                        self.infer_expression_type(value)
                    }
                }
            };
            if let Some(t) = inferred_type {
                self.local_var_types.insert(name.to_string(), t);
            }
        }

        // PHASE 8: Check if this variable should use SmallVec
        if let Some(name) = var_name {
            if let Some(smallvec_opt) = self.smallvec_optimizations.get(name) {
                // Use SmallVec with stack allocation
                // If there's a type annotation, extract the element type
                let elem_type = if let Some(Type::Vec(inner)) = type_ {
                    self.type_to_rust(inner)
                } else {
                    "_".to_string() // Type inference
                };
                output.push_str(&format!(
                    ": SmallVec<[{}; {}]>",
                    elem_type, smallvec_opt.stack_size
                ));
                output.push_str(" = ");

                // Generate the expression but wrap in smallvec! if it's a vec! macro
                let expr_str = self.generate_expression(value);
                if let Some(stripped) = expr_str.strip_prefix("vec!") {
                    // Replace vec! with smallvec!
                    output.push_str("smallvec!");
                    output.push_str(stripped);
                } else {
                    // For other expressions, try to convert
                    output.push_str(&expr_str);
                    output.push_str(".into()"); // Convert Vec to SmallVec
                }
            } else if let Some(t) = type_ {
                output.push_str(": ");
                output.push_str(&self.type_to_rust(t));
                output.push_str(" = ");

                let is_string_type = matches!(t, Type::String)
                    || matches!(t, Type::Custom(name) if name == "String" || name == "string");

                let old_coerce_lit = self.coerce_string_literals_to_owned;
                if is_string_type {
                    self.coerce_string_literals_to_owned = true;
                }
                // Same as other `let` RHS paths: value is used (e.g. `let x: f32 = if ...`).
                // Without this, if/else branch bodies get `expr;` and infer `()` (E0308).
                let old_ctx = self.in_expression_context;
                self.in_expression_context = true;

                let prev_assign_float = self.assignment_float_target_type.take();
                if Self::assignment_target_needs_float_codegen_context(t) {
                    self.assignment_float_target_type = Some(t.clone());
                }
                let prev_suppress_turbo = self.suppress_collection_turbofish;
                let suppress_turbofish_here = matches!(t, Type::Vec(_))
                    || matches!(
                        t,
                        Type::Parameterized(base, _)
                            if base == "HashSet" || base == "HashMap"
                    );
                if suppress_turbofish_here {
                    self.suppress_collection_turbofish = true;
                }

                let prev_collect_target = self.collect_target_type.take();
                self.collect_target_type = Some(t.clone());

                // Auto-convert &str to String if type is String
                let mut value_str = self.generate_expression(value);

                self.collect_target_type = prev_collect_target;
                self.suppress_collection_turbofish = prev_suppress_turbo;
                self.assignment_float_target_type = prev_assign_float;

                self.in_expression_context = old_ctx;
                self.coerce_string_literals_to_owned = old_coerce_lit;
                self.apply_vec_index_let_rhs_fixup(var_name, value, Some(t), &mut value_str);

                // Convert string literals OR identifiers to String when target is String
                if is_string_type && value_str != "String::new()" {
                    let should_convert = matches!(
                        value,
                        Expression::Literal {
                            value: Literal::String(s),
                            ..
                        } if !s.is_empty()
                    ) || matches!(value, Expression::Identifier { .. });
                    if should_convert && !string_utilities::already_owned_string_expr(&value_str) {
                        value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                    }
                    if let Expression::Literal {
                        value: Literal::String(s),
                        ..
                    } = value
                    {
                        if s.is_empty() {
                            value_str = "String::new()".to_string();
                        }
                    }
                }
                output.push_str(&value_str);
            } else {
                // E0282: Emit type ascription for collection types.
                // Skip when the value's type is better inferred by Rust:
                // - Method calls may return a different type than the receiver
                //   (e.g., Vec::into_iter() → IntoIter, not Vec)
                // - Macro invocations (e.g., vec![1,2,3]) produce values whose
                //   element type should be inferred from usage context, not from
                //   Windjammer's default numeric types
                let type_inferred_from_context = matches!(
                    value,
                    Expression::MethodCall { .. } | Expression::MacroInvocation { .. }
                );
                let needs_collection_ascription_sv = !type_inferred_from_context
                    && var_name.is_some_and(|vn| {
                        matches!(
                            self.local_var_types.get(vn),
                            Some(Type::Vec(_)) | Some(Type::Parameterized(_, _))
                        )
                    });
                if needs_collection_ascription_sv {
                    let vn = var_name.unwrap();
                    let ty = self.local_var_types.get(vn).unwrap().clone();
                    output.push_str(": ");
                    output.push_str(&self.type_to_rust(&ty));
                } else if string_utilities::untyped_let_rhs_needs_string_ascription(value) {
                    output.push_str(": String");
                }
                output.push_str(" = ");
                if needs_mut_ref {
                    output.push_str("&mut ");
                }

                // EXPRESSION CONTEXT: Mark that we're generating a value that will be used
                // This prevents adding semicolons to if-else branches when used in let bindings
                let old_ctx = self.in_expression_context;
                self.in_expression_context = true;

                let old_suppress = self.suppress_collection_turbofish;
                if needs_collection_ascription_sv {
                    self.suppress_collection_turbofish = true;
                }

                // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                // String literals assigned to variables should become String (not &str)
                // because they may be passed to functions expecting String later.
                // This is safe because String auto-borrows to &str when needed.
                let mut value_str = self.generate_expression(value);

                self.apply_vec_index_let_rhs_fixup(var_name, value, None, &mut value_str);
                if let Expression::Literal {
                    value: Literal::String(s),
                    ..
                } = value
                {
                    if s.is_empty() {
                        value_str = "String::new()".to_string();
                    } else if !string_utilities::already_owned_string_expr(&value_str) {
                        value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                    }
                }

                // E0507: `let x = self.field` through `&self`/`&mut self`:
                //   Option<T> behind &mut self → .take() (moves value, leaves None)
                //   other non-Copy → .clone()
                self.apply_self_field_move_fix(value, &mut value_str);

                value_str = self.let_rhs_clone_if_mut_from_non_copy_ref(
                    mutable,
                    value,
                    needs_mut_ref,
                    &value_str,
                );

                output.push_str(&value_str);

                // Restore expression context
                self.in_expression_context = old_ctx;
                self.suppress_collection_turbofish = old_suppress;
            }
        } else {
            // No SmallVec optimization for this variable
            if let Some(t) = type_ {
                output.push_str(": ");
                output.push_str(&self.type_to_rust(t));
                output.push_str(" = ");

                // EXPRESSION CONTEXT: Mark that we're generating a value
                let old_ctx = self.in_expression_context;
                self.in_expression_context = true;

                let prev_assign_float = self.assignment_float_target_type.take();
                if Self::assignment_target_needs_float_codegen_context(t) {
                    self.assignment_float_target_type = Some(t.clone());
                }
                let prev_suppress_turbo = self.suppress_collection_turbofish;
                let suppress_turbofish_here = matches!(t, Type::Vec(_))
                    || matches!(
                        t,
                        Type::Parameterized(base, _) if base == "HashSet" || base == "HashMap"
                    );
                if suppress_turbofish_here {
                    self.suppress_collection_turbofish = true;
                }

                let prev_collect_target = self.collect_target_type.take();
                self.collect_target_type = Some(t.clone());

                // Auto-convert &str to String if type is String
                let mut value_str = self.generate_expression(value);

                self.collect_target_type = prev_collect_target;
                self.suppress_collection_turbofish = prev_suppress_turbo;
                self.assignment_float_target_type = prev_assign_float;

                self.apply_vec_index_let_rhs_fixup(var_name, value, Some(t), &mut value_str);
                let is_string_type = matches!(t, Type::String)
                    || matches!(t, Type::Custom(name) if name == "String");

                // Convert string literals OR identifiers to String when target is String
                if is_string_type && value_str != "String::new()" {
                    if let Expression::Literal {
                        value: Literal::String(s),
                        ..
                    } = value
                    {
                        if s.is_empty() {
                            value_str = "String::new()".to_string();
                        } else if !string_utilities::already_owned_string_expr(&value_str) {
                            value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                        }
                    } else if matches!(value, Expression::Identifier { .. })
                        && !string_utilities::already_owned_string_expr(&value_str)
                    {
                        value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                    }
                }

                if needs_mut_ref {
                    value_str = format!("&mut {}", value_str);
                }
                output.push_str(&value_str);

                // Restore expression context
                self.in_expression_context = old_ctx;
            } else {
                // E0282: Emit type ascription for collection types inferred from
                // forward-scanned .push()/.insert() usage
                let needs_collection_ascription = var_name.is_some_and(|vn| {
                    matches!(
                        self.local_var_types.get(vn),
                        Some(Type::Vec(_)) | Some(Type::Parameterized(_, _))
                    )
                });
                if needs_collection_ascription {
                    let vn = var_name.unwrap();
                    let ty = self.local_var_types.get(vn).unwrap().clone();
                    output.push_str(": ");
                    output.push_str(&self.type_to_rust(&ty));
                } else if string_utilities::untyped_let_rhs_needs_string_ascription(value) {
                    output.push_str(": String");
                }
                output.push_str(" = ");
                if needs_mut_ref {
                    output.push_str("&mut ");
                }

                // EXPRESSION CONTEXT: Mark that we're generating a value
                let old_ctx = self.in_expression_context;
                self.in_expression_context = true;

                let old_suppress = self.suppress_collection_turbofish;
                if needs_collection_ascription {
                    self.suppress_collection_turbofish = true;
                }

                // WINDJAMMER PHILOSOPHY: Auto-convert mutable string variables
                // When a mutable variable is initialized with a string literal,
                // it should be a String (not &str) because &str can't be mutated
                let mut value_str = self.generate_expression(value);
                self.apply_vec_index_let_rhs_fixup(var_name, value, None, &mut value_str);
                if mutable
                    && matches!(
                        value,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    )
                    && !string_utilities::already_owned_string_expr(&value_str)
                {
                    value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                }

                // E0507: `let x = self.field` through `&self`/`&mut self`:
                //   Option<T> behind &mut self → .take() (moves value, leaves None)
                //   other non-Copy → .clone()
                self.apply_self_field_move_fix(value, &mut value_str);

                value_str = self.let_rhs_clone_if_mut_from_non_copy_ref(
                    mutable,
                    value,
                    needs_mut_ref,
                    &value_str,
                );

                output.push_str(&value_str);

                // Restore expression context
                self.in_expression_context = old_ctx;
                self.suppress_collection_turbofish = old_suppress;
            }
        }

        self.register_tuple_let_binding_types(pattern, value);

        output.push_str(";\n");

        // Track variables assigned from .len() as usize type
        // OR variables with explicit usize type annotation
        // This enables auto-casting in comparisons with i32
        if let Some(name) = var_name {
            let is_usize = self.expression_produces_usize(value)
                || matches!(type_, Some(Type::Custom(s)) if s == "usize");
            if is_usize {
                self.usize_variables.insert(name.to_string());
            }
        }

        output
    }

    /// Check if a let-bound variable's value (from HashMap.get()) is mutated
    /// in subsequent match/if-let statements in the current function body.
    fn let_binding_value_is_mutated_downstream(&self, var_name: &str) -> bool {
        let current_idx = self.current_block_local_idx;
        let body = &self.current_function_body;

        for stmt in body.iter().skip(current_idx + 1) {
            if self.stmt_has_match_that_mutates_get_binding(stmt, var_name) {
                return true;
            }
        }
        false
    }

    fn stmt_has_match_that_mutates_get_binding(
        &self,
        stmt: &Statement<'ast>,
        var_name: &str,
    ) -> bool {
        match stmt {
            Statement::Match { value, arms, .. } => {
                let is_scrutinee =
                    matches!(value, Expression::Identifier { name, .. } if name == var_name);
                if is_scrutinee {
                    for arm in arms.iter() {
                        if let Some(binding) =
                            super::self_analysis::extract_some_binding(&arm.pattern)
                        {
                            if self.match_body_has_mutating_call(arm.body, binding) {
                                return true;
                            }
                        }
                        if let Some(bindings) =
                            super::self_analysis::extract_tuple_some_bindings(&arm.pattern)
                        {
                            for binding in &bindings {
                                if self.match_body_has_mutating_call(arm.body, binding) {
                                    return true;
                                }
                            }
                        }
                    }
                }

                // Handle tuple scrutinee: match (a_opt, b_opt) { (Some(a), Some(b)) => ... }
                if let Expression::Tuple { .. } = *value {
                    for arm in arms.iter() {
                        if let Some(binding) =
                            super::self_analysis::find_binding_for_var_in_tuple_match(
                                value,
                                var_name,
                                &arm.pattern,
                            )
                        {
                            if self.match_body_has_mutating_call(arm.body, binding) {
                                return true;
                            }
                        }
                    }
                }

                false
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.stmt_has_match_that_mutates_get_binding(s, var_name))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_has_match_that_mutates_get_binding(s, var_name))
                    })
            }
            _ => false,
        }
    }

    fn match_body_has_mutating_call(&self, body: &Expression<'ast>, var_name: &str) -> bool {
        match body {
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.stmt_has_mutating_method_on_var(s, var_name)),
            Expression::MethodCall { object, method, .. } => {
                if matches!(&**object, Expression::Identifier { name, .. } if name == var_name) {
                    return !super::self_analysis::is_known_readonly_method_name(method);
                }
                false
            }
            _ => false,
        }
    }

    fn stmt_has_mutating_method_on_var(&self, stmt: &Statement<'ast>, var_name: &str) -> bool {
        match stmt {
            Statement::Expression { expr, .. } => {
                self.expr_has_mutating_method_on_var(expr, var_name)
            }
            Statement::Assignment { target, .. } => {
                super::self_analysis::expression_references_variable_or_field(target, var_name)
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block
                    .iter()
                    .any(|s| self.stmt_has_mutating_method_on_var(s, var_name))
                    || else_block.as_ref().is_some_and(|b| {
                        b.iter()
                            .any(|s| self.stmt_has_mutating_method_on_var(s, var_name))
                    })
            }
            Statement::While { body, .. }
            | Statement::For { body, .. }
            | Statement::Loop { body, .. } => body
                .iter()
                .any(|s| self.stmt_has_mutating_method_on_var(s, var_name)),
            Statement::Match { arms, .. } => arms
                .iter()
                .any(|arm| self.expr_has_mutating_method_on_var(arm.body, var_name)),
            Statement::Let { value, .. } => self.expr_has_mutating_method_on_var(value, var_name),
            _ => false,
        }
    }

    fn expr_has_mutating_method_on_var(&self, expr: &Expression<'ast>, var_name: &str) -> bool {
        match expr {
            Expression::MethodCall { object, method, .. } => {
                let is_var =
                    matches!(&**object, Expression::Identifier { name, .. } if name == var_name);
                let is_field_of_var =
                    if let Expression::FieldAccess { object: inner, .. } = &**object {
                        matches!(&**inner, Expression::Identifier { name, .. } if name == var_name)
                    } else {
                        false
                    };
                if (is_var || is_field_of_var)
                    && !super::self_analysis::is_known_readonly_method_name(method)
                {
                    return true;
                }
                false
            }
            Expression::Block { statements, .. } => statements
                .iter()
                .any(|s| self.stmt_has_mutating_method_on_var(s, var_name)),
            _ => false,
        }
    }
}

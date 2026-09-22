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
    /// Infer a let-binding type from its value (signature-driven), refining bare
    /// `Vec` / `HashSet` element types from return type or forward `.push`/`.insert` usage.
    fn infer_let_value_type(
        &self,
        value: &Expression<'ast>,
        var_name: Option<&str>,
    ) -> Option<Type> {
        let base = self.infer_expression_type(value)?;
        let collection_name = match &base {
            Type::Custom(n)
                if crate::type_classification::is_single_elem_iterable_base(n)
                    || crate::type_classification::is_set_type_name(n) =>
            {
                Some(n.as_str())
            }
            Type::Vec(_) => return Some(base),
            Type::Parameterized(base_name, args)
                if (crate::type_classification::is_single_elem_iterable_base(base_name)
                    || crate::type_classification::is_set_type_name(base_name))
                    && args.is_empty() =>
            {
                Some(base_name.as_str())
            }
            _ => return Some(base),
        };
        let Some(collection_name) = collection_name else {
            return Some(base);
        };

        let elem_from_return = match &self.current_function_return_type {
            Some(Type::Vec(inner))
                if crate::type_classification::is_single_elem_iterable_base(collection_name) =>
            {
                Some(inner.as_ref().clone())
            }
            Some(Type::Parameterized(b, args)) if b == collection_name && !args.is_empty() => {
                Some(args[0].clone())
            }
            _ => None,
        };
        let elem_from_push = if elem_from_return.is_none() {
            var_name.and_then(|vn| self.infer_collection_element_type_from_usage(vn))
        } else {
            None
        };
        if let Some(inner) = elem_from_return.or(elem_from_push) {
            return Some(
                if crate::type_classification::type_name_leaf(collection_name) == "Vec" {
                    Type::Vec(Box::new(inner))
                } else {
                    Type::Parameterized(collection_name.to_string(), vec![inner])
                },
            );
        }
        Some(base)
    }

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
            if super::self_analysis::is_self_field_get_call(
                value,
                &self.signature_registry,
                self.current_struct_name.as_deref(),
                Some(&self.struct_field_types),
            ) && self.let_binding_value_is_mutated_downstream(vn)
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
        } else if matches!(pattern, Pattern::Tuple(_) | Pattern::EnumVariant(_, _)) {
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
                if let Some(vn) = var_name {
                    if matches!(type_, Type::Int)
                        || matches!(type_, Type::Custom(n) if n == "int" || n == "i64")
                    {
                        self.usize_variables.remove(vn);
                        self.explicit_wj_int_annotated_locals.insert(vn.to_string());
                    }
                }
                Some((*type_).clone())
            } else {
                // Infer from value expression
                match value {
                    Expression::StructLiteral {
                        name: struct_name, ..
                    } => Some(Type::Custom(struct_name.to_string())),
                    // Unit-struct / type-path values: `let r = Renderer` (no braces).
                    // CamelCase identifiers are type constructors, not locals.
                    Expression::Identifier { name, .. }
                        if name.starts_with(|c: char| c.is_ascii_uppercase()) =>
                    {
                        Some(Type::Custom(name.to_string()))
                    }
                    // P3.329: `let mut i = clock_end` when binding/RHS is a usize index counter.
                    Expression::Identifier { name: rhs_name, .. } => {
                        if self.usize_variables.contains(name)
                            || self.usize_variables.contains(rhs_name)
                        {
                            Some(Type::Custom("usize".into()))
                        } else {
                            self.infer_expression_type(value)
                        }
                    }
                    // Literal types: untyped `let x = 25` follows enclosing return width
                    // (WDB-081 / P3.280). Bool/string returns keep WJ `int` so loop counters
                    // `i = i + 1` stay i64 (not `+= 1 as i32`). Custom/struct returns still
                    // prefer i32 for coordinate locals (`let cy = 10` in builders).
                    Expression::Literal {
                        value: crate::parser::Literal::Int(_),
                        ..
                    } => {
                        // Prepass may have marked this binding as a usize index/len counter
                        // (`while i < vec.len()` / `vec[i]`) before the let is emitted.
                        // Do not overwrite with return-inferred Int32 — that yields
                        // `let i = 0_usize` + `i += 1 as i32` / `i == 0_i32` (P3.311/P3.314).
                        if self.usize_variables.contains(name) {
                            Some(Type::Custom("usize".into()))
                        } else if let Some(ret_ty) = &self.current_function_return_type {
                            // Peel Result/Option so `-> Result<int, string>` keeps i64
                            // accumulators (P3.317), not coordinate-default Int32.
                            let hint = self.int_width_hint_from_return_type_resolved(ret_ty);
                            // P3.370: void `@test` payload sizes (`let n = 100_000`) stay WJ int.
                            if matches!(hint, Type::Int32)
                                && matches!(
                                    Self::peel_option_result_payload(ret_ty),
                                    Type::Custom(n) if n == "Unit"
                                )
                            {
                                Some(Type::Int)
                            } else {
                                Some(hint)
                            }
                        } else {
                            // Void / no return type (incl. `@test`): keep WJ int (i64).
                            // Game i32 coords use i32-returning builders or explicit `i32`.
                            Some(Type::Int)
                        }
                    }
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
                    Expression::Unary {
                        op: crate::parser::UnaryOp::Neg,
                        operand,
                        ..
                    } => self.infer_expression_type(operand),
                    Expression::Call { .. } | Expression::MethodCall { .. } => {
                        // Signature-driven via infer_expression_type (associated Type::assoc,
                        // WDB-091). Refine empty Vec/HashSet element types from usage.
                        self.infer_let_value_type(value, var_name)
                    }
                    // P3.280: `let cx = VIEWER_GRID / 2` — Binary must prefer module-const /
                    // param width (i32) over default WJ `int` from a Literal sibling.
                    Expression::Binary { left, right, op, .. }
                        if matches!(
                            op,
                            crate::parser::BinaryOp::Add
                                | crate::parser::BinaryOp::Sub
                                | crate::parser::BinaryOp::Mul
                                | crate::parser::BinaryOp::Div
                                | crate::parser::BinaryOp::Mod
                                | crate::parser::BinaryOp::BitAnd
                                | crate::parser::BinaryOp::BitOr
                                | crate::parser::BinaryOp::BitXor
                                | crate::parser::BinaryOp::Shl
                                | crate::parser::BinaryOp::Shr
                        ) =>
                    {
                        let l = self.infer_expression_type(left);
                        let r = self.infer_expression_type(right);
                        match (l, r) {
                            (Some(a), Some(b)) if a == b
                                && matches!(a, Type::Int)
                                && self.function_prefers_i32_coord_locals() =>
                            {
                                Some(Type::Int32)
                            }
                            (Some(a), Some(b)) if a != b => {
                                if self.function_prefers_i32_coord_locals()
                                    && (matches!(a, Type::Int) || matches!(b, Type::Int))
                                {
                                    Some(Type::Int32)
                                } else if matches!(a, Type::Int)
                                    && Self::assignment_target_needs_int_codegen_context(&b)
                                    && !matches!(b, Type::Int)
                                {
                                    Some(b)
                                } else if matches!(b, Type::Int)
                                    && Self::assignment_target_needs_int_codegen_context(&a)
                                    && !matches!(a, Type::Int)
                                {
                                    Some(a)
                                } else {
                                    Some(a)
                                }
                            }
                            (Some(t), _) | (_, Some(t)) => Some(t),
                            (None, None) => None,
                        }
                    }
                    Expression::Block { statements, .. } => {
                        let if_i32 = statements.last().and_then(|last_stmt| {
                            let Statement::If {
                                then_block,
                                else_block,
                                ..
                            } = last_stmt
                            else {
                                return None;
                            };
                            let then_expr = then_block.last().and_then(|s| match s {
                                Statement::Expression { expr, .. } => Some(*expr),
                                _ => None,
                            })?;
                            let else_expr = else_block.as_ref().and_then(|b| {
                                b.last().and_then(|s| match s {
                                    Statement::Expression { expr, .. } => Some(*expr),
                                    _ => None,
                                })
                            })?;
                            self.if_else_binding_should_be_i32(then_expr, else_expr)
                                .then_some(Type::Int32)
                        });
                        if_i32.or_else(|| self.infer_expression_type(value))
                    }
                    _ => {
                        // Fall back to general expression type inference
                        // Handles if/else, binary ops, method calls, etc.
                        self.infer_expression_type(value)
                    }
                }
            };
            if let Some(t) = inferred_type {
                self.local_var_types.insert(name.to_string(), t.clone());
                if matches!(t, Type::Int32) {
                    self.codegen_i32_binding_names.insert(name.to_string());
                }
            }
            if mutable
                && Self::mut_let_rhs_is_return_width_counter(value)
                && matches!(
                    self.local_var_types.get(name),
                    Some(Type::Int) | Some(Type::Int32)
                )
            {
                self.literal_init_wj_int_loop_counters
                    .insert(name.to_string());
            }
        } else {
            // Struct / enum destructure: `let VertexMap { mut inner } = map`
            // Register field binding types so `inner.get(k)` resolves to HashMap::get.
            self.register_destructure_binding_types(pattern);
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
                let prev_assign_int = self.assignment_int_target_type.take();
                if Self::assignment_target_needs_int_codegen_context(t) {
                    self.assignment_int_target_type = Some(t.clone());
                }
                let prev_suppress_turbo = self.suppress_collection_turbofish;
                let suppress_turbofish_here =
                    crate::codegen::rust::collection_detection::type_is_collect_turbofish_target(t);
                if suppress_turbofish_here {
                    self.suppress_collection_turbofish = true;
                }

                let prev_collect_target = self.collect_target_type.take();
                if suppress_turbofish_here {
                    self.collect_target_type = Some(t.clone());
                }

                // Auto-convert &str to String if type is String
                let mut value_str = self.generate_expression(value);

                self.collect_target_type = prev_collect_target;
                self.suppress_collection_turbofish = prev_suppress_turbo;
                self.assignment_int_target_type = prev_assign_int;
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
                if let Expression::Identifier { name, .. } = value {
                    if !Self::is_mut_local_self_rebind(pattern, value, mutable, auto_needs_mut) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(name, self.current_statement_idx)
                                .is_some()
                                && !value_str.ends_with(".clone()")
                                && !value_str.ends_with(".to_string()")
                            {
                                // WDB-343: `maybe_auto_clone` skips Copy scalars — do not
                                // force-append `.clone()` after it (that undoes Copy skips).
                                value_str = self.maybe_auto_clone(name, &value_str);
                            }
                        }
                    }
                    string_utilities::rewrite_borrowed_str_clone_to_to_string(
                        &mut value_str,
                        value,
                        &self.emitted_rust_ref_formals,
                        &self.current_function_params,
                    );
                }
                if let Some(vn) = var_name {
                    self.reconcile_ambiguous_int_local_after_let(vn, value, &value_str);
                    self.sync_i32_coord_binding_after_let(vn, &value_str);
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
                        self.local_var_types.get(vn).is_some_and(|ty| {
                            matches!(ty, Type::Vec(_) | Type::Parameterized(_, _))
                                && !crate::codegen::rust::types::type_contains_unbound_generic_param(
                                    ty,
                                )
                        })
                    });
                if needs_collection_ascription_sv {
                    let vn = var_name.unwrap();
                    let ty = self.local_var_types.get(vn).unwrap().clone();
                    output.push_str(": ");
                    output.push_str(&self.type_to_rust(&ty));
                } else if string_utilities::untyped_let_rhs_needs_string_ascription(value) {
                    output.push_str(": String");
                } else if mutable
                    && Self::mut_let_rhs_is_return_width_counter(value)
                {
                    // WDB-305: later `x = u32` assign beats return-width i64 for untyped counters.
                    let later_peer = var_name.and_then(|vn| {
                        self.mut_int_local_peer_width_from_later_assigns(vn)
                    });
                    // WDB-308: `-> u32` / later u32 peer wins over `.len()` usize marking.
                    // WDB-361: `.len()` while-counters win over `-> i32` / later i32 peer.
                    let prefer_usize_over_i32 = var_name
                        .is_some_and(|vn| self.usize_variables.contains(vn))
                        && !matches!(later_peer.as_ref(), Some(Type::Uint))
                        && !self.function_returns_u32_for_loop_scan();
                    if let Some(Type::Uint) = later_peer.as_ref() {
                        output.push_str(": u32");
                        if let Some(vn) = var_name {
                            self.local_var_types.insert(vn.to_string(), Type::Uint);
                        }
                    } else if prefer_usize_over_i32 {
                        output.push_str(": usize");
                        if let Some(vn) = var_name {
                            self.local_var_types
                                .insert(vn.to_string(), Type::Custom("usize".into()));
                        }
                    } else if let Some(Type::Int32) = later_peer.as_ref() {
                        output.push_str(": i32");
                        if let Some(vn) = var_name {
                            self.local_var_types.insert(vn.to_string(), Type::Int32);
                            self.codegen_i32_binding_names.insert(vn.to_string());
                        }
                    } else if let Some(ret_ty) = &self.current_function_return_type {
                        match Self::peel_option_result_payload(ret_ty) {
                            Type::Int32 => {
                                // WDB-361: `.len()` while-counters already in `usize_variables`
                                // must stay usize — do not stamp `: i32` from `-> i32` return.
                                if var_name.is_some_and(|vn| self.usize_variables.contains(vn)) {
                                    output.push_str(": usize");
                                    if let Some(vn) = var_name {
                                        self.local_var_types.insert(
                                            vn.to_string(),
                                            Type::Custom("usize".into()),
                                        );
                                    }
                                } else {
                                    output.push_str(": i32");
                                    if let Some(vn) = var_name {
                                        self.local_var_types.insert(vn.to_string(), Type::Int32);
                                        self.codegen_i32_binding_names.insert(vn.to_string());
                                        if self.function_returns_i32_for_loop_scan() {
                                            self.usize_variables.remove(vn);
                                        }
                                    }
                                }
                            }
                            Type::Uint => {
                                output.push_str(": u32");
                                if let Some(vn) = var_name {
                                    self.local_var_types.insert(vn.to_string(), Type::Uint);
                                }
                            }
                            Type::Custom(n) if matches!(n.as_str(), "u32" | "i32") => {
                                // WDB-361: `-> i32` often arrives as Custom("i32"); prefer
                                // usize for `.len()` while-counters. WDB-308: `-> u32` stays u32.
                                if n == "i32"
                                    && var_name
                                        .is_some_and(|vn| self.usize_variables.contains(vn))
                                {
                                    output.push_str(": usize");
                                    if let Some(vn) = var_name {
                                        self.local_var_types.insert(
                                            vn.to_string(),
                                            Type::Custom("usize".into()),
                                        );
                                    }
                                } else {
                                    output.push_str(": ");
                                    output.push_str(n);
                                    if let Some(vn) = var_name {
                                        self.local_var_types
                                            .insert(vn.to_string(), Type::Custom(n.clone()));
                                        if n == "i32" {
                                            self.codegen_i32_binding_names.insert(vn.to_string());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
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

                let prev_assign_int = self.assignment_int_target_type.take();
                if mutable && Self::mut_let_rhs_is_return_width_counter(value) {
                    // WDB-305: later u32 assign peer beats return i64 for `let mut x = 0`.
                    if let Some(vn) = var_name {
                        if let Some(peer) = self.mut_int_local_peer_width_from_later_assigns(vn) {
                            self.assignment_int_target_type = Some(peer);
                        }
                    }
                    if self.assignment_int_target_type.is_none() {
                        if let Some(ret_ty) = &self.current_function_return_type {
                            if Self::assignment_target_needs_int_codegen_context(ret_ty) {
                                // Store peeled int width (not Result/Option wrapper).
                                self.assignment_int_target_type =
                                    Some(self.int_width_hint_from_return_type_resolved(ret_ty));
                            }
                        }
                    }
                }
                // WDB-361: `.len()` while-counters already in `usize_variables` must not be
                // demoted by `-> i32` return-width scan (cast at `return i` instead).
                // WDB-308: `-> u32` still wins over usize for CLI/index loops.
                let len_while_usize_counter =
                    var_name.is_some_and(|n| self.usize_variables.contains(n));
                let i32_return_scan_counter = mutable
                    && Self::mut_let_rhs_is_return_width_counter(value)
                    && self.function_returns_i32_for_loop_scan()
                    && !len_while_usize_counter;
                let u32_return_scan_counter = mutable
                    && Self::mut_let_rhs_is_return_width_counter(value)
                    && (self.function_returns_u32_for_loop_scan()
                        || self.assignment_int_target_type.as_ref().is_some_and(|t| {
                            matches!(t, Type::Uint)
                                || matches!(t, Type::Custom(n) if n == "u32")
                        }));
                if len_while_usize_counter && !u32_return_scan_counter && !i32_return_scan_counter
                {
                    self.assignment_int_target_type = Some(Type::Custom("usize".into()));
                }
                if i32_return_scan_counter {
                    self.assignment_int_target_type = Some(Type::Int32);
                    if let Some(vn) = var_name {
                        self.usize_variables.remove(vn);
                    }
                } else if u32_return_scan_counter {
                    // WDB-308: return/later u32 peer wins over index-driven usize for `let mut i = 0`.
                    self.assignment_int_target_type = Some(Type::Uint);
                    if let Some(vn) = var_name {
                        self.usize_variables.remove(vn);
                        self.local_var_types.insert(vn.to_string(), Type::Uint);
                    }
                } else if self.assignment_int_target_type.is_none()
                    && self.function_prefers_i32_coord_locals()
                {
                    // WDB-328: bare `let mut i = -1` (Unary Neg of Int, or Int lit) must
                    // emit `_i32` in i32 builders — not stick to WJ `int`/i64 from default
                    // literal inference (`infer_expression_type(Literal::Int) → Type::Int`).
                    let bare_int_lit_init = matches!(
                        value,
                        Expression::Literal {
                            value: crate::parser::Literal::Int(_),
                            ..
                        }
                    ) || matches!(
                        value,
                        Expression::Unary {
                            op: crate::parser::UnaryOp::Neg,
                            operand,
                            ..
                        } if matches!(
                            &**operand,
                            Expression::Literal {
                                value: crate::parser::Literal::Int(_),
                                ..
                            }
                        )
                    );
                    let wj_int_slot = !bare_int_lit_init
                        && var_name.is_some_and(|vn| {
                            if self.explicit_wj_int_annotated_locals.contains(vn) {
                                return true;
                            }
                            matches!(
                                self.local_var_types.get(vn),
                                Some(Type::Int)
                            ) || matches!(
                                self.local_var_types.get(vn),
                                Some(Type::Custom(n)) if n == "int" || n == "i64"
                            )
                        });
                    let peer = if wj_int_slot {
                        Type::Int
                    } else if var_name.is_some_and(|vn| {
                        self.local_var_types.get(vn).is_some_and(|t| {
                            matches!(
                                t,
                                Type::Custom(n)
                                    if crate::codegen::rust::type_casting::assignment_int_peer_from_owner_type_name(
                                        n,
                                    )
                                    .is_some()
                            )
                        })
                    }) {
                        // P3_ATOMIC_I64_LET_OWNER_PEER: AtomicI64 let RHS keeps i64 constructor peers in void builders.
                        Type::Int
                    } else {
                        self.current_function_return_type
                            .as_ref()
                            .map(|rt| match Self::peel_option_result_payload(rt) {
                                Type::Uint => Type::Uint,
                                Type::Custom(n) if n == "u32" => Type::Uint,
                                _ => Type::Int32,
                            })
                            .unwrap_or(Type::Int32)
                    };
                    self.assignment_int_target_type = Some(peer);
                }

                // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                // String literals assigned to variables should become String (not &str)
                // because they may be passed to functions expecting String later.
                // This is safe because String auto-borrows to &str when needed.
                let mut value_str = self.generate_expression(value);
                self.assignment_int_target_type = prev_assign_int;
                if i32_return_scan_counter && value_str.ends_with("_usize") {
                    value_str = value_str.replace("_usize", "_i32");
                }
                if u32_return_scan_counter && value_str.ends_with("_usize") {
                    value_str = value_str.replace("_usize", "_u32");
                }

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
                } else if var_name.is_some_and(|vn| {
                    self.local_var_types
                        .get(vn)
                        .is_some_and(string_utilities::type_is_owned_string)
                }) && !string_utilities::already_owned_string_expr(&value_str)
                    && value_str.starts_with('&')
                {
                    // Untyped `let s = …` infers WJ `string` but RHS may emit `&text[i..j]`
                    // — coerce so `Some(s)` / owned returns typecheck.
                    value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                } else if mutable
                    && string_utilities::return_type_expects_owned_string(
                        &self.current_function_return_type,
                    )
                    && !string_utilities::already_owned_string_expr(&value_str)
                    && matches!(value, Expression::Identifier { name, .. }
                    if self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && (crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                || matches!(
                                    &p.type_,
                                    Type::Reference(inner)
                                        if crate::codegen::rust::types::is_windjammer_text_type(
                                            inner,
                                        )
                                ))
                    }))
                {
                    value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                }

                // E0507: `let x = self.field` through `&self`/`&mut self`:
                //   Option<T> behind &mut self → .take() (moves value, leaves None)
                //   other non-Copy → .clone()
                self.apply_self_field_move_fix(value, &mut value_str, var_name);
                self.apply_owned_param_field_extract_clone(value, &mut value_str);

                if let Expression::Identifier { name, .. } = value {
                    if !Self::is_mut_local_self_rebind(pattern, value, mutable, auto_needs_mut) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(name, self.current_statement_idx)
                                .is_some()
                                && !value_str.ends_with(".clone()")
                                && !value_str.ends_with(".to_string()")
                            {
                                // WDB-343: trust maybe_auto_clone (skips Copy; no force-append).
                                value_str = self.maybe_auto_clone(name, &value_str);
                            }
                        }
                    }
                    // P3.325: demoted `&str` formal `.clone()` is still `&str` — own it.
                    string_utilities::rewrite_borrowed_str_clone_to_to_string(
                        &mut value_str,
                        value,
                        &self.emitted_rust_ref_formals,
                        &self.current_function_params,
                    );
                }

                value_str = self.let_rhs_clone_if_mut_from_non_copy_ref(
                    mutable,
                    value,
                    needs_mut_ref,
                    &value_str,
                );

                if let Some(vn) = var_name {
                    self.reconcile_ambiguous_int_local_after_let(vn, value, &value_str);
                    self.sync_i32_coord_binding_after_let(vn, &value_str);
                }
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
                let prev_assign_int = self.assignment_int_target_type.take();
                if Self::assignment_target_needs_int_codegen_context(t) {
                    self.assignment_int_target_type = Some(t.clone());
                }
                let prev_suppress_turbo = self.suppress_collection_turbofish;
                let suppress_turbofish_here =
                    crate::codegen::rust::collection_detection::type_is_collect_turbofish_target(t);
                if suppress_turbofish_here {
                    self.suppress_collection_turbofish = true;
                }

                let prev_collect_target = self.collect_target_type.take();
                if suppress_turbofish_here {
                    self.collect_target_type = Some(t.clone());
                }

                // Auto-convert &str to String if type is String
                let mut value_str = self.generate_expression(value);

                self.collect_target_type = prev_collect_target;
                self.suppress_collection_turbofish = prev_suppress_turbo;
                self.assignment_int_target_type = prev_assign_int;
                self.assignment_float_target_type = prev_assign_float;

                self.apply_vec_index_let_rhs_fixup(var_name, value, Some(t), &mut value_str);
                let is_string_type = matches!(t, Type::String)
                    || matches!(t, Type::Custom(name) if name == "String" || name == "string");

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
                    self.local_var_types.get(vn).is_some_and(|ty| {
                        matches!(ty, Type::Vec(_) | Type::Parameterized(_, _))
                            && !crate::codegen::rust::types::type_contains_unbound_generic_param(ty)
                    })
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
                } else if mutable
                    && string_utilities::return_type_expects_owned_string(
                        &self.current_function_return_type,
                    )
                    && !string_utilities::already_owned_string_expr(&value_str)
                    && matches!(value, Expression::Identifier { name, .. }
                    if self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && (p.type_ == Type::String
                                || matches!(
                                    &p.type_,
                                    Type::Custom(n) if n == "string" || n == "String"
                                )
                                || matches!(
                                    &p.type_,
                                    Type::Reference(inner)
                                        if matches!(inner.as_ref(), Type::String)
                                            || matches!(
                                                inner.as_ref(),
                                                Type::Custom(s) if s == "str"
                                            )
                                ))
                    }))
                {
                    value_str = string_utilities::coerce_expr_to_owned_string(&value_str);
                }

                // E0507: `let x = self.field` through `&self`/`&mut self`:
                //   Option<T> behind &mut self → .take() (moves value, leaves None)
                //   other non-Copy → .clone()
                self.apply_self_field_move_fix(value, &mut value_str, var_name);
                self.apply_owned_param_field_extract_clone(value, &mut value_str);

                if let Expression::Identifier { name, .. } = value {
                    if !Self::is_mut_local_self_rebind(pattern, value, mutable, auto_needs_mut) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(name, self.current_statement_idx)
                                .is_some()
                                && !value_str.ends_with(".clone()")
                                && !value_str.ends_with(".to_string()")
                            {
                                // WDB-343: trust maybe_auto_clone (skips Copy; no force-append).
                                value_str = self.maybe_auto_clone(name, &value_str);
                            }
                        }
                    }
                    string_utilities::rewrite_borrowed_str_clone_to_to_string(
                        &mut value_str,
                        value,
                        &self.emitted_rust_ref_formals,
                        &self.current_function_params,
                    );
                }

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
                || self.infer_expression_type_is_usize(value)
                || matches!(value, Expression::MethodCall { method, .. } if method == "len")
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
                    // Signature / consensus only — unknown user methods are not mutating
                    // (assignment detection covers field writes). Do not treat
                    // `!is_known_readonly` as mutate: consensus miss would false-upgrade get→get_mut.
                    return super::self_analysis::method_is_mutating(
                        method,
                        Some(&self.signature_registry),
                        None,
                        Some(&self.self_receiver_upgrades),
                    );
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
                    && super::self_analysis::method_is_mutating(
                        method,
                        Some(&self.signature_registry),
                        None,
                        Some(&self.self_receiver_upgrades),
                    )
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

    /// Register types for bindings in `let Type { field, mut other } = …` so later
    /// method calls on those bindings resolve via `Type::field`'s type (e.g. HashMap).
    fn register_destructure_binding_types(&mut self, pattern: &Pattern<'_>) {
        use crate::parser::EnumPatternBinding;
        let Pattern::EnumVariant(type_name, EnumPatternBinding::Struct(fields, _)) = pattern else {
            return;
        };
        let base = type_name.split('<').next().unwrap_or(type_name);
        let field_types: Vec<(String, Type)> = self
            .lookup_struct_field_types(type_name)
            .or_else(|| self.lookup_struct_field_types(base))
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        if field_types.is_empty() {
            return;
        }
        for (field_name, pat) in fields {
            let binding = match pat {
                Pattern::Identifier(n) | Pattern::MutBinding(n) => Some(n.as_str()),
                Pattern::Ref(n) | Pattern::RefMut(n) => Some(n.as_str()),
                _ => None,
            };
            let Some(binding) = binding else { continue };
            if let Some((_, ty)) = field_types.iter().find(|(k, _)| k == field_name) {
                self.local_var_types.insert(binding.to_string(), ty.clone());
            }
        }
    }

    /// `let mut x = x` (or `let mut x = x` with MutBinding) must move, not auto-clone.
    fn is_mut_local_self_rebind(
        pattern: &Pattern,
        value: &Expression,
        mutable: bool,
        auto_needs_mut: bool,
    ) -> bool {
        let bound = match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => Some(name.as_str()),
            _ => None,
        };
        let init = match value {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            _ => None,
        };
        matches!((bound, init), (Some(b), Some(i)) if b == i)
            && (mutable || auto_needs_mut || matches!(pattern, Pattern::MutBinding(_)))
    }
}

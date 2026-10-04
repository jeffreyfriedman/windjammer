//! Assignment statement generation
//!
//! Handles code generation for assignments including:
//! - Simple assignments (x = y)
//! - Compound assignments (+=, -=, *=, etc.)
//! - Field assignments (self.x = y)
//! - Index assignments (arr[i] = y)
//! - Float type inference for assignments
//! - Reference/dereference handling

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for an assignment statement
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn generate_assignment_statement(
        &mut self,
        target: &'ast Expression<'ast>,
        value: &'ast Expression<'ast>,
        compound_op: &Option<crate::parser::ast::CompoundOp>,
    ) -> String {
        let mut output = self.indent();

        if let Some(op) = compound_op {
            self.generating_assignment_target = true;
            let target_str = self.generate_expression(target);
            self.generating_assignment_target = false;

            // Compound assignments on `&mut T` bindings need `*var += x` (E0368).
            // Covers for-loop `&mut` iteration and MutBorrowed Copy formals
            // (`increment(x: &mut i64)` → `*x += 1`).
            let needs_deref = if let Expression::Identifier { name, .. } = target {
                self.mut_borrowed_iterator_vars.contains(name)
                    || self.inferred_mut_borrowed_params.contains(name)
                    || self.identifier_already_mut_ref(name)
            } else {
                false
            };

            if needs_deref {
                output.push('*');
            }
            output.push_str(&target_str);

            output.push_str(match op {
                CompoundOp::Add => " += ",
                CompoundOp::Sub => " -= ",
                CompoundOp::Mul => " *= ",
                CompoundOp::Div => " /= ",
                CompoundOp::Mod => " %= ",
                CompoundOp::BitAnd => " &= ",
                CompoundOp::BitOr => " |= ",
                CompoundOp::BitXor => " ^= ",
                CompoundOp::Shl => " <<= ",
                CompoundOp::Shr => " >>= ",
            });

            let prev_assign_ty = self.assignment_float_target_type.take();
            let prev_assign_int = self.assignment_int_target_type.take();
            let tgt_ty = self.infer_expression_type(target);
            if tgt_ty
                .as_ref()
                .is_some_and(Self::assignment_target_needs_float_codegen_context)
            {
                self.assignment_float_target_type = tgt_ty.clone();
            }
            if tgt_ty
                .as_ref()
                .is_some_and(Self::assignment_target_needs_int_codegen_context)
            {
                self.assignment_int_target_type = tgt_ty.clone();
            }
            // Annotated / local usize bindings beat weak inference (WDB-121).
            if let Expression::Identifier { name, .. } = target {
                if let Some(t) = self.local_var_types.get(name) {
                    if Self::assignment_target_needs_int_codegen_context(t) {
                        self.assignment_int_target_type = Some(t.clone());
                    }
                }
            }
            self.set_assignment_int_target_from_compound_target(target);
            let mut value_str = self.generate_expression(value);

            // Int counters compared with `.len()` must not accumulate via `1 as usize` (P3.304).
            if self
                .resolve_compound_assign_int_rust_type_name(target)
                .is_some_and(|w| w != "usize")
            {
                value_str = Self::strip_compound_assign_int_literal_suffix(&value_str);
            }

            // Untyped integer literals: let Rust infer width from the binding (`i32 += 1`
            // not `i32 += 1_i64` when `let mut i = 0` is inferred as i32 from `while i < n as i32`).
            if matches!(
                value,
                Expression::Literal {
                    value: Literal::Int(_),
                    ..
                }
            ) && matches!(
                op,
                CompoundOp::Add
                    | CompoundOp::Sub
                    | CompoundOp::Mul
                    | CompoundOp::Div
                    | CompoundOp::Mod
            ) {
                value_str = Self::strip_compound_assign_int_literal_suffix(&value_str);
            }

            // Mixed int/float compound assignment: `f32 += i32` → `f32 += i32 as f32`
            // Only cast when the target is genuinely a float type (not int).
            if matches!(
                op,
                CompoundOp::Add
                    | CompoundOp::Sub
                    | CompoundOp::Mul
                    | CompoundOp::Div
                    | CompoundOp::Mod
            ) {
                let val_ty = self.infer_expression_type(value);
                let tgt_is_int = tgt_ty.as_ref().is_some_and(Self::is_int_numeric_type)
                    || if let Expression::Identifier { name, .. } = target {
                        self.local_var_types
                            .get(name)
                            .is_some_and(Self::is_int_numeric_type)
                    } else {
                        false
                    };
                if !tgt_is_int {
                    if let Some(v) = &val_ty {
                        if Self::is_int_numeric_type(v) {
                            let float_name = self.resolve_compound_assign_float_target(target);
                            if let Some(fname) = float_name {
                                if value_str.contains(" as ")
                                    || matches!(value, Expression::Binary { .. })
                                {
                                    value_str = format!("({}) as {}", value_str, fname);
                                } else {
                                    value_str = format!("{} as {}", value_str, fname);
                                }
                            }
                        }
                    }
                } else if let Some(cast) = self.resolve_compound_assign_int_rust_type_name(target) {
                    // P3.646: `HashMap::values()` / borrowed-iter `&Copy` into owned
                    // int compound assign → `*count`, not `count as usize` (E0606).
                    let deref_borrowed_copy = self.deref_borrowed_iter_copy_rhs_for_int_compound(
                        value,
                        &mut value_str,
                        tgt_is_int,
                        val_ty.as_ref(),
                    );
                    if !deref_borrowed_copy {
                        let val_width = val_ty
                            .as_ref()
                            .and_then(Self::int_rust_type_name)
                            .unwrap_or("i64");
                        let already_target_cast = value_str.ends_with(&format!(" as {cast}"))
                            || value_str.ends_with(&format!(") as {cast}"));
                        if val_width != cast
                            && !already_target_cast
                            && !Self::compound_rhs_is_untyped_int_literal(value, &value_str)
                        {
                            if matches!(value, Expression::Binary { .. })
                                || matches!(value, Expression::Call { .. })
                            {
                                value_str = format!("({value_str}) as {cast}");
                            } else {
                                value_str = format!("{value_str} as {cast}");
                            }
                        }
                    }
                }
            }

            self.assignment_int_target_type = prev_assign_int;
            self.assignment_float_target_type = prev_assign_ty;

            // String += String doesn't work in Rust (needs String += &str).
            // Only add & when the RHS is NOT a Copy type — Copy types (i32, f32, etc.)
            // work directly in compound assignments without borrowing.
            if matches!(op, CompoundOp::Add) {
                let value_is_copy = self
                    .infer_expression_type(value)
                    .as_ref()
                    .is_some_and(|t| self.is_type_copy(t));

                if !value_is_copy {
                    if let Expression::Identifier { name, .. } = value {
                        if self.owned_string_iterator_vars.contains(name) {
                            value_str = format!("&{}", value_str);
                        }
                    }

                    let value_type = self.infer_expression_type(value);
                    if matches!(value_type, Some(Type::String)) {
                        let is_string_literal = matches!(
                            value,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        );
                        let already_borrowed = value_str.starts_with('&');

                        if !is_string_literal && !already_borrowed {
                            value_str = format!("&{}", value_str);
                        }
                    }
                }
            }

            output.push_str(&value_str);
            output.push_str(";\n");
            return output;
        }

        if let Expression::Binary {
            left, right, op, ..
        } = value
        {
            let targets_match = match (target, &**left) {
                (
                    Expression::Identifier { name: t, .. },
                    Expression::Identifier { name: l, .. },
                ) => t == l,
                (Expression::FieldAccess { .. }, Expression::FieldAccess { .. })
                | (Expression::Index { .. }, Expression::Index { .. }) => {
                    self.generate_expression(target) == self.generate_expression(left)
                }
                _ => false,
            };

            let target_type = self.infer_expression_type(target);
            let right_type = self.infer_expression_type(right);

            // TDD FIX: String += String/&str doesn't work in Rust (needs String += &str with explicit &)
            // Disable compound assignment if EITHER:
            // 1. Right side is String/&str (needs borrowing)
            // 2. Target is String (likely string concatenation)
            let right_is_string_like = match &right_type {
                Some(Type::String) => true,
                Some(Type::Reference(inner)) => matches!(&**inner, Type::String),
                _ => false,
            };
            let target_is_string = matches!(&target_type, Some(Type::String));
            let is_string_addition =
                matches!(op, BinaryOp::Add) && (right_is_string_like || target_is_string);

            let target_supports_compound_assign = target_type.as_ref().is_some_and(|t| {
                matches!(
                    t,
                    Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool
                ) || matches!(t, Type::Custom(name) if crate::type_classification::is_numeric_type(name))
            });
            let is_compound_safe = target_supports_compound_assign && !is_string_addition;

            if targets_match && is_compound_safe {
                let compound_op_str = match op {
                    BinaryOp::Add => Some("+="),
                    BinaryOp::Sub => Some("-="),
                    BinaryOp::Mul => Some("*="),
                    BinaryOp::Div => Some("/="),
                    BinaryOp::Mod => Some("%="),
                    BinaryOp::BitAnd => Some("&="),
                    BinaryOp::BitOr => Some("|="),
                    BinaryOp::BitXor => Some("^="),
                    BinaryOp::Shl => Some("<<="),
                    BinaryOp::Shr => Some(">>="),
                    _ => None,
                };

                if let Some(op_str) = compound_op_str {
                    self.generating_assignment_target = true;
                    let target_str = self.generate_expression(target);
                    self.generating_assignment_target = false;

                    // `&mut T` formals / for-loop mut bindings: `*x += 1` (E0368).
                    let needs_deref = if let Expression::Identifier { name, .. } = target {
                        self.mut_borrowed_iterator_vars.contains(name)
                            || self.inferred_mut_borrowed_params.contains(name)
                            || self.identifier_already_mut_ref(name)
                    } else {
                        false
                    };

                    if needs_deref {
                        output.push('*');
                    }
                    output.push_str(&target_str);
                    output.push(' ');
                    output.push_str(op_str);
                    output.push(' ');
                    let prev_assign_ty = self.assignment_float_target_type.take();
                    let prev_assign_int = self.assignment_int_target_type.take();
                    let tgt_ty = self.infer_expression_type(target);
                    if tgt_ty
                        .as_ref()
                        .is_some_and(Self::assignment_target_needs_float_codegen_context)
                    {
                        self.assignment_float_target_type = tgt_ty.clone();
                    }
                    if tgt_ty
                        .as_ref()
                        .is_some_and(Self::assignment_target_needs_int_codegen_context)
                    {
                        self.assignment_int_target_type = tgt_ty.clone();
                    }
                    if let Expression::Identifier { name, .. } = target {
                        if let Some(t) = self.local_var_types.get(name) {
                            if Self::assignment_target_needs_int_codegen_context(t) {
                                self.assignment_int_target_type = Some(t.clone());
                            }
                        }
                    }
                    self.set_assignment_int_target_from_compound_target(target);
                    let mut right_str = self.generate_expression(right);
                    if matches!(
                        right,
                        Expression::Literal {
                            value: Literal::Int(_),
                            ..
                        }
                    ) && matches!(
                        op,
                        BinaryOp::Add
                            | BinaryOp::Sub
                            | BinaryOp::Mul
                            | BinaryOp::Div
                            | BinaryOp::Mod
                    ) {
                        right_str = Self::strip_compound_assign_int_literal_suffix(&right_str);
                    }
                    if self
                        .resolve_compound_assign_int_rust_type_name(target)
                        .is_some_and(|w| w != "usize")
                    {
                        right_str = Self::strip_compound_assign_int_literal_suffix(&right_str);
                    }

                    // Mixed int/float: cast RHS integer to target float type
                    // Only cast when the target is genuinely a float type (not int).
                    let synth_tgt_is_int = tgt_ty.as_ref().is_some_and(Self::is_int_numeric_type)
                        || if let Expression::Identifier { name, .. } = target {
                            self.local_var_types
                                .get(name)
                                .is_some_and(Self::is_int_numeric_type)
                        } else {
                            false
                        };
                    if !synth_tgt_is_int
                        && matches!(
                            op,
                            BinaryOp::Add
                                | BinaryOp::Sub
                                | BinaryOp::Mul
                                | BinaryOp::Div
                                | BinaryOp::Mod
                        )
                    {
                        let rhs_ty = self.infer_expression_type(right);
                        if let Some(v) = &rhs_ty {
                            if Self::is_int_numeric_type(v) {
                                let tgt_float = self.resolve_compound_assign_float_target(target);
                                if let Some(float_name) = tgt_float {
                                    if right_str.contains(" as ")
                                        || matches!(&**right, Expression::Binary { .. })
                                    {
                                        right_str = format!("({}) as {}", right_str, float_name);
                                    } else {
                                        right_str = format!("{} as {}", right_str, float_name);
                                    }
                                }
                            }
                        }
                    } else if let Some(cast) =
                        self.resolve_compound_assign_int_rust_type_name(target)
                    {
                        let rhs_ty = self.infer_expression_type(right);
                        // P3.646: never `ref_copy as usize` — deref `&Copy` into owned int.
                        let deref_borrowed_copy = self
                            .deref_borrowed_iter_copy_rhs_for_int_compound(
                                right,
                                &mut right_str,
                                true,
                                rhs_ty.as_ref(),
                            );
                        let val_width = rhs_ty
                            .as_ref()
                            .and_then(|t| match t {
                                Type::Reference(inner) | Type::MutableReference(inner) => {
                                    Self::int_rust_type_name(inner)
                                }
                                other => Self::int_rust_type_name(other),
                            })
                            .unwrap_or("i64");
                        let already_target_cast = right_str.ends_with(&format!(" as {cast}"))
                            || right_str.ends_with(&format!(") as {cast}"));
                        // WDB-302: never narrow an explicit `as i64` RHS to i32
                        // (`triangles as i64 as i32`) — emitted i64 width wins.
                        let rhs_explicit_i64 =
                            right_str.ends_with(" as i64") || right_str.ends_with(") as i64");
                        if !deref_borrowed_copy
                            && val_width != cast
                            && !already_target_cast
                            && !(cast == "i32" && rhs_explicit_i64)
                            && !Self::compound_rhs_is_untyped_int_literal(right, &right_str)
                        {
                            if matches!(right, Expression::Binary { .. })
                                || matches!(right, Expression::Call { .. })
                            {
                                right_str = format!("({right_str}) as {cast}");
                            } else {
                                right_str = format!("{right_str} as {cast}");
                            }
                        }
                        if rhs_explicit_i64 {
                            if let Expression::Identifier { name, .. } = target {
                                self.local_var_types.insert(name.to_string(), Type::Int);
                                self.codegen_i32_binding_names.remove(name);
                            }
                        }
                    }

                    self.assignment_int_target_type = prev_assign_int;
                    self.assignment_float_target_type = prev_assign_ty;

                    output.push_str(&right_str);
                    output.push_str(";\n");
                    return output;
                }
            }
        }

        self.generating_assignment_target = true;
        let target_str = self.generate_expression(target);
        self.generating_assignment_target = false;

        // TDD FIX: Regular assignments on mutable references need deref operator
        // For loop variables bound from &mut iteration are &mut T, so `var = x` must become `*var = x`
        let needs_deref = if let Expression::Identifier { name, .. } = target {
            self.mut_borrowed_iterator_vars.contains(name)
        } else {
            false
        };

        if needs_deref {
            output.push('*');
        }
        output.push_str(&target_str);
        output.push_str(" = ");

        let old_expr_ctx = self.in_expression_context;
        self.in_expression_context = true;

        let prev_assign_ty = self.assignment_float_target_type.take();
        let prev_assign_int = self.assignment_int_target_type.take();
        let tgt_ty = self.infer_expression_type(target);
        if tgt_ty
            .as_ref()
            .is_some_and(Self::assignment_target_needs_float_codegen_context)
        {
            self.assignment_float_target_type = tgt_ty.clone();
        } else if let Some(ft) = self.resolve_compound_assign_float_target(target) {
            // WDB-092: Index assign (`distances[u] = 0.0`) — fall back to float inference /
            // compound-assign resolution when direct LHS type inference is weak.
            self.assignment_float_target_type = Some(Type::Custom(ft.to_string()));
        }
        // P3.326: drive int literal / peer width from the LHS (same as compound assign).
        if tgt_ty
            .as_ref()
            .is_some_and(Self::assignment_target_needs_int_codegen_context)
        {
            self.assignment_int_target_type = tgt_ty.clone();
        }
        if let Expression::Identifier { name, .. } = target {
            if let Some(t) = self.local_var_types.get(name) {
                if Self::assignment_target_needs_int_codegen_context(t) {
                    self.assignment_int_target_type = Some(t.clone());
                }
            }
            if self.usize_variables.contains(name) {
                let wj_int_local = self.local_var_types.get(name).is_some_and(|t| {
                    matches!(t, Type::Int)
                        || matches!(t, Type::Custom(s) if s == "int" || s == "i64")
                });
                if !wj_int_local {
                    self.assignment_int_target_type = Some(Type::Custom("usize".into()));
                }
            }
        }
        self.set_assignment_int_target_from_compound_target(target);
        let mut value_str = self.generate_expression(value);
        self.assignment_float_target_type = prev_assign_ty;
        self.assignment_int_target_type = prev_assign_int;
        if matches!(
            value,
            Expression::Literal {
                value: Literal::String(_),
                ..
            }
        ) {
            value_str =
                crate::codegen::rust::string_utilities::coerce_expr_to_owned_string(&value_str);
        }

        // Vec<T>[i] → owned String field: clone the element, not borrow it.
        if matches!(value, Expression::Index { .. }) {
            let target_type = self.infer_expression_type(target);
            let expects_owned = !matches!(
                target_type.as_ref(),
                Some(Type::Reference(_)) | Some(Type::MutableReference(_))
            );
            if expects_owned
                && !value_str.ends_with(".clone()")
                && !crate::codegen::rust::literals::is_already_owned_string(&value_str)
            {
                let elem_type = self.infer_expression_type(value);
                if elem_type.as_ref().is_some_and(|t| !self.is_type_copy(t)) {
                    if value_str.starts_with('&') {
                        let base = value_str.trim_start_matches('&').trim();
                        value_str = format!("{}.clone()", base);
                    } else {
                        value_str = format!("{}.clone()", value_str);
                    }
                }
            }
        }

        if let Expression::Identifier { ref name, .. } = value {
            // Match/for bindings from borrowed scrutinees: Copy targets need * not .clone().
            let is_owned_match_binding = self.match_arm_bindings.contains(name.as_str())
                && !self.borrowed_iterator_vars.contains(name);
            if self.borrowed_iterator_vars.contains(name)
                && !is_owned_match_binding
                && !value_str.starts_with('*')
            {
                let target_type = self.infer_expression_type(target);
                if target_type.as_ref().is_some_and(|t| self.is_type_copy(t)) {
                    value_str = format!("*{}", value_str);
                }
            }

            {
                // P3.418: demoted `&mut String` / `&str` formals into owned String fields
                // (console.set_search_query) must `.to_string()` — MutBorrowed is tracked in
                // `inferred_mut_borrowed_params`, not only `inferred_borrowed_params`.
                let target_type = self.infer_expression_type(target);
                let owned_string_field = target_type.as_ref().is_some_and(|t| {
                    matches!(t, Type::String)
                        || matches!(t, Type::Custom(n) if n == "string" || n == "String")
                });
                let demoted_text_formal = self.inferred_borrowed_params.contains(name)
                    || self.inferred_mut_borrowed_params.contains(name)
                    || self.str_ref_optimized_params.contains(name)
                    || self.emitted_rust_ref_formals.contains(name);
                if owned_string_field
                    && demoted_text_formal
                    && !value_str.ends_with(".to_string()")
                    && !value_str.ends_with(".clone()")
                    && !crate::codegen::rust::literals::is_already_owned_string(&value_str)
                {
                    value_str = format!("{}.to_string()", value_str);
                }
            }

            // WDB-367: `None` / bool keywords are not bindings — auto_clone can
            // false-hit `"None"` at the wrong statement_idx → `None.clone()`.
            // Unit `None` never needs clone (Option::None is freely constructible).
            let is_unit_keyword = name == "None" || name == "true" || name == "false";
            if !is_unit_keyword {
                if let Some(ref analysis) = self.auto_clone_analysis {
                    if analysis
                        .needs_clone(name, self.current_statement_idx)
                        .is_some()
                        && !value_str.ends_with(".clone()")
                        && !value_str.starts_with('*')
                    {
                        let target_type = self.infer_expression_type(target);
                        let owned_string_field = target_type.as_ref().is_some_and(|t| {
                            matches!(t, Type::String)
                                || matches!(t, Type::Custom(n) if n == "string" || n == "String")
                        });
                        let target_is_copy =
                            target_type.as_ref().is_some_and(|t| self.is_type_copy(t));
                        // Dual-oracle: identifier emit already skips Copy; do not re-clone
                        // on field assign (WDB-391 helper u32, WDB-393 i32 formals, WDB-394 usize).
                        let skip_copy =
                            target_is_copy || self.ident_skips_auto_clone_as_copy(name, value);
                        if owned_string_field && self.inferred_borrowed_params.contains(name) {
                            value_str = format!("{}.to_string()", value_str);
                        } else if !skip_copy {
                            value_str = format!("{}.clone()", value_str);
                        }
                    }
                }
            }
            if self.inferred_borrowed_params.contains(name) {
                let target_type = self.infer_expression_type(target);
                let assignment_target_is_text = target_type
                    .as_ref()
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
                if assignment_target_is_text
                    && !value_str.contains(".clone()")
                    && !crate::codegen::rust::literals::is_already_owned_string(&value_str)
                {
                    value_str = format!("{}.into()", value_str);
                }
            }
            if self.into_string_formal_params.contains(name) {
                let target_type = self.infer_expression_type(target);
                let assignment_target_is_text = target_type
                    .as_ref()
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);
                if assignment_target_is_text
                    && !value_str.ends_with(".into()")
                    && !value_str.ends_with(".clone()")
                    && !value_str.ends_with(".to_string()")
                {
                    value_str = format!("{}.into()", value_str);
                }
            }
        }

        // Auto-clone when assigning one self field from another self field.
        // In Rust, `self.a = self.b` is E0507 when self is &mut self and b is non-Copy,
        // because you can't move out of a mutable reference. Clone solves this.
        if !value_str.ends_with(".clone()") && !value_str.ends_with(".to_string()") {
            let target_is_self_field = matches!(target, Expression::FieldAccess { object, .. }
                    if matches!(&**object, Expression::Identifier { name, .. } if name == "self"));
            let value_is_self_field = matches!(value, Expression::FieldAccess { object, .. }
                    if matches!(&**object, Expression::Identifier { name, .. } if name == "self"));

            if target_is_self_field && value_is_self_field {
                let val_type = self.infer_expression_type(value);
                let is_copy = val_type.as_ref().is_some_and(|t| self.is_type_copy(t));
                if !is_copy {
                    value_str = format!("{}.clone()", value_str);
                }
            }
        }

        {
            let target_type = self.get_assignment_target_type(target);
            self.maybe_cast_usize_to_int_target(&mut value_str, value, target_type.as_deref());
        }

        output.push_str(&value_str);

        self.in_expression_context = old_expr_ctx;

        output.push_str(";\n");
        output
    }

    /// P3.646: borrowed-iter Copy (`HashMap::values()` → `&usize`) into owned int
    /// compound assign must `*name`, not `name as usize`.
    ///
    /// Returns `true` when a leading `*` was applied (caller should skip width cast).
    fn deref_borrowed_iter_copy_rhs_for_int_compound(
        &self,
        value: &Expression<'_>,
        value_str: &mut String,
        tgt_is_int: bool,
        val_ty: Option<&Type>,
    ) -> bool {
        if !tgt_is_int {
            return false;
        }
        let Expression::Identifier { name, .. } = value else {
            return false;
        };
        // Prefer typed `&Copy` (values()/keys() loop locals). Also honor
        // `borrowed_iterator_vars` / local_var_types when infer lost the Reference wrap.
        let ty = val_ty
            .cloned()
            .or_else(|| self.local_var_types.get(name).cloned());
        let copy_pointee = match ty.as_ref() {
            Some(Type::Reference(inner) | Type::MutableReference(inner)) => {
                self.is_type_copy(inner)
            }
            Some(t)
                if self.borrowed_iterator_vars.contains(name) && self.is_type_copy(t) =>
            {
                true
            }
            None if self.borrowed_iterator_vars.contains(name) => true,
            // Map::values()/keys() always yield shared refs even when element is Copy.
            _ if self.borrowed_iterator_vars.contains(name)
                && crate::type_classification::is_numeric_type(
                    &match ty.as_ref() {
                        Some(Type::Custom(n)) => n.clone(),
                        Some(Type::Int) => "i64".into(),
                        Some(Type::Int32) => "i32".into(),
                        Some(Type::Uint) => "u64".into(),
                        _ => String::new(),
                    },
                ) =>
            {
                true
            }
            _ => false,
        };
        if !copy_pointee {
            return false;
        }
        // Drop a mistaken width cast on the bare ident (`count as usize` → `count`).
        let bare = if let Some(rest) = value_str.strip_prefix('*') {
            rest.trim()
        } else {
            value_str.as_str()
        };
        let bare = if let Some((head, _)) = bare.split_once(" as ") {
            if head.trim() == name {
                head.trim().to_string()
            } else {
                bare.to_string()
            }
        } else {
            bare.to_string()
        };
        *value_str = format!("*{bare}");
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::analyzer::Analyzer;
    use crate::codegen::rust::CodeGenerator;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::CompilationTarget;

    /// P3.646: Map::values() &usize into usize compound assign must deref.
    #[test]
    fn p3646_hashmap_values_usize_sum_must_deref() {
        let source = r#"
use std::collections::HashMap
pub struct Bus { counts: HashMap<string, usize> }
impl Bus {
    pub fn listener_count(self) -> usize {
        let mut total: usize = 0usize
        for count in self.counts.values() {
            total = total + count
        }
        total
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        let generated = codegen.generate_program(&program, &analyzed);
        assert!(
            !generated.contains("count as usize"),
            "P3.646 unit RED: must not cast &usize:\n{generated}"
        );
        assert!(
            generated.contains("*count"),
            "P3.646 unit RED: expect *count:\n{generated}"
        );
    }

    /// WDB-391: helper `u32` assigned to a field and reused must not `.clone()`.
    #[test]
    fn copy_u32_helper_return_assign_must_not_clone() {
        let source = r#"
pub fn get_screen_width() -> u32 {
    1280
}

pub fn get_screen_height() -> u32 {
    720
}

pub struct Renderer {
    pub screen_width: u32,
    pub screen_height: u32,
}

impl Renderer {
    pub fn init_gpu(self) {
        let w = get_screen_width()
        let h = get_screen_height()
        self.screen_width = w
        self.screen_height = h
        let pixel_count = w * h
        let _ = pixel_count
    }
}
"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        let program = parser.parse().expect("parse");
        let mut analyzer = Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
        let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
        let generated = codegen.generate_program(&program, &analyzed);
        assert!(
            !generated.contains("w.clone()") && !generated.contains("h.clone()"),
            "Copy u32 helper return must not clone on field assign/reuse:\n{generated}"
        );
    }
}

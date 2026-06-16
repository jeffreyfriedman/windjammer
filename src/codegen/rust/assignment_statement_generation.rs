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

            // TDD FIX: Compound assignments on mutable references need dereference operator
            // For loop variables bound from &mut iteration are &mut T, so `var += x` must become `*var += x`
            let needs_deref = if let Expression::Identifier { name, .. } = target {
                self.mut_borrowed_iterator_vars.contains(name)
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
            let tgt_ty = self.infer_expression_type(target);
            if tgt_ty
                .as_ref()
                .is_some_and(Self::assignment_target_needs_float_codegen_context)
            {
                self.assignment_float_target_type = tgt_ty.clone();
            }
            let mut value_str = self.generate_expression(value);

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
                }
            }

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

                    // TDD FIX: Compound assignments on mutable references need deref operator
                    let needs_deref = if let Expression::Identifier { name, .. } = target {
                        self.mut_borrowed_iterator_vars.contains(name)
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
                    let tgt_ty = self.infer_expression_type(target);
                    if tgt_ty
                        .as_ref()
                        .is_some_and(Self::assignment_target_needs_float_codegen_context)
                    {
                        self.assignment_float_target_type = tgt_ty.clone();
                    }
                    let mut right_str = self.generate_expression(right);

                    // Mixed int/float: cast RHS integer to target float type
                    // Only cast when the target is genuinely a float type (not int).
                    let synth_tgt_is_int = tgt_ty.as_ref().is_some_and(Self::is_int_numeric_type);
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
                    }

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
        let tgt_ty = self.infer_expression_type(target);
        if tgt_ty
            .as_ref()
            .is_some_and(Self::assignment_target_needs_float_codegen_context)
        {
            self.assignment_float_target_type = tgt_ty.clone();
        }
        let mut value_str = self.generate_expression(value);
        self.assignment_float_target_type = prev_assign_ty;
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

            if let Some(ref analysis) = self.auto_clone_analysis {
                if analysis
                    .needs_clone(name, self.current_statement_idx)
                    .is_some()
                    && !value_str.ends_with(".clone()")
                    && !value_str.starts_with('*')
                {
                    value_str = format!("{}.clone()", value_str);
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
}

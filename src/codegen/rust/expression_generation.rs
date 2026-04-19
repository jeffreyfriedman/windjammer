//! Expression Generation Module
//!
//! Handles generation of Rust code for all expression types:
//! - Literals, identifiers, binary/unary operations
//! - Function and method calls
//! - Field access, index access
//! - Struct/array/map literals
//! - Closures, blocks, match expressions
//! - Cast, try, await, range expressions

use crate::analyzer::*;
use crate::parser::*;

use super::arm_string_analysis;
use super::ast_utilities;
use super::constant_folding;
use super::expression_helpers;
use super::operators;
use super::pattern_analysis;
use super::string_analysis;
use super::CodeGenerator;

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    // Helper method for expressions that need to be evaluated without &mut self
    pub(crate) fn generate_expression_immut(&self, expr: &Expression) -> String {
        use crate::parser::ast::operators::{BinaryOp, UnaryOp};

        match expr {
            Expression::Literal { value: lit, .. } => self.generate_literal_with_context(lit, expr),
            Expression::Identifier { name, .. } => self.qualify_external_path_identifier(name),
            Expression::Unary { op, operand, .. } => {
                use crate::parser::Literal;
                // IntInference attaches constraints to the Unary for `-n` struct fields (score: -10).
                // Inner Literal would otherwise miss lookup and default to i32.
                if matches!(op, UnaryOp::Neg) {
                    if let Expression::Literal {
                        value: lit @ Literal::Int(_),
                        ..
                    } = &**operand
                    {
                        let s = self.generate_literal_with_context(lit, expr);
                        return format!("-{}", s);
                    }
                }
                
                // TDD FIX: Skip explicit * deref of &String in string comparisons
                // Problem: In Rust, *(&String) yields &str (not String), breaking &str == &String
                // Solution: Just use the identifier without *, making it &String == &String
                if matches!(op, UnaryOp::Deref) && self.in_string_comparison {
                    if let Some(operand_type) = self.infer_expression_type(operand) {
                        if matches!(operand_type, Type::Reference(inner) 
                            if crate::codegen::rust::types::is_windjammer_text_type(&inner)) {
                            // Skip the *, just generate the operand (keeping it as &String)
                            return self.generate_expression_immut(operand);
                        }
                    }
                }
                
                let op_str = match op {
                    UnaryOp::Not => "!",
                    UnaryOp::Neg => "-",
                    UnaryOp::Ref => "&",
                    UnaryOp::MutRef => "&mut ",
                    UnaryOp::Deref => "*",
                };
                format!("({}{})", op_str, self.generate_expression_immut(operand))
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::Eq => "==",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Le => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                    BinaryOp::BitAnd => "&",
                    BinaryOp::BitOr => "|",
                    BinaryOp::BitXor => "^",
                    BinaryOp::Shl => "<<",
                    BinaryOp::Shr => ">>",
                };

                // TDD FIX: Generate comparison without adding incorrect dereferences
                // When comparing &String == &String, both sides are already borrowed - no deref needed!
                // Rust's PartialEq trait handles comparisons correctly for references.
                let left_str = self.generate_expression_immut(left);
                let right_str = self.generate_expression_immut(right);

                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::FieldAccess { object, field, .. } => {
                format!("{}.{}", self.generate_expression_immut(object), field)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                if method == "as_str" && arguments.is_empty() && self.expression_produces_str_ref(object) {
                    return self.generate_expression_immut(object);
                }

                let obj_str = self.generate_expression_immut(object);
                let args_str = arguments
                    .iter()
                    .map(|(_label, arg)| self.generate_expression_immut(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}.{}({})", obj_str, method, args_str)
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                let func_str = self.generate_expression_immut(function);

                // TDD FIX: Check if this is a stdlib method that needs usize parameters
                // e.g., Vec::with_capacity(size) where size: int should generate: Vec::with_capacity(size as usize)
                let func_name = match function {
                    Expression::Identifier { name, .. } => Some(name.as_str()),
                    Expression::FieldAccess { object, field, .. } => {
                        if let Expression::Identifier {
                            name: type_name, ..
                        } = &**object
                        {
                            Some(format!("{}::{}", type_name, field).leak() as &str)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                let needs_usize_first_arg = func_name.map_or(false, |name| {
                    name == "Vec::with_capacity"
                        || name == "HashMap::with_capacity"
                        || name == "String::with_capacity"
                        || name == "Vec::reserve"
                });

                let args_str = arguments
                    .iter()
                    .enumerate()
                    .map(|(idx, (_label, arg))| {
                        let arg_str = self.generate_expression_immut(arg);
                        // For first argument to with_capacity/reserve, cast int to usize if it's an identifier
                        if idx == 0 && needs_usize_first_arg {
                            if matches!(arg, Expression::Identifier { .. }) {
                                format!("{} as usize", arg_str)
                            } else {
                                arg_str
                            }
                        } else {
                            arg_str
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", func_str, args_str)
            }
            Expression::Index { object, index, .. } => {
                format!(
                    "{}[{}]",
                    self.generate_expression_immut(object),
                    self.generate_expression_immut(index)
                )
            }
            // For complex expressions, just output a placeholder
            // Decorators are primarily documentation/runtime checks
            _ => "true".to_string(),
        }
    }

    /// e.g., stack.item.id → "stack", self.field → "self"
    fn extract_root_identifier(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => Some(name.clone()),
            Expression::FieldAccess { object, .. } => self.extract_root_identifier(object),
            Expression::Index { object, .. } => self.extract_root_identifier(object),
            _ => None,
        }
    }

    /// `if let` / `match` on `&enum` binds `&U` for Copy fields; struct literals need `U` (E0308).
    fn peel_copy_ref_binding_for_struct_field(
        &self,
        expr: &Expression<'ast>,
        generated: &str,
    ) -> String {
        let Some(ty) = self.infer_expression_type(expr) else {
            return generated.to_string();
        };
        let pointee = match &ty {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            _ => return generated.to_string(),
        };
        if !self.is_type_copy(pointee) {
            return generated.to_string();
        }
        format!("*({generated})")
    }

    /// After `peel_copy_ref_binding_for_struct_field`, non-Copy `&T` bindings still need `.clone()`
    /// for owned struct fields (e.g. `Vec` from `if let E { clips, .. } = &vec[i]`).
    fn clone_non_copy_ref_binding_for_struct_field(
        &self,
        expr: &Expression<'ast>,
        expr_str: &str,
    ) -> String {
        if expr_str.contains(".clone()") {
            return expr_str.to_string();
        }
        let Some(ty) = self.infer_expression_type(expr) else {
            return expr_str.to_string();
        };
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                if self.is_type_copy(inner.as_ref()) {
                    expr_str.to_string()
                } else {
                    format!("{}.clone()", expr_str)
                }
            }
            _ => expr_str.to_string(),
        }
    }

    /// Check if match needs .clone() to avoid partial move from self
    fn match_needs_clone_for_self_field(
        &self,
        value: &Expression,
        arms: &[crate::parser::MatchArm],
    ) -> bool {
        let is_self_field = if let Expression::FieldAccess { object, .. } = value {
            matches!(&**object, Expression::Identifier { name, .. } if name == "self")
        } else {
            false
        };

        if !is_self_field {
            return false;
        }

        let has_self = self
            .current_function_params
            .iter()
            .any(|p| p.name == "self");

        if !has_self {
            return false;
        }

        arms.iter()
            .any(|arm| pattern_analysis::pattern_extracts_value(&arm.pattern))
    }

    fn generate_expression_with_precedence(&mut self, expr: &Expression<'ast>) -> String {
        // Wrap expressions in parentheses if they need them for proper precedence
        // when used as the object of a method call or field access
        match expr {
            Expression::Range { .. }
            | Expression::Binary { .. }
            | Expression::Closure { .. }
            | Expression::Unary { .. }
            | Expression::Cast { .. } => {
                // Unary expressions like (*entity).field need parens for correct precedence
                // Without parens: *entity.field means *(entity.field) - WRONG
                // With parens: (*entity).field means dereference then access field - CORRECT
                //
                // Cast expressions like (x as usize).method() need parens because `as` has
                // lower precedence than `.` in Rust:
                // Without parens: x as usize.method() means x as (usize.method()) - WRONG
                // With parens: (x as usize).method() - CORRECT
                format!("({})", self.generate_expression(expr))
            }
            _ => self.generate_expression(expr),
        }
    }

    // PHASE 7: Constant folding - evaluate constant expressions at compile time
    pub(crate) fn generate_expression(&mut self, expr: &Expression<'ast>) -> String {
        // RECURSION GUARD: Check depth before processing expression
        if let Err(e) = self.enter_recursion("generate_expression") {
            eprintln!("{}", e);
            return format!("/* {} */", e);
        }

        // PHASE 7: Try constant folding first
        let folded_expr = constant_folding::try_fold_constant(expr);
        let expr_to_generate = folded_expr.as_ref().unwrap_or(expr);

        let result = self.generate_expression_impl(expr_to_generate);
        self.exit_recursion();
        result
    }

    fn generate_expression_impl(&mut self, expr_to_generate: &Expression<'ast>) -> String {
        match expr_to_generate {
            Expression::Literal { value: lit, .. } => {
                self.generate_literal_with_context(lit, expr_to_generate)
            }
            Expression::Identifier { name, .. } => {
                // Qualified paths use :: from parser (e.g., std::fs::read)
                // Simple identifiers: variable_name -> variable_name
                // Check if this is a struct field and we're in an impl block
                // BUT: Don't apply implicit field access if:
                // 1. It's a parameter name (parameters shadow fields)
                // 2. It's a local variable (local vars shadow fields)
                let is_parameter = self.current_function_params.iter().any(|p| p.name == *name);
                let is_local_variable = self
                    .local_variable_scopes
                    .iter()
                    .any(|scope| scope.contains(name));

                let is_implicit_self_field = self.in_impl_block
                    && !is_parameter
                    && !is_local_variable
                    && self.current_struct_fields.contains(name);
                let base_name = if is_implicit_self_field {
                    format!("self.{}", name)
                } else {
                    name.clone()
                };
                let base_name = self.qualify_external_path_identifier(&base_name);

                // AUTO-CLONE: Check if this variable needs to be cloned at this point
                // CRITICAL: Never clone assignment targets (left side of `=`)
                // DOUBLE-CLONE FIX: Skip auto-clone when inside an explicit .clone() call
                if !self.generating_assignment_target && !self.in_explicit_clone_call {
                    if let Some(ref analysis) = self.auto_clone_analysis {
                        if analysis
                            .needs_clone(name, self.current_statement_idx)
                            .is_some()
                        {
                            // Skip .clone() for Copy types — they are implicitly copied,
                            // so .clone() is unnecessary noise.
                            let is_copy_type = analysis.string_literal_vars.contains(name)
                                || self.usize_variables.contains(name)
                                || self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));

                            if !is_copy_type {
                                return format!("{}.clone()", base_name);
                            }
                        }
                    }

                    // &self field clone: when accessing self.field in a &self method,
                    // non-Copy types can't be moved out of the reference — auto-clone.
                    // Skip in comparison contexts — refs compare fine without cloning.
                    if is_implicit_self_field
                        && self.inferred_borrowed_params.contains("self")
                        && !self.suppress_borrowed_clone
                    {
                        let field_is_copy = self
                            .current_struct_name
                            .as_ref()
                            .and_then(|sn| self.struct_field_types.get(sn.as_str()))
                            .and_then(|fields| fields.get(name))
                            .is_some_and(|ty| self.is_type_copy(ty));
                        if !field_is_copy {
                            return format!("{}.clone()", base_name);
                        }
                    }
                }

                base_name
            }
            Expression::Binary {
                left, op, right, ..
            } => {
                // TDD FIX: Optimize .len() comparisons to .is_empty()
                // Clippy warns about .len() == 0, .len() != 0, .len() > 0
                // Transform to .is_empty() or !.is_empty()
                if let Expression::MethodCall {
                    object,
                    method,
                    arguments,
                    ..
                } = left
                {
                    if method == "len" && arguments.is_empty() {
                        // Check if comparing to 0
                        if let Expression::Literal {
                            value: Literal::Int(0),
                            ..
                        } = right
                        {
                            match op {
                                BinaryOp::Eq => {
                                    // .len() == 0 → .is_empty()
                                    let prev = self.in_field_access_object;
                                    self.in_field_access_object = true;
                                    let obj_str = self.generate_expression(object);
                                    self.in_field_access_object = prev;
                                    return format!("{}.is_empty()", obj_str);
                                }
                                BinaryOp::Ne | BinaryOp::Gt => {
                                    // .len() != 0 → !.is_empty()
                                    // .len() > 0 → !.is_empty()
                                    let prev = self.in_field_access_object;
                                    self.in_field_access_object = true;
                                    let obj_str = self.generate_expression(object);
                                    self.in_field_access_object = prev;
                                    return format!("!{}.is_empty()", obj_str);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Special handling for string concatenation
                if matches!(op, BinaryOp::Add) {
                    let has_string_operand = matches!(
                        left,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || matches!(
                        right,
                        Expression::Literal {
                            value: Literal::String(_),
                            ..
                        }
                    ) || string_analysis::contains_string_literal(left)
                        || string_analysis::contains_string_literal(right)
                        || string_analysis::expression_produces_string(left)
                        || string_analysis::expression_produces_string(right);

                    if has_string_operand {
                        return self.generate_string_concat(left, right);
                    }
                }

                // Check for usize/i32 comparison or arithmetic - cast if needed
                let is_comparison = matches!(
                    op,
                    BinaryOp::Lt
                        | BinaryOp::Le
                        | BinaryOp::Gt
                        | BinaryOp::Ge
                        | BinaryOp::Eq
                        | BinaryOp::Ne
                );
                let is_arithmetic = matches!(
                    op,
                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                );
                let left_is_usize = self.expression_produces_usize(left);
                let right_is_usize = self.expression_produces_usize(right);
                let right_is_int_literal = matches!(
                    right,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );
                let left_is_int_literal = matches!(
                    left,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );

                // When true, the usize/len() comparison path below casts the usize/.len() side to `i64`.
                // Skip general int promotion so we never double-cast (e.g. `(len as i64) as u32`).
                // Only when the non-len operand is a signed / Windjammer `int` — not `usize`.
                let usize_cmp_cast_will_apply = is_comparison
                    && ((left_is_usize
                        && !right_is_usize
                        && !right_is_int_literal
                        && self.comparison_other_side_needs_len_as_i64(right))
                        || (right_is_usize
                            && !left_is_usize
                            && !left_is_int_literal
                            && self.comparison_other_side_needs_len_as_i64(left)));

                // COMPARISON CLONE SUPPRESSION: For comparison operators (==, !=, <, >, etc.),
                // suppress borrowed-iterator cloning on operands. Comparisons work on references
                // in Rust (&String == &String, &T == &T via PartialEq), so cloning is unnecessary.
                // e.g., `recipe.name.clone() == target` → `recipe.name == target`
                let prev_suppress = self.suppress_borrowed_clone;
                if is_comparison {
                    self.suppress_borrowed_clone = true;
                }

                // Wrap operands in parens if they have lower precedence, or if both
                // parent and child are comparison/equality operators (Rust forbids
                // chaining them, e.g. `a > b != c > d` is invalid).
                let parent_is_cmp = matches!(
                    op,
                    BinaryOp::Eq
                        | BinaryOp::Ne
                        | BinaryOp::Lt
                        | BinaryOp::Le
                        | BinaryOp::Gt
                        | BinaryOp::Ge
                );
                // TDD FIX: Set context flag for string comparisons to enable * deref removal
                // This allows the Unary::Deref generator to skip * for &String operands
                let is_string_comparison = is_comparison && matches!(op, BinaryOp::Eq | BinaryOp::Ne);
                if is_string_comparison {
                    // Check if either operand type is a string
                    let left_type = self.infer_expression_type(left);
                    let right_type = self.infer_expression_type(right);
                    let has_string = [left_type.as_ref(), right_type.as_ref()].iter().any(|t| {
                        t.is_some_and(|ty| {
                            crate::codegen::rust::types::is_windjammer_text_type(ty)
                                || matches!(ty, Type::Reference(inner) 
                                    if crate::codegen::rust::types::is_windjammer_text_type(inner))
                        })
                    });
                    if has_string {
                        self.in_string_comparison = true;
                    }
                }
                
                let mut left_str = match left {
                    Expression::Binary { op: left_op, .. } => {
                        let child_is_cmp = matches!(
                            left_op,
                            BinaryOp::Eq
                                | BinaryOp::Ne
                                | BinaryOp::Lt
                                | BinaryOp::Le
                                | BinaryOp::Gt
                                | BinaryOp::Ge
                        );
                        let needs_parens = operators::op_precedence(left_op)
                            < operators::op_precedence(op)
                            || (parent_is_cmp && child_is_cmp);
                        if needs_parens {
                            format!("({})", self.generate_expression(left))
                        } else {
                            self.generate_expression(left)
                        }
                    }
                    _ => self.generate_expression(left),
                };
                let mut right_str = match right {
                    Expression::Binary { op: right_op, .. } => {
                        let child_is_cmp = matches!(
                            right_op,
                            BinaryOp::Eq
                                | BinaryOp::Ne
                                | BinaryOp::Lt
                                | BinaryOp::Le
                                | BinaryOp::Gt
                                | BinaryOp::Ge
                        );
                        let needs_parens = operators::op_precedence(right_op)
                            < operators::op_precedence(op)
                            || (parent_is_cmp && child_is_cmp)
                            || operators::binary_rhs_needs_parens_for_rust_left_assoc(op, right_op);
                        if needs_parens {
                            format!("({})", self.generate_expression(right))
                        } else {
                            self.generate_expression(right)
                        }
                    }
                    _ => self.generate_expression(right),
                };
                
                // TDD FIX: Reset string comparison context flag after generating operands
                if is_string_comparison {
                    self.in_string_comparison = false;
                }

                // Restore previous suppress state
                self.suppress_borrowed_clone = prev_suppress;

                // WINDJAMMER PHILOSOPHY: Auto-cast int/usize in comparisons
                // When comparing int (i64) with usize, automatically cast to make it work.
                //
                // CORRECTNESS: Always cast the usize/.len() side to i64, NOT the int side to usize.
                // Casting i64 → usize is UNSAFE for negative values (wraps to a huge usize).
                // Casting usize → i64 is safe for lengths that fit in i64 (practical vectors).
                //
                // For int literals compared to usize: Rust infers the literal type from context
                // (no cast needed): `items.len() > 0` stays as-is.
                //
                // Examples:
                // - int < items.len()  →  int < (items.len() as i64)
                // - items.len() > int  →  (items.len() as i64) > int
                // - usize < items.len() → no cast (both usize)
                if is_comparison
                    && left_is_usize
                    && !right_is_usize
                    && !right_is_int_literal
                    && self.comparison_other_side_needs_len_as_i64(right)
                {
                    (left_str, right_str) = super::type_casting::cast_for_usize_binary_op(
                        &left_str,
                        &right_str,
                        true,
                        false,
                    );
                } else if is_comparison
                    && right_is_usize
                    && !left_is_usize
                    && !left_is_int_literal
                    && self.comparison_other_side_needs_len_as_i64(left)
                {
                    (left_str, right_str) = super::type_casting::cast_for_usize_binary_op(
                        &left_str,
                        &right_str,
                        false,
                        true,
                    );
                }
                // If both are usize: no cast (usize == usize is fine)
                // If neither is usize: no cast (i64 == i64 is fine)

                // AUTO-CAST: When doing arithmetic between usize and int literal, Rust infers
                // the literal type from context. So `items.len() - 1` works without casting.
                // Only cast if the literal is negative (usize can't represent negative values).
                if is_arithmetic && left_is_usize && right_is_int_literal && !right_is_usize {
                    let is_negative = matches!(right, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
                    if is_negative {
                        right_str = format!("{} as usize", right_str);
                    }
                } else if is_arithmetic && right_is_usize && left_is_int_literal && !left_is_usize {
                    let is_negative = matches!(left, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
                    if is_negative {
                        left_str = format!("{} as usize", left_str);
                    }
                }

                // Mixed concrete integer types (e.g. u32 vs i32): Rust needs explicit `as T`.
                // Only when int inference has resolved BOTH sides and they differ.
                // Skip if the usize/len() heuristic already cast one operand to usize.
                if !usize_cmp_cast_will_apply {
                    // `usize`/`len()` ± untyped literal: Rust infers the literal as `usize` — do not
                    // rewrite to `1_usize as i64` etc.
                    let skip_int_promotion_usize_arith_untyped_lit = is_arithmetic
                        && ((left_is_usize
                            && right_is_int_literal
                            && !right_is_usize)
                            || (right_is_usize
                                && left_is_int_literal
                                && !left_is_usize));
                    // Both operands are `usize` (locals/fields/suffixed literals): no `i64` promotion.
                    let skip_int_promotion_both_inferred_usize = (is_comparison || is_arithmetic)
                        && self.infer_expression_type_is_usize(left)
                        && self.infer_expression_type_is_usize(right);
                    if !skip_int_promotion_usize_arith_untyped_lit
                        && !skip_int_promotion_both_inferred_usize
                    {
                        if let Some(inference) = &self.int_inference {
                            if is_comparison || is_arithmetic {
                            use crate::type_inference::int_implicit_casts::{
                                get_cast_suffix, is_safe_implicit_cast, promote_types,
                            };
                            use crate::type_inference::IntType;

                            let left_ty = self.int_type_for_mixed_int_codegen(left, inference);
                            let right_ty = self.int_type_for_mixed_int_codegen(right, inference);
                            if left_ty != IntType::Unknown
                                && right_ty != IntType::Unknown
                                && left_ty != right_ty
                            {
                                let promoted = promote_types(left_ty, right_ty);
                                if promoted != IntType::Unknown {
                                    if left_ty != promoted
                                        && is_safe_implicit_cast(left_ty, promoted)
                                    {
                                        let suffix = get_cast_suffix(promoted);
                                        let needs_inner = matches!(left, Expression::Binary { .. })
                                            || left_str.contains(" as ");
                                        left_str = if needs_inner {
                                            format!("({}) as {}", left_str, suffix)
                                        } else {
                                            format!("{} as {}", left_str, suffix)
                                        };
                                    }
                                    if right_ty != promoted
                                        && is_safe_implicit_cast(right_ty, promoted)
                                    {
                                        let suffix = get_cast_suffix(promoted);
                                        let needs_inner =
                                            matches!(right, Expression::Binary { .. })
                                                || right_str.contains(" as ");
                                        right_str = if needs_inner {
                                            format!("({}) as {}", right_str, suffix)
                                        } else {
                                            format!("{} as {}", right_str, suffix)
                                        };
                                    }
                                }
                            }
                        }
                        }
                    }
                }

                // Mixed `usize` + `i32` in `+` / `-` (dogfooding: voxel grid coordinates).
                if is_arithmetic && matches!(op, BinaryOp::Add | BinaryOp::Sub) {
                    self.promote_usize_i32_mixed_add_sub(
                        left,
                        right,
                        &mut left_str,
                        &mut right_str,
                    );
                }

                // E0277: mixed f32/f64 (inference + `as f32` vs default `_f64` literals).
                if (is_arithmetic || is_comparison)
                    && matches!(
                        op,
                        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                    )
                {
                    let prefer_f32_from_assignment = is_arithmetic
                        && matches!(
                            &self.assignment_float_target_type,
                            Some(Type::Custom(n)) if n == "f32"
                        );
                    self.promote_mixed_f32_f64_operands(
                        left,
                        right,
                        &mut left_str,
                        &mut right_str,
                        prefer_f32_from_assignment,
                    );
                }

                // E0277: mixed int/float arithmetic (i32 + f32, usize * f32, etc.)
                if is_arithmetic {
                    self.promote_int_to_float_in_mixed_arithmetic(
                        left,
                        right,
                        &mut left_str,
                        &mut right_str,
                    );
                }

                let op_str = operators::binary_op_to_rust(op);

                // TDD FIX: Rust parses `expr as usize < y` as `expr as usize<y>` (generics).
                // When the left operand is a cast (or ends with `as TYPE`) and the operator
                // is `<`, we must wrap the left side in parentheses to disambiguate.
                // Other comparison operators (>=, <=, ==, !=, >) don't have this ambiguity.
                //
                // TDD FIX (VOXEL DOGFOODING): Bitwise operators (<<, >>, |, &, ^) have
                // LOWER precedence than `as` in Rust, so `(x as u32) << 8` is required.
                // Without parens: `x as u32 << 8` is parsed as `x as (u32 << 8)` - WRONG!
                //
                // DISCOVERED: VoxelColor::to_hex() compilation failure
                //   Source: `let r = (self.r as u32) << 24;`
                //   Generated: `let r = self.r as u32 << 24;`  ← Missing parens!
                //   Error: `<<` is interpreted as start of generic arguments for `u32`
                let needs_cast_parens_for_op =
                    matches!(op_str, "<" | ">" | "<<" | ">>" | "|" | "&" | "^");
                let left_needs_cast_parens = needs_cast_parens_for_op
                    && (matches!(left, Expression::Cast { .. }) || left_str.contains(" as "));
                let right_needs_cast_parens = needs_cast_parens_for_op
                    && (matches!(right, Expression::Cast { .. }) || right_str.contains(" as "));

                if left_needs_cast_parens {
                    left_str = format!("({})", left_str);
                }
                if right_needs_cast_parens {
                    right_str = format!("({})", right_str);
                }

                // TDD FIX: String + String/&str concatenation needs borrowing
                // In Rust, String + String doesn't work - needs String + &str
                // If LEFT side is String and op is Add, RIGHT must be borrowed (unless string literal)
                // Also: if RIGHT produces String (e.g., parts[j].clone()), add & for coercion
                if matches!(op, BinaryOp::Add) {
                    let left_type = self.infer_expression_type(left);
                    let right_type = self.infer_expression_type(right);
                    let left_is_string = matches!(left_type, Some(Type::String));
                    let right_is_string = matches!(right_type, Some(Type::String));

                    // Add & when either side is String (covers result + parts[j].clone())
                    if left_is_string || right_is_string {
                        // Don't add & for string literals (they're already &str)
                        let is_string_literal = matches!(
                            right,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        );
                        if !is_string_literal && !right_str.starts_with('&') {
                            right_str = format!("&{}", right_str);
                        }
                    }
                }

                // TDD FIX: Smart XOR deref logic for comparisons
                // Only applies to COMPARISON operators (==, !=, <, >, <=, >=).
                // Arithmetic operators (Add, Sub, Mul, Div) don't need this because
                // Rust auto-derefs Copy types in arithmetic, and non-Copy types use
                // trait impls that handle references.
                //
                // Rules (comparisons only):
                // - Both borrowed (&T == &T): NO deref (PartialEq<&T> works)
                // - Both owned (T == T): NO deref (PartialEq<T> works)
                // - One borrowed, one owned: Add * to borrowed side (XOR)
                //
                // NOTE: For text params typed as `string` (WJ) or `&str` (explicit),
                // is_str_param excludes them from XOR because &str comparisons work
                // natively in Rust. But &String iteration vars still need XOR deref
                // when compared with owned String (&String == String doesn't compile).
                if is_comparison {

                let is_str_param = |name: &str| {
                    self.current_function_params.iter().any(|p| {
                        p.name == name
                            && (matches!(p.type_, crate::parser::Type::String)
                                || matches!(p.type_, crate::parser::Type::Custom(ref n) if n == "string")
                                || matches!(&p.type_, crate::parser::Type::Reference(inner)
                                    if crate::codegen::rust::types::is_windjammer_text_type(inner)))
                            && self.inferred_borrowed_params.contains(name)
                    })
                };

                // Check if identifier is tracked (function param, match binding, local var)
                let left_is_tracked = match left {
                    Expression::Identifier { name, .. } => {
                        self.inferred_borrowed_params.contains(name.as_str())
                            || self.borrowed_iterator_vars.contains(name)
                            || self.local_var_types.contains_key(name.as_str())
                            || self.current_function_params.iter().any(|p| p.name == *name)
                    }
                    _ => true, // Non-identifier expressions are "tracked" (we know their type)
                };

                let right_is_tracked = match right {
                    Expression::Identifier { name, .. } => {
                        self.inferred_borrowed_params.contains(name.as_str())
                            || self.borrowed_iterator_vars.contains(name)
                            || self.local_var_types.contains_key(name.as_str())
                            || self.current_function_params.iter().any(|p| p.name == *name)
                    }
                    _ => true, // Non-identifier expressions are "tracked" (we know their type)
                };

                let left_is_borrowed = match left {
                    Expression::Identifier { name, .. } => {
                        !is_str_param(name)
                            && (self.inferred_borrowed_params.contains(name.as_str())
                                || self.borrowed_iterator_vars.contains(name))
                    }
                    Expression::MethodCall { method, .. } => {
                        method == "as_str"
                    }
                    _ => false,
                };

                let right_is_borrowed = match right {
                    Expression::Identifier { name, .. } => {
                        !is_str_param(name)
                            && (self.inferred_borrowed_params.contains(name.as_str())
                                || self.borrowed_iterator_vars.contains(name))
                    }
                    Expression::MethodCall { method, .. } => {
                        method == "as_str"
                    }
                    _ => false,
                };
                
                // Check if one side is an explicit deref of a borrowed value
                // Example: *id == flag_id where id is &String
                let left_is_explicit_deref = matches!(left, Expression::Unary { op: UnaryOp::Deref, .. });
                let right_is_explicit_deref = matches!(right, Expression::Unary { op: UnaryOp::Deref, .. });

                // TDD FIX: XOR logic for borrowed/owned mismatch ONLY when BOTH sides are tracked
                // Skip when one side is untracked (closure param, etc.) - likely BOTH are borrowed
                // ALSO skip when one side is explicit deref - handle in balance_eq_operands_for_rust
                if left_is_tracked && right_is_tracked && left_is_borrowed != right_is_borrowed 
                    && !left_is_explicit_deref && !right_is_explicit_deref {
                    if left_is_borrowed {
                        left_str = format!("*{}", left_str);
                    } else {
                        right_str = format!("*{}", right_str);
                    }
                }
                } // end is_comparison guard

                if is_comparison && matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
                    self.balance_eq_operands_for_rust(left, right, &mut left_str, &mut right_str);
                }

                format!("{} {} {}", left_str, op_str, right_str)
            }
            Expression::Unary { op, operand, .. } => {
                use crate::parser::Literal;
                if matches!(op, crate::parser::UnaryOp::Neg) {
                    if let Expression::Literal {
                        value: lit @ Literal::Int(_),
                        ..
                    } = &**operand
                    {
                        let s = self.generate_literal_with_context(lit, operand);
                        return format!("-{}", s);
                    }
                }

                // TDD FIX: Explicit deref handling is now in balance_eq_operands_for_rust
                // where we have access to BOTH operands to make the right decision
                
                let op_str = operators::unary_op_to_rust(op);

                // BORROW CONTEXT: When generating &expr or &mut expr, suppress Vec index
                // auto-clone in the operand. We want a reference to the original element.
                // e.g., &self.items[i] → NOT &self.items[i].clone()
                //        &mut self.items[i] → NOT &mut self.items[i].clone()
                let is_borrow = matches!(
                    op,
                    crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef
                );
                let prev_borrow = self.in_borrow_context;
                if is_borrow {
                    self.in_borrow_context = true;
                }
                let operand_str = self.generate_expression(operand);
                self.in_borrow_context = prev_borrow;

                // CRITICAL: Preserve parentheses for binary expressions in unary context
                // !(a || b) should generate !(a || b), not !a || b
                // Binary operators have lower precedence than unary operators, so we need parens
                let needs_parens = matches!(&**operand, Expression::Binary { .. });

                if needs_parens {
                    format!("{}({})", op_str, operand_str)
                } else {
                    format!("{}{}", op_str, operand_str)
                }
            }
            Expression::Call {
                function,
                arguments,
                ..
            } => {
                // Extract function name for signature lookup
                let func_name = ast_utilities::extract_function_name(function);

                // THE WINDJAMMER WAY: User-defined functions always take priority
                // over built-in name mappings. If the user defines a function with
                // the same name as a test macro or runtime function (e.g., their own
                // `assert_approx`), their definition wins. We check the signature
                // registry: if the function exists and is NOT extern, it's user-defined.
                let is_user_defined = self
                    .signature_registry
                    .get_signature(&func_name)
                    .map(|sig| !sig.is_extern)
                    .unwrap_or(false);

                if !is_user_defined {
                    // Special case: convert test assertion functions to macros
                    // THE WINDJAMMER WAY: assert_eq(a, b) -> assert_eq!(a, b)
                    // NOTE: assert_gt, assert_gte, assert_is_some, assert_is_none, etc. are runtime functions, not macros
                    // Print functions need special handling (format! unwrapping, interpolation)
                    // so they are NOT in the simple macro list — handled separately below.
                    let test_macros = [
                        "assert",
                        "assert_eq",
                        "assert_ne",
                        "assert_ok",
                        "assert_err",
                        "panic",
                        "vec",
                        "format",
                        "write",
                        "writeln",
                        "dbg",
                        "todo",
                        "unimplemented",
                        "unreachable",
                    ];

                    if test_macros.contains(&func_name.as_str()) {
                        // Rust 2021: panic!(format!("...", args)) is invalid because
                        // panic! requires a string literal as first arg.
                        // Unwrap: panic(format!("...", a, b)) → panic!("...", a, b)
                        if func_name == "panic" && arguments.len() == 1 {
                            if let Expression::MacroInvocation {
                                name: ref inner_name,
                                args: ref inner_args,
                                ..
                            } = arguments[0].1
                            {
                                if inner_name == "format" {
                                    let inner: Vec<String> = inner_args
                                        .iter()
                                        .map(|a| self.generate_expression(a))
                                        .collect();
                                    return format!("panic!({})", inner.join(", "));
                                }
                            }
                        }

                        let args: Vec<String> = arguments
                            .iter()
                            .map(|(_label, arg)| self.generate_expression(arg))
                            .collect();
                        return format!("{}!({})", func_name, args.join(", "));
                    }

                    // Special case: qualify test assertion runtime functions
                    // THE WINDJAMMER WAY: These are functions, not macros, so they need proper paths
                    let test_functions = [
                        "assert_gt",
                        "assert_lt",
                        "assert_gte",
                        "assert_lte",
                        "assert_approx",
                        "assert_not_empty",
                        "assert_empty",
                        "assert_contains",
                        "assert_is_some",
                        "assert_is_none",
                    ];

                    if test_functions.contains(&func_name.as_str()) {
                        let args: Vec<String> = arguments
                            .iter()
                            .enumerate()
                            .map(|(idx, (_label, arg))| {
                                let generated = self.generate_expression(arg);
                                // assert_is_some and assert_is_none expect &Option, so add & for first arg
                                if (func_name == "assert_is_some" || func_name == "assert_is_none")
                                    && idx == 0
                                {
                                    format!("&{}", generated)
                                } else {
                                    generated
                                }
                            })
                            .collect();
                        return format!(
                            "windjammer_runtime::test::{}({})",
                            func_name,
                            args.join(", ")
                        );
                    }
                }

                // Special case: convert print/println/eprintln/eprint() to macros
                if func_name == "print"
                    || func_name == "println"
                    || func_name == "eprintln"
                    || func_name == "eprint"
                {
                    let macro_name = func_name.clone();

                    // For print() -> println!(), otherwise keep the same name
                    let target_macro = if macro_name == "print" {
                        "println".to_string()
                    } else {
                        macro_name.clone()
                    };
                    // Check if the first argument is a format! macro (from string interpolation)
                    if let Some((_, first_arg)) = arguments.first() {
                        // Check for MacroInvocation (explicit format! calls)
                        // first_arg is &&Expression (ref to ref from Vec element), deref both
                        if let Expression::MacroInvocation {
                            is_repeat: _,
                            ref name,
                            args: ref macro_args,
                            ..
                        } = **first_arg
                        {
                            if name == "format" && !macro_args.is_empty() {
                                // Unwrap the format! call and put its arguments directly into println!
                                // format!("text {}", var) -> println!("text {}", var)
                                let format_str = self.generate_expression(macro_args[0]);
                                let format_args: Vec<String> = macro_args[1..]
                                    .iter()
                                    .map(|arg| self.generate_expression(arg))
                                    .collect();

                                let args_str = if format_args.is_empty() {
                                    String::new()
                                } else {
                                    format!(", {}", format_args.join(", "))
                                };

                                return format!("{}!({}{})", target_macro, format_str, args_str);
                            }
                        }

                        // Check for Binary expression with string concatenation (will become format!)
                        if let Expression::Binary {
                            left,
                            op: BinaryOp::Add,
                            right,
                            ..
                        } = **first_arg
                        {
                            // Check if this is string concatenation
                            let has_string_literal =
                                matches!(
                                    left,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) || matches!(
                                    right,
                                    Expression::Literal {
                                        value: Literal::String(_),
                                        ..
                                    }
                                ) || string_analysis::contains_string_literal(left)
                                    || string_analysis::contains_string_literal(right);

                            if has_string_literal {
                                // Collect all parts of the concatenation
                                let mut parts = Vec::new();
                                string_analysis::collect_concat_parts_static(left, &mut parts);
                                string_analysis::collect_concat_parts_static(right, &mut parts);

                                // Generate format string and arguments
                                let format_str = "{}".repeat(parts.len());
                                let format_args: Vec<String> = parts
                                    .iter()
                                    .map(|expr| self.generate_expression(expr))
                                    .collect();

                                return format!(
                                    "{}!(\"{}\", {})",
                                    target_macro,
                                    format_str,
                                    format_args.join(", ")
                                );
                            }
                        }
                    }

                    // No interpolation, just regular print
                    // TDD FIX: Auto-format non-string arguments
                    // println(value) where value: bool → println!("{}", value)
                    // println("text") → println!("text") (string literals stay as-is)
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();

                    // Check if first argument is a string literal
                    let first_arg_is_string_literal = arguments
                        .first()
                        .map(|(_, arg)| {
                            matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            )
                        })
                        .unwrap_or(false);

                    if args.len() == 1 && !first_arg_is_string_literal {
                        // Single non-string argument - format it
                        return format!("{}!(\"{{}}\", {})", target_macro, args[0]);
                    } else {
                        // Multiple args or string literal - keep as-is
                        return format!("{}!({})", target_macro, args.join(", "));
                    }
                }

                // Special case: convert assert() to assert!()
                if func_name == "assert" {
                    let args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| self.generate_expression(arg))
                        .collect();
                    return format!("assert!({})", args.join(", "));
                }

                // TDD FIX: Call(FieldAccess) → method call WITH SIGNATURE LOOKUP
                // When the parser produces Call { function: FieldAccess { object, field }, args }
                // instead of MethodCall { object, method, args }, we need to:
                // 1. Handle it as a method call (not function call)
                // 2. Do signature lookup to get parameter ownership info
                // 3. Apply correct ownership conversions (& vs .clone() etc.)
                //
                // This was the AUTO-CLONE BUG: method calls skipped signature lookup!
                if let Expression::FieldAccess {
                    object: call_obj,
                    field: call_method,
                    ..
                } = &**function
                {
                    // DOUBLE-CLONE FIX: When the method is .clone(), suppress auto-clone on
                    // the object to prevent .clone().clone(). Same as MethodCall handler.
                    let prev_explicit_clone = self.in_explicit_clone_call;
                    if call_method == "clone" {
                        self.in_explicit_clone_call = true;
                    }
                    let mut obj_str = self.generate_expression(call_obj);
                    self.in_explicit_clone_call = prev_explicit_clone;
                    // DOUBLE-CLONE SAFETY NET: Strip redundant auto-clone from object
                    if call_method == "clone" && obj_str.ends_with(".clone()") {
                        obj_str = obj_str[..obj_str.len() - 8].to_string();
                    }

                    // TDD FIX: Lookup method signature for ownership inference
                    // Prefer `Type::method` (matches MethodCall path) so `HashMap::get` wins over wrong `get`.
                    let type_name = self.infer_type_name(call_obj);
                    let method_signature = type_name
                        .as_ref()
                        .map(|tn| format!("{}::{}", tn, call_method))
                        .and_then(|q| self.signature_registry.get_signature(&q).cloned())
                        .or_else(|| {
                            // When `call_obj` is a module identifier (e.g., `draw` in `draw::draw_text`),
                            // infer_type_name returns None. Try module-qualified lookup directly.
                            if let Expression::Identifier { name: mod_name, .. } = &**call_obj {
                                let qualified = format!("{}::{}", mod_name, call_method);
                                if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                                    return Some(sig.clone());
                                }
                            }
                            if super::stdlib_method_traits::is_common_stdlib_method(call_method) {
                                None
                            } else {
                                let bare_sig = self.signature_registry.get_signature(call_method).cloned();
                                bare_sig
                            }
                        });

                    // Generate arguments with ownership awareness (same logic as regular Call)
                    let args: Vec<String> = if let Some(ref sig) = method_signature {
                        arguments
                            .iter()
                            .enumerate()
                            .flat_map(|(i, (_label, arg))| {
                                let arg_to_generate =
                                    Self::strip_unary_ref_for_collection_key_arg(call_method, i, arg);
                                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                                self.coerce_string_literals_to_owned = false;
                                let prev_match_arm_str = self.in_match_arm_needing_string;
                                self.in_match_arm_needing_string = false;
                                let mut arg_str = self.generate_expression(arg_to_generate);
                                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                                self.in_match_arm_needing_string = prev_match_arm_str;

                                // Apply ownership conversion based on signature
                                let sig_param_idx = if sig.has_self_receiver {
                                    i + 1
                                } else {
                                    i
                                };
                                if let Some(&ownership) = sig.param_ownership.get(sig_param_idx) {
                                    match ownership {
                                        OwnershipMode::Borrowed => {
                                            // Destination wants borrowed — use same rules as MethodCall
                                            // (fixes `map.get(&key)` when `key: str` → Rust `&str`, E0277).
                                            let is_string_literal = matches!(
                                                arg_to_generate,
                                                Expression::Literal {
                                                    value: Literal::String(_),
                                                    ..
                                                }
                                            );
                                            let is_user_closure_param =
                                                if let Expression::Identifier { name, .. } =
                                                    arg_to_generate
                                                {
                                                    self.in_user_written_closure
                                                        && self.user_closure_params.contains(name)
                                                } else {
                                                    false
                                                };
                                            if !is_string_literal && !is_user_closure_param {
                                                let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                                    arg_to_generate,
                                                    &arg_str,
                                                    call_method.as_str(),
                                                    i,
                                                    &method_signature,
                                                    &self.usize_variables,
                                                    &self.current_function_params,
                                                    &self.borrowed_iterator_vars,
                                                    &self.inferred_borrowed_params,
                                                    arguments.len(),
                                                    type_name.as_deref(),
                                                    Some(&self.local_var_types),
                                                    Some(&self.stdlib_method_signatures),
                                                    Some(&self.method_signatures_by_type),
                                                );
                                                if should_ref {
                                                    arg_str = format!("&{}", arg_str);
                                                }
                                            }
                                        }
                                        OwnershipMode::MutBorrowed => {
                                            let is_already_mut_ref =
                                                if let Expression::Identifier { name, .. } = arg_to_generate {
                                                    let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                                        param.name == *name
                                                            && matches!(&param.type_, crate::parser::Type::MutableReference(_))
                                                    });
                                                    let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                                    explicit_mut_ref || inferred_mut_ref
                                                } else {
                                                    false
                                                };
                                            if !expression_helpers::is_reference_expression(arg_to_generate)
                                                && !is_already_mut_ref
                                            {
                                                let mut mut_arg_str = if arg_str.ends_with(".clone()") {
                                                    arg_str[..arg_str.len() - 8].to_string()
                                                } else {
                                                    arg_str
                                                };
                                                if mut_arg_str.starts_with("&") && !mut_arg_str.starts_with("&mut ") {
                                                    mut_arg_str = mut_arg_str[1..].to_string();
                                                }
                                                arg_str = format!("&mut {}", mut_arg_str);
                                            }
                                        }
                                        OwnershipMode::Owned => {
                                            // String literal coercion: "foo" → "foo".to_string()
                                            // when param expects owned String
                                            let is_str_lit = matches!(
                                                arg_to_generate,
                                                Expression::Literal { value: Literal::String(_), .. }
                                            );
                                            if is_str_lit {
                                                let is_explicit_str_ref = sig.param_types.get(sig_param_idx)
                                                    .is_some_and(|t| matches!(t, Type::Reference(inner) if
                                                        matches!(**inner, Type::String) ||
                                                        matches!(**inner, Type::Custom(ref s) if s == "str")
                                                    ));
                                                if !is_explicit_str_ref {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                }
                                            }
                                            // Destination wants owned - add .clone() for borrowed sources
                                            if let Expression::FieldAccess {
                                                object: field_obj,
                                                ..
                                            } = arg_to_generate
                                            {
                                                if let Expression::Identifier { name, .. } =
                                                    &**field_obj
                                                {
                                                    let is_borrowed =
                                                        self.borrowed_iterator_vars.contains(name)
                                                            || self
                                                                .inferred_borrowed_params
                                                                .contains(name);
                                                    if is_borrowed && !arg_str.ends_with(".clone()")
                                                    {
                                                        let is_copy = self
                                                            .infer_expression_type(arg_to_generate)
                                                            .as_ref()
                                                            .is_some_and(|t| self.is_type_copy(t));
                                                        if !is_copy {
                                                            arg_str =
                                                                format!("{}.clone()", arg_str);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }

                                // AUTO-CAST int → float: Call(FieldAccess) path
                                // Skip when signature has a collision (different types with same name).
                                let qualified_key = type_name.as_ref()
                                    .map(|tn| format!("{}::{}", tn, call_method));
                                let has_collision = qualified_key.as_ref()
                                    .is_some_and(|k| self.signature_registry.has_collision(k))
                                    || self.signature_registry.has_collision(call_method);
                                if !has_collision {
                                    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                                        let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                        let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                        if param_is_f32 || param_is_f64 {
                                            let arg_ty = self.infer_expression_type(arg);
                                            let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                                matches!(t, Type::Int)
                                                    || matches!(t, Type::Custom(n) if matches!(n.as_str(),
                                                        "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                                                    ))
                                            });
                                            if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                                let target = if param_is_f32 { "f32" } else { "f64" };
                                                arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                    format!("({}) as {}", arg_str, target)
                                                } else {
                                                    format!("{} as {}", arg_str, target)
                                                };
                                            }
                                        }
                                    }
                                }

                                vec![arg_str]
                            })
                            .collect()
                    } else {
                        // No signature: still apply map-key strip + stdlib `should_add_ref` (parser uses Call+FieldAccess)
                        // Try to find signature by qualified or simple method name for string coercion.
                        // CRITICAL: For common stdlib methods (get, remove, contains, etc.),
                        // do NOT fall back to unqualified lookup — it can match the WRONG
                        // user-defined method (e.g., ComponentArray::get when we want
                        // HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                        // This mirrors the guard in the MethodCall handler.
                        let fallback_sig = type_name
                            .as_ref()
                            .map(|tn| format!("{}::{}", tn, call_method))
                            .and_then(|q| self.signature_registry.get_signature(&q).cloned())
                            .or_else(|| {
                                if super::stdlib_method_traits::is_common_stdlib_method(call_method) {
                                    None
                                } else {
                                    self.signature_registry.get_signature(call_method).cloned()
                                }
                            });
                        arguments
                            .iter()
                            .enumerate()
                            .map(|(i, (_label, arg))| {
                                let arg_to_generate =
                                    Self::strip_unary_ref_for_collection_key_arg(call_method, i, arg);
                                let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                                self.coerce_string_literals_to_owned = false;
                                let prev_match_arm_str = self.in_match_arm_needing_string;
                                self.in_match_arm_needing_string = false;
                                let mut arg_str = self.generate_expression(arg_to_generate);
                                self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                                self.in_match_arm_needing_string = prev_match_arm_str;

                                let is_string_literal = matches!(
                                    arg_to_generate,
                                    Expression::Literal { value: Literal::String(_), .. }
                                );
                                if is_string_literal {
                                    let needs_to_string = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(
                                        i,
                                        call_method,
                                        &fallback_sig,
                                    );
                                    if needs_to_string {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    }
                                }

                                let should_ref =
                                    crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                        arg_to_generate,
                                        &arg_str,
                                        call_method.as_str(),
                                        i,
                                        &fallback_sig,
                                        &self.usize_variables,
                                        &self.current_function_params,
                                        &self.borrowed_iterator_vars,
                                        &self.inferred_borrowed_params,
                                        arguments.len(),
                                        type_name.as_deref(),
                                        Some(&self.local_var_types),
                                        Some(&self.stdlib_method_signatures),
                                        Some(&self.method_signatures_by_type),
                                    );
                                if should_ref {
                                    arg_str = format!("&{}", arg_str);
                                }
                                arg_str
                            })
                            .collect()
                    };

                    let call_str = format!("{}.{}({})", obj_str, call_method, args.join(", "));

                    let is_extern_call = method_signature
                        .as_ref()
                        .is_some_and(|sig| sig.is_extern)
                        || self.signature_registry
                            .get_signature(call_method)
                            .is_some_and(|sig| sig.is_extern)
                        || self.extern_function_names.contains(call_method);

                    return if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {} }})", call_str)
                    } else {
                        call_str
                    };
                }

                let mut func_str = self.generate_expression(function);

                // Windjammer stdlib type mapping: Map::method → HashMap::method
                if func_str.starts_with("Map::") {
                    func_str = func_str.replacen("Map::", "HashMap::", 1);
                }

                // In an impl block, bare function calls to sibling methods need qualified dispatch.
                // Instance methods (take self) → self.method(args)
                // Static methods → Self::method(args)
                if self.in_impl_block
                    && !func_name.contains("::")
                    && self.current_impl_methods.contains(&func_name)
                {
                    if self.current_impl_instance_methods.contains(&func_name) {
                        func_str = format!("self.{}", func_str);
                    } else {
                        func_str = format!("Self::{}", func_str);
                    }
                }

                // WINDJAMMER PHILOSOPHY: Some/Ok/Err with string literals need .to_string()
                // Some("literal") -> Some("literal".to_string())
                // Ok("literal") -> Ok("literal".to_string())
                // Err("literal") -> Err("literal".to_string())
                // Also: Some(borrowed_iterator_var) -> Some(borrowed_iterator_var.clone())

                // TDD FIX (Bug #2): Detect ALL enum constructors, not just Some/Ok/Err
                // Pattern: Module::Variant or Enum::Variant (both CamelCase)
                let is_std_enum = matches!(func_name.as_str(), "Some" | "Ok" | "Err");
                let is_custom_enum = func_name.contains("::") && {
                    let parts: Vec<&str> = func_name.split("::").collect();
                    parts.len() == 2
                        && parts[0].chars().next().is_some_and(|c| c.is_uppercase())
                        && parts[1].chars().next().is_some_and(|c| c.is_uppercase())
                };

                if is_std_enum || is_custom_enum {
                    // Enum variant constructors need owned values (Some(T), Ok(T), Err(E)).
                    // Set owned context so index expressions use .clone() instead of &,
                    // BUT only for arguments that aren't already explicit references.
                    let prev_owned_context = self.in_owned_value_context;
                    let generated_args: Vec<String> = arguments
                        .iter()
                        .map(|(_label, arg)| {
                            let is_explicit_ref = matches!(arg,
                                Expression::Unary { op: crate::parser::UnaryOp::Ref | crate::parser::UnaryOp::MutRef, .. }
                            );
                            if !is_explicit_ref {
                                self.in_owned_value_context = true;
                            }
                            let result = self.generate_expression(arg);
                            self.in_owned_value_context = prev_owned_context;
                            result
                        })
                        .collect();

                    let has_format_arg = generated_args
                        .iter()
                        .any(|arg_str| arg_str.contains("format!("));

                    if has_format_arg {
                        // Extract format!() macros to temp variables
                        let mut temp_decls = String::new();
                        let mut temp_counter = 0;
                        let fixed_args: Vec<String> = generated_args
                            .iter()
                            .map(|arg_str| {
                                if arg_str.starts_with("format!(")
                                    || arg_str.starts_with("&format!(")
                                {
                                    // Strip leading & if present
                                    let format_expr = if arg_str.starts_with("&") {
                                        arg_str.strip_prefix("&").unwrap()
                                    } else {
                                        arg_str
                                    };
                                    // Extract to temp var
                                    let temp_name = format!("_temp{}", temp_counter);
                                    temp_counter += 1;
                                    temp_decls.push_str(&format!(
                                        "let {} = {}; ",
                                        temp_name, format_expr
                                    ));

                                    // TDD FIX: Don't add & for owned parameters
                                    // Err(format!(...)) should be Err(_temp0), not Err(&_temp0)
                                    // Original arg didn't have &, so pass owned value
                                    if arg_str.starts_with("&") {
                                        format!("&{}", temp_name)
                                    } else {
                                        temp_name
                                    }
                                } else {
                                    arg_str.clone()
                                }
                            })
                            .collect();

                        return format!(
                            "{{ {}{}({}) }}",
                            temp_decls,
                            func_str,
                            fixed_args.join(", ")
                        );
                    }

                    let args: Vec<String> = generated_args
                        .iter()
                        .enumerate()
                        .map(|(i, arg_str)| {
                            // Get the original argument expression for type checking
                            let arg = &arguments[i].1;
                            let result = arg_str.clone();

                            // Auto-convert string literals to String for Option/Result wrappers
                            if matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                }
                            ) {
                                format!("{}.to_string()", result)
                            } else if let Expression::Identifier { name, .. } = arg {
                                // BUGFIX: Don't clone if function returns Option<&T>, Option<&mut T>, or Result<&T, E>
                                // When returning Option<&Squad>, Some(squad) should NOT become Some(squad.clone())

                                // Check if return type is Option<&T> or Option<&mut T> (reference inside)
                                let returns_option_ref = match &self.current_function_return_type {
                                    Some(Type::Option(inner_type)) => {
                                        matches!(
                                            **inner_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                    }
                                    _ => false,
                                };

                                // Check if return type is Result<&T, E> or Result<&mut T, E>
                                let returns_result_ref = match &self.current_function_return_type {
                                    Some(Type::Result(ok_type, _err_type)) => {
                                        matches!(
                                            **ok_type,
                                            Type::Reference(_) | Type::MutableReference(_)
                                        )
                                    }
                                    _ => false,
                                };

                                // AUTO-CLONE: When wrapping a borrowed iterator variable in Some/Ok/Err,
                                // we need to clone it since the wrapper takes ownership
                                // UNLESS we're returning Option<&T>, Option<&mut T>, Result<&T, E>, etc.
                                if !returns_option_ref
                                    && !returns_result_ref
                                    && self.borrowed_iterator_vars.contains(name)
                                    && !result.ends_with(".clone()")
                                {
                                    // Function returns owned, but variable is borrowed - need to clone
                                    format!("{}.clone()", result)
                                } else {
                                    // Function returns reference, or variable not borrowed - don't clone
                                    result
                                }
                            } else {
                                result
                            }
                        })
                        .collect();
                    return format!("{}({})", func_str, args.join(", "));
                }

                // Look up signature and clone it to avoid borrow conflicts
                // THE WINDJAMMER WAY: Try qualified name first, then simple name
                // e.g., "Sound::new" -> try "Sound::new", then "new"

                // TDD FIX: Function pointer signature extraction
                // When calling a function pointer parameter (e.g., has_item(arg1, arg2)),
                // extract the signature from the parameter's type instead of the registry
                let mut signature = if let Some(param) = self
                    .current_function_params
                    .iter()
                    .find(|p| p.name == func_name)
                {
                    // Check if this parameter is a function pointer
                    if let Type::FunctionPointer {
                        params,
                        return_type,
                    } = &param.type_
                    {
                        // TDD FIX: Build signature from function pointer type
                        // CRITICAL: Match the conversion logic in types.rs type_to_rust()!
                        // fn(string, i32) in Windjammer → fn(&String, i32) in Rust
                        //
                        // Conversion rules (from types.rs lines 148-160):
                        // - Type::String → "&String" → Borrowed
                        // - Type::Custom("string") → "&String" → Borrowed
                        // - Type::Reference(_) → "&T" → Borrowed
                        // - Copy types (Int, Bool, etc.) → owned → Owned
                        // - Everything else → as-is (keep explicit types)
                        let param_ownership: Vec<OwnershipMode> = params
                            .iter()
                            .map(|ty| {
                                match ty {
                                    // Idiomatic Windjammer: string parameters are borrowed (types.rs:151)
                                    Type::String => OwnershipMode::Borrowed,
                                    Type::Custom(name) if name == "string" => {
                                        OwnershipMode::Borrowed
                                    }
                                    // Explicit references - borrowed (types.rs:154)
                                    Type::Reference(_) | Type::MutableReference(_) => {
                                        OwnershipMode::Borrowed
                                    }
                                    // Copy types - owned (types.rs:156-157)
                                    Type::Int
                                    | Type::Int32
                                    | Type::Uint
                                    | Type::Float
                                    | Type::Bool => OwnershipMode::Owned,
                                    Type::Custom(name)
                                        if matches!(
                                            name.as_str(),
                                            "i32"
                                                | "i64"
                                                | "u32"
                                                | "u64"
                                                | "f32"
                                                | "f64"
                                                | "bool"
                                                | "char"
                                                | "usize"
                                                | "isize"
                                        ) =>
                                    {
                                        OwnershipMode::Owned
                                    }
                                    // Everything else - keep as-is (types.rs:159)
                                    // For non-Copy custom types, default is as-is, which means Owned in this context
                                    // (the analyzer will have determined the correct type already)
                                    _ => OwnershipMode::Owned,
                                }
                            })
                            .collect();

                        Some(crate::analyzer::FunctionSignature {
                            name: func_name.clone(),
                            param_types: params.clone(),
                            param_ownership,
                            return_type: return_type.as_ref().map(|t| (**t).clone()),
                            return_ownership: OwnershipMode::Owned, // Functions return owned by default
                            has_self_receiver: false,
                            is_extern: false,
                        })
                    } else {
                        // Not a function pointer - try registry
                        self.signature_registry.get_signature(&func_name).cloned()
                    }
                } else {
                    // Not a parameter - try registry lookup
                    let direct = self.signature_registry.get_signature(&func_name).cloned();
                    direct.or_else(|| {
                            if let Some(pos) = func_name.rfind("::") {
                                let qualifier = &func_name[..pos];
                                let simple_name = &func_name[pos + 2..];
                                let is_type_qualifier = qualifier
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_uppercase());
                                if is_type_qualifier {
                                    self.signature_registry
                                        .get_signature(simple_name)
                                        .cloned()
                                } else {
                                    // For module-qualified calls (e.g., draw::draw_text),
                                    // try progressively shorter qualified names.
                                    // Do NOT fall back to simple name - it may collide
                                    // with a different module's function with the same name.
                                    let parts: Vec<&str> = func_name.split("::").collect();
                                    let mut found = None;
                                    for start in (0..parts.len().saturating_sub(1)).rev() {
                                        let candidate = parts[start..].join("::");
                                        if let Some(sig) = self.signature_registry.get_signature(&candidate) {
                                            found = Some(sig.clone());
                                            break;
                                        }
                                    }
                                    found
                                }
                            } else {
                                None
                            }
                        })
                };

                // For module-qualified calls (e.g., gpu::load_compute_shader_from_file),
                // the signature lookup above may fail. Try resolving through module aliases
                // first (e.g., `use crate::ffi::gpu_safe as gpu` → try gpu_safe::func),
                // then fall back to the simple name.
                let mut signature_from_simple_fallback = false;
                if signature.is_none() && func_name.contains("::") {
                    let qualifier = func_name.split("::").next().unwrap_or("");
                    let simple = func_name.rsplit("::").next().unwrap_or(&func_name);

                    // Try resolving through module alias map first
                    if let Some(original_module) = self.module_alias_map.get(qualifier) {
                        let resolved_name = format!("{}::{}", original_module, simple);
                        if let Some(resolved_sig) = self.signature_registry.get_signature(&resolved_name) {
                            signature = Some(resolved_sig.clone());
                            // NOT a simple fallback — we resolved through the alias
                        }
                    }

                    // If alias resolution didn't work, try simple-name fallback
                    if signature.is_none() {
                        if let Some(fallback) = self.signature_registry.get_signature(simple) {
                            signature = Some(fallback.clone());
                            signature_from_simple_fallback = true;
                        }
                    }
                }

                // Check if this is an extern function call for unsafe wrapping + FFI str handling.
                let is_extern_call = if let Some(ref sig) = signature {
                    sig.is_extern
                } else {
                    let simple = func_name.rsplit("::").next().unwrap_or(&func_name);
                    self.extern_function_names.contains(simple)
                };

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .flat_map(|(i, (_label, arg))| {
                        // CRITICAL: Reset in_field_access_object for argument generation.
                        // Arguments are independent expressions, NOT part of a field/method/index chain.
                        // Without this, `process_property(prop.name, prop.value).as_str()` would
                        // leak in_field_access_object from the MethodCall handler into prop.name/prop.value,
                        // suppressing necessary .clone() calls.
                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;

                        // TDD FIX: Set call argument context to suppress premature .clone()
                        // The FieldAccess handler normally adds .clone() for borrowed iterator vars,
                        // but in call arguments, we need to let the ownership check below decide
                        let prev_in_call_arg = self.in_call_argument_generation;
                        self.in_call_argument_generation = true;

                        // Return/match contexts set `coerce_string_literals_to_owned` and
                        // `in_match_arm_needing_string` for the outer expression; nested call
                        // arguments must use only parameter-type conversion (below), not context
                        // coercion — avoids `"x".to_string().to_string()` and wrong `.to_string()`
                        // on &str params, and prevents format!("...".to_string(), ...) in match arms.
                        let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                        self.coerce_string_literals_to_owned = false;
                        let prev_match_arm_str = self.in_match_arm_needing_string;
                        self.in_match_arm_needing_string = false;
                        let mut arg_str = self.generate_expression(arg);
                        self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                        self.in_match_arm_needing_string = prev_match_arm_str;

                        self.in_call_argument_generation = prev_in_call_arg;
                        self.in_field_access_object = prev_field_access_obj;
                        
                        // TDD FIX: Cast int arguments to usize for stdlib methods
                        // Vec::with_capacity(size) where size: int → Vec::with_capacity(size as usize)
                        // Vec::with_capacity(10) where 10: int literal → Vec::with_capacity(10_usize)
                        if i == 0 && (func_name == "Vec::with_capacity" || func_name == "HashMap::with_capacity" ||
                                      func_name == "String::with_capacity" || func_name == "Vec::reserve") {
                            match arg {
                                Expression::Identifier { .. } => {
                                    // Variables: add explicit cast
                                    arg_str = format!("{} as usize", arg_str);
                                }
                                Expression::Literal { value: Literal::Int(val), .. } => {
                                    // Literals: use usize suffix
                                    arg_str = format!("{}_usize", val);
                                }
                                _ => {
                                    // Other expressions (e.g., calculations): wrap in (expr) as usize
                                    if !arg_str.ends_with("_usize") && !arg_str.contains(" as usize") {
                                        arg_str = format!("({}) as usize", arg_str);
                                    }
                                }
                            }
                        }

                        // WINDJAMMER FFI: Convert string arguments for extern functions
                        if is_extern_call {
                            if let Some(ref sig) = signature {
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, Type::Custom(name) if name == "str") {
                                        // Expand str to (ptr, len)
                                        return vec![
                                            format!("{}.as_bytes().as_ptr()", arg_str),
                                            format!("{}.as_bytes().len()", arg_str),
                                        ];
                                    }
                                    // string/String params → FfiString via string_to_ffi
                                    // TDD FIX: Always use .to_string() - infer_expression_type returns
                                    // declared param type (Type::String), not actual Rust type. When
                                    // ownership infers Borrowed, param becomes &str in Rust, but we
                                    // thought it was String and passed directly → E0308.
                                    // .to_string() works for both &str and String (String::to_string = clone).
                                    //
                                    // TDD FIX: Strip redundant .to_string() before wrapping.
                                    // Bug: User writes render_text(label.to_string(), x, y). Expression
                                    // generation produces "label.to_string()", then we added another
                                    // → string_to_ffi(label.to_string().to_string()). Fix: If arg_str
                                    // already ends with .to_string(), don't add another.
                                    if matches!(param_type, Type::String)
                                        || matches!(param_type, Type::Custom(n) if n == "string" || n == "String")
                                    {
                                        let inner = if arg_str.ends_with(".to_string()") {
                                            arg_str.clone()
                                        } else {
                                            format!("{}.to_string()", arg_str)
                                        };
                                        return vec![format!(
                                            "windjammer_runtime::ffi::string_to_ffi({})",
                                            inner
                                        )];
                                    }
                                }
                            }
                        }

                        // Auto-convert string literals to String for functions expecting owned String
                        // THE WINDJAMMER WAY: Smart inference based on available information!
                        if matches!(
                            arg,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) {
                            // Check if the parameter expects an owned String
                            let should_convert = if let Some(ref sig) = signature {
                                if sig.is_extern {
                                    // Extern functions have explicit types; ownership inference
                                    // is meaningless (empty body defaults to Borrowed).
                                    // Convert if parameter type is String.
                                    sig.param_types.get(i).is_some_and(|ty| {
                                        matches!(ty, Type::String)
                                            || matches!(ty, Type::Custom(name) if name == "string" || name == "String")
                                    })
                                } else if signature_from_simple_fallback && {
                                    let qualifier = func_name.split("::").next().unwrap_or("");
                                    qualifier.chars().next().is_some_and(|c| c.is_lowercase())
                                } {
                                    // Fallback-resolved from module::function: the signature may
                                    // be from a different module. Don't trust ownership for
                                    // string coercion — the actual target may take &str.
                                    false
                                } else if let Some(&ownership) = sig.param_ownership.get(i) {
                                    // Convert if parameter expects owned String
                                    matches!(ownership, OwnershipMode::Owned)
                                } else {
                                    // No ownership info for this param
                                    // THE WINDJAMMER WAY: Heuristic for constructors
                                    // Functions named 'new' (or Type::new) taking string params likely expect String
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            } else {
                                // No signature found — check enum variant registry
                                // WINDJAMMER FIX: Enum variant constructors like GameEvent::ItemPickup("text")
                                // need .to_string() when the variant field is String type
                                if let Some(variant_types) = self.enum_variant_types.get(&func_name) {
                                    // TDD FIX: Check for both Type::String and Type::Custom("String")
                                    variant_types.get(i).is_some_and(|ty| {
                                        matches!(ty, Type::String)
                                            || matches!(ty, Type::Custom(name) if name == "String")
                                    })
                                } else {
                                    // Fallback heuristic for constructors
                                    func_name == "new" || func_name.ends_with("::new")
                                }
                            };

                            if should_convert {
                                arg_str = format!("{}.to_string()", arg_str);
                            }
                        }

                        // Check if this parameter expects a borrow
                        // Skip ownership inference for extern function calls - they have explicit types
                        if let Some(ref sig) = signature {
                            if sig.is_extern {
                                // Auto-convert mut locals to &mut when FFI param is *mut T
                                // This eliminates Rust leakage: users write `ffi_fn(x)` not `ffi_fn(&mut x)`
                                if let Some(param_type) = sig.param_types.get(i) {
                                    if matches!(param_type, crate::parser::ast::types::Type::RawPointer { mutable: true, .. }) {
                                        return vec![format!("&mut {}", arg_str)];
                                    }
                                }
                                return vec![arg_str];
                            }

                            // COLLISION GUARD: When the signature was resolved via a
                            // simple-name fallback from a module-qualified call AND the
                            // simple name has a collision, skip auto-borrow/auto-mutborrow.
                            // The looked-up signature may be from the wrong module,
                            // so applying its ownership blindly can produce incorrect
                            // `&` or `&mut` prefixes.
                            //
                            // We only guard fallback-resolved signatures because:
                            // - Direct qualified lookups are unambiguous (right signature)
                            // - Bare-name calls within the same file are also unambiguous
                            // - Only fallback from module::fn → fn is risky (wrong module)
                            let simple_name = func_name.rsplit("::").next().unwrap_or(&func_name);
                            let has_ownership_collision = signature_from_simple_fallback
                                && (self.signature_registry.has_collision(&func_name)
                                    || self.signature_registry.has_collision(simple_name));

                            if let Some(&ownership) = sig.param_ownership.get(i) {
                                match ownership {
                                    OwnershipMode::Borrowed if !has_ownership_collision => {
                                        // NEW DESIGN: Borrowed string parameters → &str (not &String!)
                                        // String literals are already &str in Rust, so they can be passed directly.
                                        // No conversion needed: "literal" → &str parameter is a perfect match
                                        let is_string_literal = matches!(
                                            arg,
                                            Expression::Literal {
                                                value: Literal::String(_),
                                                ..
                                            }
                                        );

                                        if is_string_literal {
                                            // String literals are already &str, pass directly to &str parameter
                                            // No & needed, no .to_string() needed
                                            return vec![arg_str];
                                        }

                                        // TDD FIX: Check if parameter is already a reference type
                                        // If param is &string, don't add another & (would be &&string)
                                        let is_param_already_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::Reference(_)
                                                                | Type::MutableReference(_)
                                                        )
                                                })
                                            } else {
                                                false
                                            };

                                        // TDD FIX: Don't add & for Copy type parameters
                                        // When signature says Borrowed but param type is Copy,
                                        // codegen keeps it as owned (e.g., x: usize not x: &usize)
                                        // So the call site should NOT add &
                                        // BUT: Reference types (&Vec<T>, &[T]) are NOT treated as
                                        // Copy here - if param type is &T, caller still needs &
                                        let is_copy_param = sig.param_types.get(i)
                                            .map(|t| {
                                                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                                                    && crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::is_copy_type_annotation_pub(t)
                                            })
                                            .unwrap_or(false);

                                        // TDD FIX (Bug #16): Don't add & to temp variables!
                                        // Temp variables (like _temp0) hold OWNED values from format!()
                                        // format!() returns String, not &str, so _temp0 is String
                                        // If we add &, we get &String when we need String
                                        let is_temp_variable = arg_str.starts_with("_temp")
                                            && arg_str.chars().skip(5).all(|c| c.is_numeric());

                                        // TDD FIX: IDIOMATIC WINDJAMMER - Strip .clone() if present!
                                        // When destination wants Borrowed, pass &field, NOT &field.clone()
                                        // Example: has_item(ingredient.item_id) with has_item(item_id: string)
                                        // Should generate: has_item(&ingredient.item_id)
                                        // NOT: has_item(&ingredient.item_id.clone())
                                        // The .clone() may have been added by generate_expression for borrowed iterator vars
                                        if arg_str.ends_with(".clone()") {
                                            arg_str = arg_str[..arg_str.len() - 8].to_string();
                                        }

                                        // Insert & if not already a reference and not a string literal and not a temp var
                                        // THE WINDJAMMER WAY: Preserve user-written closure params
                                        let is_user_closure_param = if let Expression::Identifier { name, .. } = arg {
                                            self.in_user_written_closure && self.user_closure_params.contains(name)
                                        } else {
                                            false
                                        };

                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_param_already_ref
                                            && !is_copy_param
                                            && !is_temp_variable
                                            && !is_user_closure_param
                                        {
                                            return vec![format!("&{}", arg_str)];
                                        } else {
                                            return vec![arg_str];
                                        }
                                    }
                                    OwnershipMode::MutBorrowed if !has_ownership_collision => {
                                        // TDD FIX: Don't add &mut if arg is already a &mut parameter
                                        // Covers both explicitly declared &mut params AND
                                        // params inferred as &mut through ownership analysis
                                        let is_already_mut_ref =
                                            if let Expression::Identifier { name, .. } = arg {
                                                // Check 1: Explicit &mut in AST type
                                                let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                                    param.name == *name
                                                        && matches!(
                                                            &param.type_,
                                                            Type::MutableReference(_)
                                                        )
                                                });
                                                // Check 2: Inferred &mut through ownership analysis
                                                let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                                explicit_mut_ref || inferred_mut_ref
                                            } else {
                                                false
                                            };

                                        // Insert &mut if not already a reference
                                        if !expression_helpers::is_reference_expression(arg)
                                            && !is_already_mut_ref
                                        {
                                            // CRITICAL FIX: Remove .clone() if present - we want to mutate the original!
                                            // &mut counter.clone() → &mut counter
                                            // When passing &mut, we're giving mutable access to the original,
                                            // not a clone. The .clone() would break mutation semantics.
                                            let mut_arg_str = if arg_str.ends_with(".clone()") {
                                                arg_str[..arg_str.len() - 8].to_string()
                                            } else {
                                                arg_str
                                            };
                                            return vec![format!("&mut {}", mut_arg_str)];
                                        }
                                    }
                                    OwnershipMode::Owned => {
                                        // TDD FIX: AUTO-CONVERT for &str/&String → String, &T → T
                                        // When passing a reference to a function expecting owned, convert it
                                        // - &str → String: use .to_string()
                                        // - &String → String: use .clone()
                                        // - &T → T: use .clone()
                                        if let Expression::Identifier { name, .. } = arg {
                                            // Find the parameter type
                                            let param_type = self
                                                .current_function_params
                                                .iter()
                                                .find(|p| &p.name == name)
                                                .map(|p| &p.type_);

                                            // Check if it's a reference parameter (&str, &String, &T)
                                            if let Some(Type::Reference(inner_type)) = param_type {
                                                // Special case: &str (Type::Reference(Type::String) in Rust parlance)
                                                // &str.clone() → &str, but we need String, so use .to_string()
                                                if matches!(**inner_type, Type::String)
                                                    && !arg_str.ends_with(".to_string()")
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    arg_str = format!("{}.to_string()", arg_str);
                                                } else if !arg_str.ends_with(".clone()") {
                                                    // For other reference types, .clone() works
                                                    arg_str = format!("{}.clone()", arg_str);
                                                }
                                            } else {
                                                // TDD FIX: Check if it's from a borrowed iterator (for loop)
                                                // Example: for npc_id in npc_ids { Member::new(npc_id) }
                                                // npc_id is &String from iterator, needs .clone() for owned String
                                                //
                                                // CRITICAL: We're in OwnershipMode::Owned block, which means
                                                // the DESTINATION parameter wants an owned value (String, not &String).
                                                //
                                                // Windjammer `string` parameters lower to `&str`: `.clone()` keeps
                                                // `&str` (E0308). Use `.to_string()` for text types instead.
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(name);

                                                // Also check if it's inferred as borrowed
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(name);

                                                if (is_borrowed_iterator_var
                                                    || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    let is_text = self
                                                        .infer_expression_type(arg)
                                                        .as_ref()
                                                        .is_some_and(|t| {
                                                            crate::codegen::rust::types::is_windjammer_text_type(t)
                                                        });
                                                    if is_text {
                                                        arg_str = format!("{}.to_string()", arg_str);
                                                    } else {
                                                        // Borrowed from iterator or inferred - use .clone()
                                                        // This handles &T → T for non-text types
                                                        arg_str = format!("{}.clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }

                                        // TDD FIX: AUTO-CLONE for borrowed_param.field
                                        // When passing ingredient.item_id where ingredient is borrowed,
                                        // we need to clone() IF destination wants Owned.
                                        //
                                        // We're ALREADY in OwnershipMode::Owned block,
                                        // so destination wants owned. Safe to add .clone().
                                        //
                                        // This handles: for ingredient in &vec { func(ingredient.field) }
                                        // where func(field: String) expects owned.
                                        if let Expression::FieldAccess { .. } = arg {
                                            // Trace through nested field accesses to find the root identifier
                                            // Handles: stack.field, stack.item.id, stack.item.nested.deep
                                            let root_name = self.extract_root_identifier(arg);
                                            if let Some(name) = root_name {
                                                let is_borrowed_iterator_var =
                                                    self.borrowed_iterator_vars.contains(&name);
                                                let is_explicitly_borrowed =
                                                    self.current_function_params.iter().any(|p| {
                                                        p.name == name
                                                            && matches!(
                                                                p.ownership,
                                                                crate::parser::OwnershipHint::Ref
                                                            )
                                                    });
                                                let is_inferred_borrowed =
                                                    self.inferred_borrowed_params.contains(&name);

                                                if (is_borrowed_iterator_var
                                                    || is_explicitly_borrowed
                                                    || is_inferred_borrowed)
                                                    && !arg_str.ends_with(".clone()")
                                                {
                                                    let is_copy = self.infer_expression_type(arg)
                                                        .as_ref()
                                                        .is_some_and(|t| self.is_type_copy(t));
                                                    if !is_copy {
                                                        arg_str = format!("{}.clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }
                                        // DOGFOODING FIX: Vec indexing &vec[idx] passed to owned param
                                        // e.g. enterable.push(self.buildings[i]) → need (.clone())
                                        if let Expression::Index { .. } = arg {
                                            if arg_str.starts_with("&")
                                                && !arg_str.ends_with(".clone()")
                                            {
                                                if let Some(inner) = self.infer_expression_type(arg)
                                                {
                                                    if !self.is_type_copy(&inner) {
                                                        arg_str =
                                                            format!("({}).clone()", arg_str);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // Collision guard triggered: Borrowed or MutBorrowed
                                        // with a signature collision. Don't apply auto-borrow;
                                        // pass the argument as-is and let downstream Rust
                                        // compilation determine the correct behavior.
                                    }
                                }
                            }
                        } else {
                            // No signature found - don't auto-clone!
                            // Without signature info, we can't know if destination wants Owned or Borrowed
                            // Better to let Rust compiler catch the error than guess wrong
                        }

                        // AUTO-CAST int → float: regular Call path
                        // Skip when the signature key has a collision (different types registered
                        // the same function name with different param types). The auto-cast
                        // cannot be trusted when the looked-up signature may be from a different
                        // type in another module.
                        if let Some(ref sig) = signature {
                            let has_collision = self.signature_registry.has_collision(&func_name)
                                || self.signature_registry.has_collision(&func_str);
                            if !has_collision {
                                if let Some(param_ty) = sig.param_types.get(i) {
                                    let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                    let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                    if param_is_f32 || param_is_f64 {
                                        let arg_ty = self.infer_expression_type(arg);
                                        let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                            matches!(t, Type::Int)
                                                || matches!(t, Type::Custom(n) if matches!(n.as_str(),
                                                    "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                                                ))
                                        });
                                        if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                            let target = if param_is_f32 { "f32" } else { "f64" };
                                            arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                format!("({}) as {}", arg_str, target)
                                            } else {
                                                format!("{} as {}", arg_str, target)
                                            };
                                        }
                                    }
                                }
                            }
                        }

                        vec![arg_str]
                    })
                    .collect();

                // TDD FIX (Bug #3): Extract format!() macros in arguments to temp variables
                // The args vec has already been generated as Rust strings
                // Check if any contain format!() and extract them
                let has_format_arg = args.iter().any(|arg_str| arg_str.contains("format!("));

                // WINDJAMMER FFI: Extern functions returning string use FfiString - wrap with ffi_to_string
                let returns_string = signature
                    .as_ref()
                    .and_then(|s| s.return_type.as_ref())
                    .is_some_and(|t| {
                        matches!(t, Type::String)
                            || matches!(t, Type::Custom(n) if n == "string" || n == "String")
                    });

                // WINDJAMMER PHILOSOPHY: Auto-wrap extern function calls in unsafe blocks
                // THE WINDJAMMER WAY: Users shouldn't have to write `unsafe` manually
                let call_result = if has_format_arg {
                    // Extract format!() macros to temp variables
                    let mut temp_decls = String::new();
                    let mut temp_counter = 0;
                    let fixed_args: Vec<String> = args
                        .iter()
                        .map(|arg_str| {
                            if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                // TDD FIX (Bug #16 COMPLETE): Check if original had & to preserve intent
                                let has_borrow_prefix = arg_str.starts_with("&");
                                // Strip leading & if present
                                let format_expr = if has_borrow_prefix {
                                    &arg_str[1..]
                                } else {
                                    arg_str
                                };
                                // Extract to temp var
                                let temp_name = format!("_temp{}", temp_counter);
                                temp_counter += 1;
                                temp_decls
                                    .push_str(&format!("let {} = {}; ", temp_name, format_expr));

                                // TDD FIX: Only add & if original had it!
                                // format!() returns owned String, so if caller wants owned, pass temp directly
                                // If caller wants borrowed, pass &temp (when original was &format!())
                                if has_borrow_prefix {
                                    format!("&{}", temp_name)
                                } else {
                                    temp_name
                                }
                            } else {
                                arg_str.clone()
                            }
                        })
                        .collect();

                    let call_expr = format!("{}({})", func_str, fixed_args.join(", "));

                    // Wrap in unsafe block if extern, otherwise regular block
                    // Parenthesize so the block can be used as a sub-expression (e.g., in comparisons)
                    if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {}{}  }})", temp_decls, call_expr)
                    } else {
                        format!("{{ {}{} }}", temp_decls, call_expr)
                    }
                } else {
                    // No format!() args - generate normally with optional unsafe wrapper
                    let call_str = format!("{}({})", func_str, args.join(", "));
                    if is_extern_call && !self.in_unsafe_block {
                        format!("(unsafe {{ {} }})", call_str)
                    } else {
                        call_str
                    }
                };

                // Wrap extern string return with ffi_to_string
                if is_extern_call && returns_string {
                    format!("windjammer_runtime::ffi::ffi_to_string({})", call_result)
                } else {
                    call_result
                }
            }
            Expression::MethodCall {
                object,
                method,
                type_args,
                arguments,
                ..
            } => {
                // TDD FIX: Strip redundant .as_str() on &str parameters
                // If method is .as_str() and object is already inferred as &str, just return object
                if method == "as_str" && arguments.is_empty() {
                    if let Expression::Identifier { name, .. } = object {
                        let is_borrowed = self.inferred_borrowed_params.contains(name.as_str());
                        if is_borrowed {
                            // Parameter is already &str, .as_str() is redundant
                            return self.generate_expression(object);
                        }
                    }
                }

                // METHOD CALL CONTEXT: Suppress Vec index auto-clone when generating the
                // object of a method call. Methods take &self or &mut self, so Rust allows
                // calling methods on &T returned by Vec indexing without cloning.
                // e.g., self.lights[i].is_enabled() → no need to clone the whole Light2D
                let prev_field_access = self.in_field_access_object;
                self.in_field_access_object = true;
                // DOUBLE-CLONE FIX: When the source has explicit .clone(), suppress auto-clone
                // on the object to prevent .clone().clone(). The explicit clone IS the clone.
                let prev_explicit_clone = self.in_explicit_clone_call;
                if method == "clone" {
                    self.in_explicit_clone_call = true;
                }
                let mut obj_str = self.generate_expression_with_precedence(object);
                self.in_field_access_object = prev_field_access;
                self.in_explicit_clone_call = prev_explicit_clone;
                // E0507: `collection[i].method(args)` when the method consumes `self` (owned receiver)
                // must clone the element: `self.tracks[i].clone().sample(t)` (otherwise move out of &Vec).
                if matches!(&**object, Expression::Index { .. }) {
                    if let Some(recv_ty) = self.infer_expression_type(object) {
                        if !self.is_type_copy(&recv_ty) {
                            if let Some(tn) = Self::type_to_name(&recv_ty) {
                                let qualified = format!("{}::{}", tn, method);
                                let sig_opt = self
                                    .signature_registry
                                    .get_signature(&qualified)
                                    .or_else(|| self.signature_registry.get_signature(method));
                                if let Some(sig) = sig_opt {
                                    if sig.has_self_receiver
                                        && sig.param_ownership.first()
                                            == Some(&crate::analyzer::OwnershipMode::Owned)
                                        && !obj_str.ends_with(".clone()")
                                    {
                                        obj_str = format!("{}.clone()", obj_str);
                                    }
                                }
                            }
                        }
                    }
                }

                // DOUBLE-CLONE SAFETY NET: If the object was auto-cloned by the FieldAccess
                // handler and this IS a .clone() call, strip the redundant auto-clone.
                // e.g., "stack.item.clone()" from auto-clone + ".clone()" from source
                //     → should be "stack.item.clone()", not "stack.item.clone().clone()"
                if method == "clone" && obj_str.ends_with(".clone()") {
                    obj_str = obj_str[..obj_str.len() - 8].to_string();
                }

                // TDD FIX: Option::unwrap() move error prevention
                // TDD FIX: AUTO-CLONE Option::unwrap() on borrowed fields
                // When calling .unwrap() on a borrowed Option field, we must clone before unwrap:
                //   node.children.unwrap() where node is &Node → ERROR: cannot move from &Option
                //   node.children.clone().unwrap() → ✅ OK
                // THE WINDJAMMER WAY: Users write .unwrap() naturally, compiler handles ownership
                if method == "unwrap" {
                    // Check if object is a field access (node.children) that needs clone
                    let needs_clone = if let Expression::FieldAccess {
                        object: field_obj, ..
                    } = object
                    {
                        // Is this accessing a field on a borrowed parameter?
                        if let Expression::Identifier { ref name, .. } = **field_obj {
                            // Check if the identifier is an inferred borrowed parameter
                            self.inferred_borrowed_params.contains(name)
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if needs_clone && !obj_str.contains(".clone()") {
                        obj_str = format!("{}.clone()", obj_str);
                    }
                }

                // E0507 fix: Option::map on self.field with &self must use .as_ref().map(...)
                // self.children.map(|c| ...) with &self → self.children.as_ref().map(|c| ...)
                if method == "map"
                    && self.inferred_borrowed_params.contains("self")
                    && self.codegen_expression_traces_to_self(object)
                {
                    if !obj_str.contains(".as_ref()") {
                        obj_str = format!("{}.as_ref()", obj_str);
                    }
                }

                // BUG #8 FIX: Look up method signature with qualified name (Type::method)
                // First try to infer the type from the object expression
                let type_name = self.infer_type_name(object);
                let method_signature = if let Some(ref type_name) = type_name {
                    let qualified_name = format!("{}::{}", type_name, method);
                    self.signature_registry
                        .get_signature(&qualified_name)
                        .cloned()
                    // CRITICAL: Do NOT fall back to unqualified method name lookup!
                    // Unqualified lookup for common names like "get", "remove", "contains"
                    // can match WRONG user-defined methods (e.g., ComponentArray::get when
                    // we want HashMap::get), causing incorrect auto-ref/auto-clone behavior.
                    // When the qualified name isn't found, method_signature stays None and
                    // the stdlib heuristics in should_add_ref handle common patterns correctly.
                } else {
                    if super::stdlib_method_traits::is_common_stdlib_method(method) {
                        None // Use stdlib heuristics instead of potentially wrong signature
                    } else {
                        self.signature_registry.get_signature(method).cloned()
                    }
                };

                // Float method argument context: for methods like clamp/max/min on float
                // receivers, arguments should use the same float type as the receiver.
                let prev_float_target = self.assignment_float_target_type.clone();
                let receiver_float_type = self.infer_expression_type(object);
                let is_float_method = matches!(
                    method.as_str(),
                    "clamp" | "max" | "min" | "abs" | "copysign" | "recip"
                        | "to_degrees" | "to_radians" | "signum" | "powf" | "powi"
                        | "sqrt" | "cbrt" | "hypot" | "sin" | "cos" | "tan"
                        | "asin" | "acos" | "atan" | "atan2" | "exp" | "exp2"
                        | "ln" | "log" | "log2" | "log10" | "round" | "floor"
                        | "ceil" | "trunc" | "fract"
                );
                if is_float_method {
                    if let Some(ref rft) = receiver_float_type {
                        match rft {
                            Type::Custom(n) if n == "f64" => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f64".to_string()));
                            }
                            Type::Custom(n) if n == "f32" => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f32".to_string()));
                            }
                            Type::Float => {
                                self.assignment_float_target_type =
                                    Some(Type::Custom("f64".to_string()));
                            }
                            _ => {}
                        }
                    }
                }

                let args: Vec<String> = arguments
                    .iter()
                    .enumerate()
                    .map(|(i, (_label, arg))| {
                        // TDD FIX: Suppress auto-clone for FieldAccess when method expects Borrowed
                        // Bug: ingredient.item_id generates .clone(), then & is added -> &cloned_value
                        // Fix: Suppress clone when param expects Borrowed -> just add & to field
                        let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                        let param_expects_borrowed = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                            .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Borrowed));

                        const AUTO_BORROW_METHODS: &[&str] = &["push_str", "extend_from_slice"];
                        let is_auto_borrow_target = AUTO_BORROW_METHODS.contains(&method.as_str()) && i == 0;

                        let prev_suppress = self.suppress_borrowed_clone;
                        if (param_expects_borrowed || is_auto_borrow_target)
                            && matches!(arg, Expression::FieldAccess { .. } | Expression::Identifier { .. })
                        {
                            self.suppress_borrowed_clone = true;
                        }

                        // CRITICAL: Reset in_field_access_object for method argument generation.
                        // Same rationale as function call arguments — method arguments are
                        // independent expressions, not part of a field/method/index chain.
                        // TDD FIX: STRIP explicit &ref when parameter expects owned value.
                        // WINDJAMMER PHILOSOPHY: The developer shouldn't need to think about &.
                        // If the user writes `&object.transform` but the method takes `Transform` (owned),
                        // the compiler strips the & and passes by value (Copy types) or moves.
                        // Example: self.render_transform(&object.transform) → self.render_transform(object.transform)
                        //
                        // TDD FIX: ALSO strip explicit & for HashMap/BTreeMap key methods with &String arguments.
                        // HashMap<String, V>.contains_key() expects &str, not &&String.
                        // User writes: map.contains_key(&key) where key is inferred as &String
                        // Compiler generates: map.contains_key(key) which auto-derefs &String to &str ✅
                        let arg_to_generate = if let Expression::Unary {
                            op: crate::parser::UnaryOp::Ref,
                            operand,
                            ..
                        } = arg
                        {
                            let is_hashmap_key_method =
                                super::stdlib_method_traits::is_map_key_method(method) && i == 0;

                            if is_hashmap_key_method {
                                // Strip explicit `&ident` for map keys: `should_add_ref` will add `&` back when the
                                // Rust type is owned or a Copy `K` that still needs `&K`. For `key: &str` / `&String`
                                // parameters, `should_add_ref` stays false → we emit `get(key)` not `get(&key)` (E0277).
                                if let Expression::Identifier { .. } = &**operand {
                                    operand
                                } else {
                                    arg
                                }
                            } else if let Some(ref sig) = method_signature {
                                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                                let param_is_owned = sig
                                    .param_ownership
                                    .get(sig_param_idx)
                                    .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Owned));
                                if param_is_owned {
                                    operand // Strip & — generate the inner expression
                                } else {
                                    arg // Keep the & — parameter expects a reference
                                }
                            } else {
                                arg // No signature info — keep as-is
                            }
                        } else {
                            arg // Not a & expression — keep as-is
                        };

                        let prev_field_access_obj = self.in_field_access_object;
                        self.in_field_access_object = false;
                        let prev_coerce_string_literals = self.coerce_string_literals_to_owned;
                        self.coerce_string_literals_to_owned = false;
                        let prev_match_arm_str = self.in_match_arm_needing_string;
                        self.in_match_arm_needing_string = false;
                        let mut arg_str = self.generate_expression(arg_to_generate);
                        self.coerce_string_literals_to_owned = prev_coerce_string_literals;
                        self.in_match_arm_needing_string = prev_match_arm_str;
                        self.in_field_access_object = prev_field_access_obj;

                        // TDD FIX: Vec index methods require usize arguments.
                        // Int inference may resolve the literal to i32/u32/i64/u64 due to
                        // conflicting constraints. Fix at codegen level: rewrite any
                        // integer suffix to _usize for the first argument of known
                        // index-taking methods.
                        if i == 0
                            && super::stdlib_method_traits::is_index_taking_method(method)
                        {
                            let is_int_literal = matches!(
                                arg,
                                Expression::Literal {
                                    value: Literal::Int(_) | Literal::IntSuffixed(_, _),
                                    ..
                                }
                            );
                            if is_int_literal {
                                let int_suffixes =
                                    ["_i32", "_i64", "_u32", "_u64", "_i16", "_u16", "_i8", "_u8"];
                                for suffix in &int_suffixes {
                                    if arg_str.ends_with(suffix) {
                                        arg_str = format!(
                                            "{}_usize",
                                            &arg_str[..arg_str.len() - suffix.len()]
                                        );
                                        break;
                                    }
                                }
                            }
                        }

                        // TDD FIX: AUTO-WRAP function pointers in iterator adapter methods.
                        // Rust's .filter()/.any()/.find() on iter() yield &&T, expecting FnMut(&&T) -> bool,
                        // but bare function pointers fn(&T) -> bool don't auto-deref.
                        // THE WINDJAMMER WAY: Users write the natural `filter(predicate)` and the
                        // compiler generates `filter(|__e| predicate(__e))`.
                        if i == 0
                            && super::stdlib_method_traits::is_closure_taking_method(method)
                            && matches!(arg, Expression::Identifier { .. })
                        {
                            // Bare identifier (function pointer) passed to iterator adapter -
                            // wrap in closure so Rust's auto-deref handles &&T -> &T.
                            arg_str = format!("|__e| {}(__e)", arg_str);
                        }

                        // TDD FIX: String literal ownership conversion
                        // Windjammer philosophy: "sword" should work whether parameter wants String or &String
                        // CRITICAL: Do NOT convert for explicit &str parameters! Only for inferred &String.
                        let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                        let sig_param_idx = if method_signature.as_ref().is_some_and(|s| s.has_self_receiver) { i + 1 } else { i };
                        let param_ownership = method_signature
                            .as_ref()
                            .and_then(|sig| sig.param_ownership.get(sig_param_idx));
                        let string_literal_converted = if is_string_literal {
                            // Check what the parameter wants

                            // CRITICAL: Check if parameter is explicitly &str (not inferred &String)
                            // Explicit &str parameters should NOT get .to_string() conversion
                            let param_type = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_types.get(sig_param_idx));
                            let is_explicit_str_ref = if let Some(Type::Reference(inner)) = param_type {
                                matches!(**inner, Type::String) ||
                                matches!(**inner, Type::Custom(ref s) if s == "str")
                            } else {
                                false
                            };

                            if is_explicit_str_ref {
                                // Explicit &str parameter - no conversion needed
                                false
                            } else {
                                match param_ownership {
                                    Some(&OwnershipMode::Owned) | Some(&OwnershipMode::Borrowed) => {
                                        // TDD FIX: Both Owned and Borrowed string params need .to_string()
                                        // Owned → String needs .to_string()
                                        // Borrowed → &String needs .to_string() (then & is added later)
                                        // String literals are &str, must allocate to get String/&String
                                        arg_str = format!("{}.to_string()", arg_str);
                                        true // Mark that we converted
                                    }
                                    _ => {
                                        // No signature info - use heuristic (fallback to old logic)
                                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_to_string(i, method, &method_signature) {
                                            arg_str = format!("{}.to_string()", arg_str);
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                }
                            }
                        } else {
                            false
                        };

                        // TDD FIX: If we converted string literal for Borrowed parameter,
                        // we need to add & since .to_string() produces String but param wants &String
                        if string_literal_converted {
                            if let Some(&OwnershipMode::Borrowed) = param_ownership {
                                // .to_string() produces String, but Borrowed param wants &String
                                // So we need to add &
                                arg_str = format!("&{}", arg_str);
                            }
                        }

                        // TDD FIX: AUTO-CONVERT &str/&String → String for method calls
                        // When passing a &str parameter to a method expecting owned String, convert it
                        // This handles cases like: recipe.add_ingredient("herb", 1) where add_ingredient expects String
                        if let Expression::Identifier { name, .. } = arg {
                            // Find the parameter type
                            let param_type = self.current_function_params.iter()
                                .find(|p| &p.name == name)
                                .map(|p| &p.type_);

                            // Check if parameter type is &str (Type::Reference(Type::String))
                            if let Some(Type::Reference(inner_type)) = param_type {
                                if matches!(**inner_type, Type::String) {
                                    // Check if method signature expects owned String for this parameter
                                    let expects_owned = method_signature
                                        .as_ref()
                                        .and_then(|sig| sig.param_ownership.get(i))
                                        .is_some_and(|&ownership| matches!(ownership, OwnershipMode::Owned));

                                    if expects_owned && !arg_str.ends_with(".to_string()") && !arg_str.ends_with(".clone()") {
                                        arg_str = format!("{}.to_string()", arg_str);
                                    }
                                }
                            }
                        }

                        // AUTO .clone(): Add .clone() when needed for borrowed values
                        if crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_clone(
                            arg,
                            &arg_str,
                            method,
                            i,
                            &method_signature,
                            &self.borrowed_iterator_vars,
                            &self.current_function_params,
                            &self.inferred_borrowed_params,
                            &self.current_function_return_type,
                        ) {
                            arg_str = format!("{}.clone()", arg_str);
                        }

                        // DOGFOODING FIX: Vec indexing vec[idx] passed to owned param (e.g. push)
                        // should_add_clone handles Identifier/FieldAccess; Index needs explicit check
                        // Vec::push uses stdlib heuristics (method_signature=None) - param 0 expects Owned
                        if let Expression::Index { .. } = arg {
                            let sig_param_idx = method_signature
                                .as_ref()
                                .map(|s| if s.has_self_receiver { i + 1 } else { i })
                                .unwrap_or(i);
                            let param_expects_owned = method_signature
                                .as_ref()
                                .and_then(|sig| sig.param_ownership.get(sig_param_idx))
                                .is_some_and(|&o| matches!(o, OwnershipMode::Owned))
                                || (method == "push" && i == 0);
                            if param_expects_owned && !arg_str.ends_with(".clone()") {
                                let inferred = self.infer_expression_type(arg);
                                let is_copy = inferred.as_ref().is_some_and(|t| self.is_type_copy(t));
                                if is_copy {
                                    if arg_str.starts_with("&") {
                                        arg_str = arg_str
                                            .strip_prefix('&')
                                            .unwrap_or(&arg_str)
                                            .to_string();
                                    }
                                } else {
                                    // Non-Copy or unknown type: clone to prevent E0507
                                    if arg_str.starts_with("&") {
                                        arg_str = format!("({}).clone()", arg_str);
                                    } else {
                                        arg_str = format!("{}.clone()", arg_str);
                                    }
                                }
                            }
                        }

                        // TDD FIX: Strip unnecessary .clone() when method param is Borrowed
                        // When a field like `ingredient.item_id` is auto-cloned by the
                        // FieldAccess handler (because owner is borrowed), but the method
                        // expects &String (Borrowed), the clone is wasteful:
                        //   &ingredient.item_id.clone()  ← clones then borrows (wasteful)
                        //   &ingredient.item_id          ← borrows directly (correct)
                        // Strip the .clone() so should_add_ref can add & cleanly.
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::Borrowed));
                            if param_is_borrowed && arg_str.ends_with(".clone()") {
                                arg_str = arg_str[..arg_str.len() - 8].to_string();
                            }
                        }

                        // AUTO-MUT-BORROW: Add &mut when parameter expects MutBorrowed
                        if let Some(ref sig) = method_signature {
                            let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                            let param_is_mut_borrowed = sig
                                .param_ownership
                                .get(sig_param_idx)
                                .is_some_and(|&o| matches!(o, OwnershipMode::MutBorrowed));
                            if param_is_mut_borrowed {
                                let is_already_mut_ref =
                                    if let Expression::Identifier { name, .. } = arg {
                                        let explicit_mut_ref = self.current_function_params.iter().any(|param| {
                                            param.name == *name
                                                && matches!(&param.type_, Type::MutableReference(_))
                                        });
                                        let inferred_mut_ref = self.inferred_mut_borrowed_params.contains(name.as_str());
                                        explicit_mut_ref || inferred_mut_ref
                                    } else {
                                        false
                                    };
                                if !expression_helpers::is_reference_expression(arg)
                                    && !is_already_mut_ref
                                {
                                    if arg_str.ends_with(".clone()") {
                                        arg_str = arg_str[..arg_str.len() - 8].to_string();
                                    }
                                    if arg_str.starts_with("&") && !arg_str.starts_with("&mut ") {
                                        arg_str = arg_str[1..].to_string();
                                    }
                                    arg_str = format!("&mut {}", arg_str);
                                }
                            }
                        }

                        // AUTO-REF: Add & when parameter expects reference but arg is owned
                        if !string_literal_converted {
                            // Use `arg_to_generate` (after stripping explicit `&` for map keys / owned params)
                            // so `should_add_ref` sees `key` not `&key` — otherwise the Unary(Ref) early-return
                            // skips HashMap `str` key handling and we emit `get(&key)` for `key: &str` (E0277).
                            let should_ref = crate::codegen::rust::method_call_analyzer::MethodCallAnalyzer::should_add_ref(
                                arg_to_generate,
                                &arg_str,
                                method,
                                i,
                                &method_signature,
                                &self.usize_variables,
                                &self.current_function_params,
                                &self.borrowed_iterator_vars,
                                &self.inferred_borrowed_params,
                                arguments.len(),
                                type_name.as_deref(),
                                Some(&self.local_var_types),
                                Some(&self.stdlib_method_signatures),
                                Some(&self.method_signatures_by_type),
                            );
                            if should_ref {
                                if let Expression::Cast { .. } = arg_to_generate {
                                    arg_str = format!("&({})", arg_str);
                                } else {
                                    arg_str = format!("&{}", arg_str);
                                }
                            }
                        }

                        // AUTO-BORROW: Methods that take &T or &[T] should auto-borrow
                        // when given owned values. Eliminates Rust leakage in .wj files.
                        let auto_borrow_methods = ["push_str", "extend_from_slice"];
                        if auto_borrow_methods.contains(&method.as_str()) && i == 0 {
                            let is_string_literal = matches!(arg, Expression::Literal { value: Literal::String(_), .. });
                            if !is_string_literal && !arg_str.starts_with('&') {
                                let needs_borrow = matches!(arg,
                                    Expression::Identifier { .. } |
                                    Expression::FieldAccess { .. } |
                                    Expression::MethodCall { .. }
                                );
                                if needs_borrow {
                                    arg_str = format!("&{}", arg_str);
                                }
                            }
                        }

                        // AUTO-CAST int → float: when parameter expects f32/f64 but argument is int
                        // Skip when signature has a collision (different types with same name).
                        {
                            let effective_sig = method_signature.as_ref()
                                .or_else(|| self.signature_registry.get_signature(method));
                            let has_collision = self.signature_registry.has_collision(method);
                            if let Some(sig) = effective_sig {
                                let sig_param_idx = if sig.has_self_receiver { i + 1 } else { i };
                                if !has_collision {
                                    if let Some(param_ty) = sig.param_types.get(sig_param_idx) {
                                        let param_is_f32 = matches!(param_ty, Type::Custom(n) if n == "f32");
                                        let param_is_f64 = matches!(param_ty, Type::Custom(n) if n == "f64");
                                        if param_is_f32 || param_is_f64 {
                                            let arg_ty = self.infer_expression_type(arg);
                                            let arg_is_int = arg_ty.as_ref().is_some_and(|t| {
                                                matches!(t, Type::Int)
                                                    || matches!(t, Type::Custom(n) if matches!(n.as_str(),
                                                        "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                                                    ))
                                            });
                                            if arg_is_int && !arg_str.contains(" as f32") && !arg_str.contains(" as f64") {
                                                let target = if param_is_f32 { "f32" } else { "f64" };
                                                arg_str = if arg_str.contains(' ') || matches!(arg, Expression::Binary { .. }) {
                                                    format!("({}) as {}", arg_str, target)
                                                } else {
                                                    format!("{} as {}", arg_str, target)
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Restore suppress flag
                        self.suppress_borrowed_clone = prev_suppress;

                        arg_str
                    })
                    .collect();

                // Restore float target type after argument generation
                self.assignment_float_target_type = prev_float_target;

                // Generate turbofish if present
                let turbofish = if let Some(types) = type_args {
                    let type_strs: Vec<String> =
                        types.iter().map(|t| self.type_to_rust(t)).collect();
                    format!("::<{}>", type_strs.join(", "))
                } else {
                    String::new()
                };

                // Special case: empty method name means turbofish on a function call (func::<T>())
                if method.is_empty() {
                    return format!("{}{}({})", obj_str, turbofish, args.join(", "));
                }

                // Special case: substring(start, end) -> &text[start..end]
                if method == "substring" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // Special case: contains() with String argument needs .as_str()
                // String::contains() expects &str, not String
                if method == "contains" && args.len() == 1 {
                    // Check if argument is a method call that returns String (like to_lowercase())
                    if let Some((_label, arg)) = arguments.first() {
                        if matches!(arg, Expression::MethodCall { method: m, .. } if
                            m == "to_lowercase" || m == "to_uppercase" ||
                            m == "to_string" || m == "trim" || m == "clone")
                        {
                            // The argument is String, needs .as_str()
                            return format!("{}.{}({}.as_str())", obj_str, method, args[0]);
                        }
                    }
                }

                // Determine separator: :: for static calls, . for instance methods
                // - Type/Module (starts with uppercase): use ::
                // - Variable (starts with lowercase): use .
                let separator = match &**object {
                    Expression::Call { .. } | Expression::MethodCall { .. } => ".", // Instance method on return value
                    Expression::Identifier { name, .. } => {
                        // Check for known module/crate names that should use ::
                        // Note: Avoid common variable names like "path", "config" which are used as variables
                        let known_modules = [
                            "std",
                            "serde_json",
                            "serde",
                            "tokio",
                            "reqwest",
                            "sqlx",
                            "chrono",
                            "sha2",
                            "bcrypt",
                            "base64",
                            "rand",
                            "Vec",
                            "String",
                            "Option",
                            "Result",
                            "Box",
                            "Arc",
                            "Mutex",
                            "Utc",
                            "Local",
                            "DEFAULT_COST",
                            // Stdlib modules (avoid common variable names)
                            "mime",
                            "http",
                            "fs",
                            "strings",
                            // NOTE: "json" removed - it's a common variable name!
                            // Use "serde_json" for the module instead
                            "regex",
                            "cli",
                            "log",
                            "crypto",
                            "io",
                            "env",
                            "time",
                            "sync",
                            "thread",
                            "collections",
                            "cmp",
                        ];

                        // Type or module (uppercase) vs variable (lowercase)
                        if name.chars().next().is_some_and(|c| c.is_uppercase())
                            || name.contains('.')
                            || known_modules.contains(&name.as_str())
                        {
                            "::" // Vec::new(), std::fs::read(), serde_json::to_string()
                        } else {
                            "." // x.abs(), value.method()
                        }
                    }
                    Expression::FieldAccess { ref object, .. } => {
                        // Check if this is a module path (e.g., std::fs) or a field access (e.g., self.count)
                        // If the object is an identifier that looks like a module, use ::
                        // Otherwise, use . for instance methods on fields
                        match object {
                            Expression::Identifier { name, .. } => {
                                if name.chars().next().is_some_and(|c| c.is_uppercase())
                                    || name == "std"
                                {
                                    "::" // Module::path::method() -> static method
                                } else {
                                    "." // self.field.method() or variable.field.method() -> instance method
                                }
                            }
                            _ => ".", // Default to instance method
                        }
                    }
                    _ => ".", // Instance method on expressions
                };

                // SPECIAL CASE: .slice() method is our desugared slice syntax [start..end]
                // Convert it back to proper Rust slice syntax
                // For strings, we need to add & to get &str (a reference)
                if method == "slice" && args.len() == 2 {
                    return format!("&{}[{}..{}]", obj_str, args[0], args[1]);
                }

                // E0308: Borrowed Windjammer `string` parameters lower to `&str`. `.clone()` on `&str`
                // is still `&str`, but users mean an owned copy → emit `.to_string()`.
                if method == "clone" && arguments.is_empty() {
                    if let Expression::Identifier { name, .. } = &**object {
                        if self.inferred_borrowed_params.contains(name.as_str())
                            && self
                                .current_function_params
                                .iter()
                                .find(|p| p.name == *name)
                                .is_some_and(|p| {
                                    crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                                })
                        {
                            return format!("{}.to_string()", obj_str);
                        }
                    }
                }

                // PHASE 2 OPTIMIZATION: Eliminate unnecessary .clone() calls
                // DISABLED: This optimization was too aggressive and removed needed clones
                // TODO: Make this more conservative - only remove clone when we can prove
                // the value is Copy or when it's the last use
                // if method == "clone" && arguments.is_empty() {
                //     if let Expression::Identifier { name: ref var_name, location: None } = **object {
                //         if self.clone_optimizations.contains(var_name) {
                //             // Skip the .clone(), just return the variable (or borrow if needed)
                //             return obj_str;
                //         }
                //     }
                // }

                // UI FRAMEWORK: Check if we need to add .to_vnode() for .child() methods
                // DISABLED: Too aggressive - needs type checking to determine if parameter expects VNode
                // TODO: Re-enable with proper type checking when VNode type bindings are implemented
                let processed_args = args;

                // WINDJAMMER STDLIB → RUST TRANSLATION
                // Some Windjammer methods don't exist in Rust and need translation.
                //
                // reversed() → into_iter().rev().collect::<Vec<_>>()
                if method == "reversed" && processed_args.is_empty() {
                    return format!("{}.into_iter().rev().collect::<Vec<_>>()", obj_str);
                }
                // enumerate() → iter().enumerate()
                // Rust Vec doesn't have .enumerate() — only iterators do.
                // But if the object already ends with .iter(), .iter_mut(), or
                // .into_iter(), don't add a redundant .iter() prefix.
                if method == "enumerate" && processed_args.is_empty() {
                    let already_iterator = obj_str.ends_with(".iter()")
                        || obj_str.ends_with(".iter_mut()")
                        || obj_str.ends_with(".into_iter()");
                    if already_iterator {
                        return format!("{}.enumerate()", obj_str);
                    } else {
                        return format!("{}.iter().enumerate()", obj_str);
                    }
                }

                // TDD FIX (Bug #3): Extract format!() macros in method arguments too
                let has_format_arg = processed_args
                    .iter()
                    .any(|arg_str| arg_str.contains("format!("));

                let base_expr = if has_format_arg {
                    // Extract format!() macros to temp variables
                    let mut temp_decls = String::new();
                    let mut temp_counter = 0;
                    let fixed_args: Vec<String> = processed_args
                        .iter()
                        .map(|arg_str| {
                            if arg_str.starts_with("format!(") || arg_str.starts_with("&format!(") {
                                // Strip leading & if present (was added by argument processing)
                                let format_expr = if arg_str.starts_with("&") {
                                    arg_str.strip_prefix("&").unwrap()
                                } else {
                                    arg_str
                                };
                                // Extract to temp var
                                let temp_name = format!("_temp{}", temp_counter);
                                temp_counter += 1;
                                temp_decls
                                    .push_str(&format!("let {} = {}; ", temp_name, format_expr));

                                // When the method expects &str (push_str, extend_from_slice),
                                // add & to pass borrowed temp. Otherwise, pass owned value.
                                let method_needs_borrow = matches!(
                                    method.as_str(),
                                    "push_str" | "extend_from_slice"
                                );
                                if arg_str.starts_with("&") || method_needs_borrow {
                                    format!("&{}", temp_name)
                                } else {
                                    temp_name
                                }
                            } else {
                                arg_str.clone()
                            }
                        })
                        .collect();

                    // Wrap in block: { let _temp0 = format!(...); obj.method(&_temp0, ...) }
                    format!(
                        "{{ {}{}{}{}{}({}) }}",
                        temp_decls,
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        fixed_args.join(", ")
                    )
                } else {
                    format!(
                        "{}{}{}{}({})",
                        obj_str,
                        separator,
                        method,
                        turbofish,
                        processed_args.join(", ")
                    )
                };

                // AUTO-CLONE: Method call results are ALWAYS owned values.
                // Unlike field accesses (self.field borrows from self) or identifiers
                // (which may be borrowed), calling a method produces a fresh value.
                // The auto-clone analysis may flag the *object* for cloning, but that
                // doesn't mean the *result of the method call* needs cloning.
                //
                // Exception: methods that return references (get, first, last) are
                // handled separately by should_add_cloned().
                //
                // WINDJAMMER PHILOSOPHY: Only clone when semantically necessary.
                // Method call results are never borrowed — cloning them is pure noise.
                base_expr
            }
            Expression::FieldAccess { object, field, .. } => {
                // FIELD CHAIN OPTIMIZATION: If we're accessing a likely-Copy sub-field
                // (e.g., .x, .y, .width, .speed), suppress borrowed-iterator cloning
                // on the intermediate object. In Rust, (&enemy).velocity.y works fine
                // through auto-deref — no need to clone the intermediate Vec2.
                let field_is_likely_copy = matches!(
                    field.as_str(),
                    "x" | "y"
                        | "z"
                        | "w"
                        | "width"
                        | "height"
                        | "depth"
                        | "r"
                        | "g"
                        | "b"
                        | "a"
                        | "left"
                        | "right"
                        | "top"
                        | "bottom"
                        | "min"
                        | "max"
                        | "start"
                        | "end"
                        | "offset"
                        | "scale"
                        | "speed"
                        | "time"
                        | "delta"
                        | "angle"
                        | "radius"
                        | "distance"
                        | "visible"
                        | "enabled"
                        | "active"
                        | "selected"
                        | "focused"
                        | "id"
                        | "type"
                        | "kind"
                        | "priority"
                        | "level"
                        | "len"
                        | "count"
                        | "size"
                        | "index"
                        | "idx"
                        | "vx"
                        | "vy"
                        | "vz"
                        | "dx"
                        | "dy"
                        | "dz"
                        | "health"
                        | "damage"
                        | "score"
                        | "lives"
                        | "frame"
                );
                // Also check via type inference if the outer expression (self.obj.field) is Copy
                let field_is_copy_by_type = self
                    .infer_expression_type(expr_to_generate)
                    .as_ref()
                    .is_some_and(|t| self.is_type_copy(t));

                let prev_suppress = self.suppress_borrowed_clone;
                let prev_field_access = self.in_field_access_object;
                if field_is_likely_copy || field_is_copy_by_type {
                    self.suppress_borrowed_clone = true;
                }
                // Suppress Vec index clone when we're just accessing a field
                // e.g., players[i].score → no need to clone the whole Player
                self.in_field_access_object = true;
                let obj_str = self.generate_expression_with_precedence(object);
                self.in_field_access_object = prev_field_access;
                self.suppress_borrowed_clone = prev_suppress;

                // Determine if this is a module/type path (::) or field access (.)
                // Check the object to decide:
                let separator = match &**object {
                    Expression::Identifier { name, .. }
                        if name.contains("::")
                            || (!name.is_empty()
                                && name.chars().next().unwrap().is_uppercase()) =>
                    {
                        "::" // Module path: std::fs or Type::CONST
                    }
                    Expression::FieldAccess { .. } => {
                        // Check if this is a module path or a field chain
                        // If the object string contains ::, it's a module path
                        if obj_str.contains("::") {
                            "::" // Module path: std::fs::File
                        } else {
                            "." // Field chain: transform.position.x
                        }
                    }
                    _ => ".", // Actual field access (e.g., config.field)
                };

                let base_expr = format!("{}{}{}", obj_str, separator, field);

                // AUTO-CLONE: Check if this field access needs to be cloned
                // Extract the full path (e.g., "config.paths")
                // CRITICAL: Never clone assignment targets (left side of `=`)
                // e.g., `emitter.lifetime = 1.0` must NOT become `emitter.clone().lifetime = 1.0`
                // DOUBLE-CLONE FIX: Skip auto-clone when we're inside an explicit .clone() call
                // The source already has .clone(), so we must not add another one.
                // METHOD RECEIVER / FOR-LOOP FIX: Skip auto-clone when in a method receiver
                // or for-loop iterable context. Rust auto-borrows method receivers (&self),
                // and for-loops iterate by reference with `&`. Cloning is unnecessary and
                // breaks for Vec<Box<dyn Trait>> or Vec<T> where T may not be Clone.
                if !self.generating_assignment_target
                    && !self.in_explicit_clone_call
                    && !self.in_field_access_object
                {
                    if let Some(path) = ast_utilities::extract_field_access_path(expr_to_generate) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(&path, self.current_statement_idx)
                                .is_some()
                            {
                                // Skip .clone() for Copy types (f32, i32, bool, etc.)
                                // They are implicitly copied — .clone() is unnecessary noise.
                                let is_copy = self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    // Type inference failed — fall back to name heuristic
                                    // Fields like x, y, z, width, height are almost always Copy
                                    let is_likely_copy_field = matches!(
                                        field.as_str(),
                                        "x" | "y"
                                            | "z"
                                            | "w"
                                            | "width"
                                            | "height"
                                            | "depth"
                                            | "r"
                                            | "g"
                                            | "b"
                                            | "a"
                                            | "left"
                                            | "right"
                                            | "top"
                                            | "bottom"
                                            | "min"
                                            | "max"
                                            | "start"
                                            | "end"
                                            | "offset"
                                            | "scale"
                                            | "speed"
                                            | "time"
                                            | "delta"
                                            | "angle"
                                            | "radius"
                                            | "distance"
                                            | "visible"
                                            | "enabled"
                                            | "active"
                                            | "selected"
                                            | "focused"
                                            | "id"
                                            | "type"
                                            | "kind"
                                            | "priority"
                                            | "level"
                                            | "len"
                                            | "count"
                                            | "size"
                                            | "index"
                                            | "idx"
                                            | "vx"
                                            | "vy"
                                            | "vz"
                                            | "dx"
                                            | "dy"
                                            | "dz"
                                            | "health"
                                            | "damage"
                                            | "score"
                                            | "lives"
                                            | "frame"
                                    );
                                    if !is_likely_copy_field {
                                        return format!("{}.clone()", base_expr);
                                    }
                                }
                            }
                        }
                    }
                }

                // BORROWED ITERATOR: If accessing fields through a borrowed iterator variable,
                // we need to clone non-Copy fields since we can't move out of a reference
                // BUT: Don't clone for assignment targets (left side of =)
                // AND: Don't clone when a parent FieldAccess is reading a Copy sub-field
                //      (e.g., bullet.velocity.y → .y is Copy, so no need to clone velocity)
                // AND: Don't clone when inside an explicit .clone() call (prevents double clone)
                // AND: Don't clone when this is an intermediate object in a field access chain
                //      (e.g., stack.item.stats.armor → don't clone item, Rust auto-derefs through &)
                // AND: Don't clone in borrow context (&recipe.ingredients → reference is sufficient)
                // TDD FIX: Don't clone when generating call arguments (Call handler applies ownership)
                // WINDJAMMER PHILOSOPHY: Use type inference first, fall back to name heuristics
                if !self.generating_assignment_target
                    && !self.suppress_borrowed_clone
                    && !self.in_explicit_clone_call
                    && !self.in_field_access_object
                    && !self.in_borrow_context
                    && !self.in_call_argument_generation
                {
                    if let Expression::Identifier { name: var_name, .. } = &**object {
                        if self.borrowed_iterator_vars.contains(var_name) {
                            // First: use type inference to check if the field type is Copy
                            let is_copy = self
                                .infer_expression_type(expr_to_generate)
                                .as_ref()
                                .is_some_and(|t| self.is_type_copy(t));

                            if !is_copy {
                                // Fall back to name-based heuristics for fields we KNOW are Copy
                                let is_likely_copy_field = matches!(
                                    field.as_str(),
                                    "len" | "count" | "size" | "index" | "idx" | "i" | "j" | "k" |
                                    "x" | "y" | "z" | "w" | "width" | "height" | "depth" |
                                    "r" | "g" | "b" | "a" | "left" | "right" | "top" | "bottom" |
                                    "min" | "max" | "start" | "end" | "offset" | "scale" |
                                    "speed" | "time" | "delta" | "angle" | "radius" | "distance" |
                                    "visible" | "enabled" | "active" | "selected" | "focused" |
                                    "id" | "type" | "kind" | "priority" | "level" |
                                    // Method-like names that should NOT be cloned
                                    "as_str" | "to_string" | "clone" | "iter" | "iter_mut" | "is_empty"
                                );
                                if !is_likely_copy_field && !base_expr.ends_with(".clone()") {
                                    return format!("{}.clone()", base_expr);
                                }
                            }
                        }
                    }
                }

                // &self field clone: when accessing self.field in a &self method,
                // non-Copy types can't be moved out of the reference — auto-clone.
                // Skip in comparison contexts — refs compare fine without cloning.
                if !self.generating_assignment_target
                    && !self.in_explicit_clone_call
                    && !self.in_field_access_object
                    && !self.in_borrow_context
                    && !self.suppress_borrowed_clone
                {
                    if let Expression::Identifier { name: obj_name, .. } = &**object {
                        if obj_name == "self"
                            && self.inferred_borrowed_params.contains("self")
                            && self.in_impl_block
                        {
                            let field_is_copy = self
                                .current_struct_name
                                .as_ref()
                                .and_then(|sn| self.struct_field_types.get(sn.as_str()))
                                .and_then(|fields| fields.get(field.as_str()))
                                .is_some_and(|ty| self.is_type_copy(ty));
                            if !field_is_copy {
                                return format!("{}.clone()", base_expr);
                            }
                        }
                    }
                }

                // VEC INDEX FIELD ACCESS: When accessing a non-Copy field through Vec
                // indexing (e.g., choices[i].text), Rust can't move out of a Vec element.
                // The Index handler suppresses its own borrow/clone when in_field_access_object
                // is true (correct for Copy fields like .score), but for non-Copy fields
                // like String, the resulting expression `vec[i].text` is still a move.
                // Fix: clone the field access result when the field type is non-Copy.
                if !self.generating_assignment_target
                    && !self.in_explicit_clone_call
                    && !self.in_field_access_object
                    && !self.in_borrow_context
                    && !self.in_call_argument_generation
                {
                    let object_has_index = matches!(&**object, Expression::Index { .. })
                        || matches!(&**object, Expression::FieldAccess { object: inner, .. }
                            if matches!(&**inner, Expression::Index { .. }));

                    if object_has_index && !(field_is_likely_copy || field_is_copy_by_type) {
                        return format!("{}.clone()", base_expr);
                    }
                }

                base_expr
            }
            Expression::StructLiteral { name, fields, .. } => {
                // PHASE 3 OPTIMIZATION: Check if we have optimization hints for this struct
                let _has_optimization_hint = self.struct_mapping_hints.get(name);

                // CONTEXT-SENSITIVE INFERENCE: Set struct literal context for float type inference
                let prev_struct_name = self.current_struct_literal_name.clone();
                self.current_struct_literal_name = Some(name.to_string());

                // Generate field assignments
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(field_name, expr)| {
                        // STRUCT LITERAL CONTEXT: Array literals in struct fields should use
                        // fixed-size [...] syntax, not vec![...], because struct fields have
                        // explicit type annotations (e.g., position: [f32; 3]).
                        let prev_in_struct_field = self.in_struct_literal_field;
                        let prev_field_name = self.current_struct_field_name.clone();
                        self.in_struct_literal_field = true;
                        self.current_struct_field_name = Some(field_name.to_string());

                        // WINDJAMMER PHILOSOPHY: Auto-convert string literals to String
                        // In Windjammer, `string` type is always owned (maps to Rust String)
                        // So string literals in struct fields should be converted automatically.
                        // Set coercion flag BEFORE generation so nested expressions (if-else
                        // branches, match arms, blocks) also coerce their string literals.
                        let prev_coerce = self.coerce_string_literals_to_owned;
                        self.coerce_string_literals_to_owned = true;
                        let mut expr_str = self.generate_expression(expr);
                        self.coerce_string_literals_to_owned = prev_coerce;

                        // Restore previous context
                        self.in_struct_literal_field = prev_in_struct_field;
                        self.current_struct_field_name = prev_field_name;

                        // Auto-convert direct string literals that weren't already coerced
                        if matches!(
                            expr,
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        ) && !expr_str.ends_with(".to_string()") {
                            expr_str = format!("{}.to_string()", expr_str);
                        }

                        // CRITICAL: Auto-convert &str parameters to String for struct fields
                        // Pattern: fn create(name: &str) -> User { User { name: name } }
                        // When struct field is String but parameter is &str, add .to_string()
                        if let Expression::Identifier { name: id, .. } = expr {
                            // Check if this identifier is a &str parameter
                            // In the AST, &str parameters have type Reference(Custom("str"))
                            let is_str_param = self.current_function_params.iter().any(|p| {
                                p.name == *id && matches!(
                                    &p.type_,
                                    crate::parser::Type::Reference(inner) if matches!(**inner, crate::parser::Type::Custom(ref name) if name == "str")
                                )
                            });

                            if is_str_param && !expr_str.contains(".to_string()") {
                                expr_str = format!("{}.to_string()", expr_str);
                            }
                        }

                        // CRITICAL: Auto-clone self.field when constructing struct from borrowed self
                        // Pattern: fn method(&self) -> Self { Self { field: self.field } }
                        // Non-Copy fields from borrowed self need to be cloned
                        if let Expression::FieldAccess { object, .. } = expr {
                            if let Expression::Identifier { name: obj_name, .. } = &**object {
                                if obj_name == "self" && !expr_str.contains(".clone()") {
                                    // Check if current function takes &self (borrowed)
                                    let self_is_borrowed =
                                        self.current_function_params.iter().any(|p| {
                                            p.name == "self"
                                                && matches!(
                                                    p.ownership,
                                                    crate::parser::OwnershipHint::Ref
                                                )
                                        });

                                    if self_is_borrowed {
                                        // Clone the field access since self is borrowed
                                        expr_str = format!("{}.clone()", expr_str);
                                    }
                                }
                            }
                        }

                        // E0308: bindings from match/if-let on `&T` are `&U` when `U: Copy`
                        if matches!(
                            expr,
                            Expression::Identifier { .. } | Expression::FieldAccess { .. }
                        ) {
                            expr_str = self.peel_copy_ref_binding_for_struct_field(expr, &expr_str);
                            expr_str =
                                self.clone_non_copy_ref_binding_for_struct_field(expr, &expr_str);
                        }

                        // Check for field shorthand: if expr is just the field name AND no conversion applied, use shorthand
                        // Only use shorthand if the generated expression exactly matches the field name
                        // (no .to_string(), .clone(), etc. conversions)
                        if let Expression::Identifier { name: id, .. } = expr {
                            if id == field_name && expr_str == *field_name {
                                // Shorthand: User { name } instead of User { name: name }
                                // Only safe when no type conversion was needed
                                return field_name.clone();
                            }
                        }

                        format!("{}: {}", field_name, expr_str)
                    })
                    .collect();

                // Restore struct literal context
                self.current_struct_literal_name = prev_struct_name;

                let qualified_name = self.qualify_external_path_identifier(name);
                format!("{} {{ {} }}", qualified_name, field_str.join(", "))
            }
            Expression::MapLiteral { pairs, .. } => {
                // Generate HashMap literal: HashMap::from([(key, value), ...])
                if pairs.is_empty() {
                    "std::collections::HashMap::new()".to_string()
                } else {
                    let entries_str: Vec<String> = pairs
                        .iter()
                        .map(|(k, v)| {
                            let key_str = self.generate_expression(k);
                            let val_str = self.generate_expression(v);
                            format!("({}, {})", key_str, val_str)
                        })
                        .collect();
                    format!(
                        "std::collections::HashMap::from([{}])",
                        entries_str.join(", ")
                    )
                }
            }
            Expression::TryOp { expr: inner, .. } => {
                format!("{}?", self.generate_expression(inner))
            }
            Expression::Await { expr: inner, .. } => {
                format!("{}.await", self.generate_expression(inner))
            }
            Expression::ChannelSend { channel, value, .. } => {
                let ch_str = self.generate_expression(channel);
                let val_str = self.generate_expression(value);
                format!("{}.send({})", ch_str, val_str)
            }
            Expression::ChannelRecv { channel, .. } => {
                let ch_str = self.generate_expression(channel);
                format!("{}.recv()", ch_str)
            }
            Expression::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                // TDD FIX: Range type unification for 0..vec.len()
                // If end is .len() (returns usize), cast start to usize to avoid type mismatch
                let end_is_len =
                    matches!(end, Expression::MethodCall { method, .. } if method == "len");

                let mut start_str = self.generate_expression(start);

                // If end is .len() and start has _i32 suffix, replace with _usize or add cast
                if end_is_len {
                    if start_str.ends_with("_i32") {
                        // Replace _i32 with _usize for literals
                        start_str = start_str.replace("_i32", "_usize");
                    } else if matches!(
                        start,
                        Expression::Identifier { .. } | Expression::Binary { .. }
                    ) && !start_str.contains("as usize")
                    {
                        // Add cast for identifiers or expressions without existing cast
                        if matches!(start, Expression::Binary { .. }) {
                            start_str = format!("({} as usize)", start_str);
                        } else {
                            start_str = format!("{} as usize", start_str);
                        }
                    }
                }

                let end_str = self.generate_expression(end);
                if *inclusive {
                    format!("{}..={}", start_str, end_str)
                } else {
                    format!("{}..{}", start_str, end_str)
                }
            }
            Expression::Closure {
                parameters, body, ..
            } => {
                let params = parameters.join(", ");

                // THE WINDJAMMER WAY: Smart `move` inference for closures
                //
                // Add `move` automatically ONLY for compiler-generated closures (params start with __).
                // User-written closures are preserved as-is (respect explicit intent).
                // Rationale:
                // 1. Compiler-generated closures (function pointer wrappers) → add `move` for safety
                // 2. User-written closures → preserve exactly as written (explicit is explicit)
                // 3. Method closures that capture `self` → don't add `move` (UI callbacks need to borrow)
                //
                // This makes Windjammer code simpler while respecting explicit user intent.

                // Check if this is a compiler-generated closure (params start with __)
                let is_compiler_generated = parameters.iter().any(|p| p.starts_with("__"));

                // Check if the closure body references `self`
                let captures_self = self.expression_references_self(body);

                // For user-written closures, set flag and track params to suppress transformations
                let prev_in_user_closure = self.in_user_written_closure;
                let mut prev_closure_params = None;
                if !is_compiler_generated {
                    self.in_user_written_closure = true;
                    prev_closure_params = Some(std::mem::take(&mut self.user_closure_params));
                    for param in parameters {
                        self.user_closure_params.insert(param.clone());
                    }
                }

                // Generate closure body with context flags set
                let body_str = self.generate_expression(body);

                // Restore previous state
                if !is_compiler_generated {
                    self.in_user_written_closure = prev_in_user_closure;
                    if let Some(prev_params) = prev_closure_params {
                        self.user_closure_params = prev_params;
                    }
                }

                if is_compiler_generated && !captures_self {
                    // Compiler-generated closure that doesn't capture self → add `move`
                    format!("move |{}| {}", params, body_str)
                } else {
                    // User-written closure or captures self → preserve as-is
                    format!("|{}| {}", params, body_str)
                }
            }
            Expression::Index { object, index, .. } => {
                // INDEX CHAIN OPTIMIZATION: When generating the object of an Index expression,
                // suppress auto-clone. In `a[i][j]`, Rust auto-derefs `a[i]` (returns &Vec<T>)
                // to access [j]. Cloning the intermediate Vec is wasteful and wrong.
                // Same logic as in_field_access_object for FieldAccess chains.
                let prev_field_access = self.in_field_access_object;
                self.in_field_access_object = true;
                let obj_str = self.generate_expression(object);
                self.in_field_access_object = prev_field_access;

                // Special case: if index is a Range, this is slice syntax
                // FIXED: Don't add & - Rust will auto-coerce to &[T] when needed
                // This prevents "&temporary" errors when chaining methods like .to_vec()
                if let Expression::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = &**index
                {
                    let start_str = self.generate_expression(start);
                    let end_str = self.generate_expression(end);
                    let range_op = if *inclusive { "..=" } else { ".." };
                    return format!("{}[{}{}{}]", obj_str, start_str, range_op, end_str);
                }

                let mut idx_str = self.generate_expression(index);

                // WINDJAMMER PHILOSOPHY: Auto-cast to usize for array indexing
                // Rust requires usize for indexing, but Windjammer uses int (i64)
                // Handle cases:
                // 1. Simple identifier: arr[idx] -> arr[idx as usize]
                // 2. Integer literal: arr[0] -> arr[0 as usize]
                // 3. Cast to int/i64: arr[x as int] -> arr[x as usize]
                // 4. Parenthesized cast: arr[(x as int)] -> arr[x as usize]
                // 5. Already usize: don't double-cast
                let final_idx = if idx_str.ends_with("as i64)") || idx_str.ends_with("as int)") {
                    // Replace (... as i64/int) with (... as usize)
                    let base = idx_str
                        .trim_end_matches("as i64)")
                        .trim_end_matches("as int)")
                        .trim()
                        .trim_start_matches('(')
                        .trim();
                    format!("{} as usize", base)
                } else if idx_str.ends_with("as i64") || idx_str.ends_with("as int") {
                    // Replace ... as i64/int with ... as usize
                    let base = idx_str
                        .trim_end_matches("as i64")
                        .trim_end_matches("as int")
                        .trim();
                    format!("{} as usize", base)
                } else if !idx_str.contains(" as ") && !self.expression_produces_usize(index) {
                    // TDD FIX: Auto-cast ANY integer expression to usize (unless already cast)
                    // Handles:
                    // - Identifiers: items[i] → items[i as usize]
                    // - Literals: items[0] → items[0] (Rust infers)
                    // - Binary: items[i + 1] → items[(i + 1) as usize]  ← NEWLY FIXED!
                    // - Method calls: items[get_index()] → items[get_index() as usize]
                    //
                    // Skip cast only if:
                    // 1. Already has cast: items[i as usize]
                    // 2. Expression produces usize: items[vec.len()]
                    // 3. Identifier tracked as usize: for i in 0..10 { items[i] }
                    // 4. Non-negative literal: items[0] (Rust infers)

                    // Check special cases where cast is NOT needed
                    let needs_cast = match &**index {
                        Expression::Identifier { name, .. } => {
                            // Skip if tracked as usize variable
                            !self.usize_variables.contains(name)
                        }
                        Expression::Literal {
                            value: Literal::Int(n),
                            ..
                        } => {
                            // Non-negative int literals: Rust infers usize from index context.
                            // Strip any type suffix the inference engine may have added
                            // (e.g. `0_usize` → `0`), since the indexing context is enough.
                            if *n >= 0 {
                                let suffixes = ["_usize", "_i32", "_i64", "_u32", "_u64"];
                                for s in &suffixes {
                                    if idx_str.ends_with(s) {
                                        idx_str = idx_str[..idx_str.len() - s.len()].to_string();
                                        break;
                                    }
                                }
                            }
                            *n < 0
                        }
                        _ => true, // All other expressions need cast
                    };

                    if needs_cast {
                        // TDD FIX: Add parens for complex expressions to prevent precedence issues
                        // `i + 1` → `(i + 1) as usize` (not `i + 1 as usize` which is parsed as `i + (1 as usize)`)
                        let needs_parens = matches!(&**index, Expression::Binary { .. });
                        if needs_parens {
                            format!("({}) as usize", idx_str)
                        } else {
                            format!("{} as usize", idx_str)
                        }
                    } else {
                        idx_str
                    }
                } else {
                    idx_str
                };

                let base_expr = format!("{}[{}]", obj_str, final_idx);

                // WINDJAMMER PHILOSOPHY: Auto-borrow Vec indexing for non-Copy types (E0507 fix).
                // Rust doesn't allow moving out of a Vec index (E0507).
                // For Copy types: vec[idx] works directly (value is copied).
                // For non-Copy types: &vec[idx] (borrow) or vec[idx].clone() when owned needed.
                //
                // PREFER BORROW over clone: &vec[idx] is zero-cost; .clone() allocates.
                //
                // CRITICAL: NEVER add & or .clone() in these contexts:
                // 1. Assignment target: vec[i] = value (can't assign to .clone() or &)
                // 2. Borrow context: &vec[i] (parent adds &, we output vec[idx] only)
                // 3. Field access: vec[i].field (Rust allows field access through ref)
                // 4. Comparison context: vec[i] == val (comparisons work on &T)
                let suppress_borrow_or_clone = self.generating_assignment_target
                    || self.in_borrow_context
                    || self.in_field_access_object
                    || self.suppress_borrowed_clone;

                // TDD: Struct literal fields need owned values - force .clone() for Vec<String> etc.
                // Peel &Vec<T> (generated Rust for WJ `Vec<T>` params) so Copy element detection works.
                let element_type = self
                    .infer_expression_type(object)
                    .as_ref()
                    .and_then(|t| Self::peeled_collection_element_type(t))
                    .cloned();
                let force_clone_for_owned_context = (self.in_struct_literal_field || self.in_owned_value_context)
                    && element_type
                        .as_ref()
                        .map(|et| !self.is_type_copy(et))
                        .unwrap_or(true); // Unknown type → need clone (conservative)

                let suppress_borrow_or_clone =
                    suppress_borrow_or_clone && !force_clone_for_owned_context;

                if !suppress_borrow_or_clone {
                    // First check auto_clone_analysis (path-based analysis)
                    if let Some(path) = ast_utilities::extract_field_access_path(expr_to_generate) {
                        if let Some(ref analysis) = self.auto_clone_analysis {
                            if analysis
                                .needs_clone(&path, self.current_statement_idx)
                                .is_some()
                            {
                                let is_copy = self
                                    .infer_expression_type(expr_to_generate)
                                    .as_ref()
                                    .is_some_and(|t| self.is_type_copy(t));
                                if !is_copy {
                                    // Path analysis says clone needed (e.g. passed to owned param)
                                    return format!("{}.clone()", base_expr);
                                }
                            }
                        }
                    }

                    // Fallback: Type-based handling for Vec<NonCopy>[idx]
                    // E0507 fix: vec[idx] for String tries to move → use &vec[idx] (borrow)
                    // When owned value needed (struct literal): vec[idx].clone()
                    let needs_borrow_or_clone = self
                        .infer_expression_type(object)
                        .as_ref()
                        .and_then(|obj_ty| Self::peeled_collection_element_type(obj_ty))
                        .map(|elem_type| !self.is_type_copy(elem_type))
                        .unwrap_or_else(|| {
                            // Unknown element type: avoid `&vec[i]` for untyped Vecs filled with Copy
                            // values (e.g. `Vec::with_capacity` + `push(0 as u8)`), which produced
                            // `&u8` vs integer literal E0277. Non-Copy unknown vecs: annotate or use
                            // patterns that infer element type; E0507 is preferable to silent wrong refs.
                            false
                        });

                    if needs_borrow_or_clone {
                        if force_clone_for_owned_context {
                            return format!("{}.clone()", base_expr);
                        } else {
                            // Default: auto-borrow (zero-cost, idiomatic)
                            return format!("&{}", base_expr);
                        }
                    }
                }

                // `Vec<T>` / slice indexing in Rust already yields `T` for `T: Copy` in value
                // contexts (via the `Index` trait's desugaring). Emitting `*(vec[i])` was an
                // attempted E0308 workaround but is invalid: for `Copy` elements the inner
                // expression is already `T`, so `*` triggers E0614 for both owned and `&Vec<T>`
                // receivers.

                base_expr
            }
            Expression::Tuple {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();
                format!("({})", expr_strs.join(", "))
            }
            Expression::Array {
                elements: exprs, ..
            } => {
                let expr_strs: Vec<String> =
                    exprs.iter().map(|e| self.generate_expression(e)).collect();

                // WINDJAMMER PHILOSOPHY: Array literal syntax determines Rust output.
                //
                // In WJ, `[a, b, c]` is a fixed-size array literal → generates `[a, b, c]` in Rust.
                // In WJ, `vec![a, b, c]` is an explicit Vec constructor → generates `vec![a, b, c]`.
                //
                // Empty arrays `[]` remain `vec![]` because Rust's empty `[]` can't infer its type.
                //
                // This distinction is critical: `painter.line_segment([p1, p2], stroke)` expects
                // `[Pos2; 2]`, not `Vec<Pos2>`. The developer chose `[...]` syntax intentionally.
                if exprs.is_empty() {
                    // Empty array [] → vec![] (Vec::new())
                    // Rust's [] is a fixed-size array and can't infer type from later usage.
                    "vec![]".to_string()
                } else {
                    // Non-empty array literals: generate fixed-size array [a, b, c]
                    // The developer uses `vec![...]` macro syntax when Vec is needed.
                    format!("[{}]", expr_strs.join(", "))
                }
            }
            Expression::MacroInvocation {
                is_repeat,
                name,
                args,
                delimiter,
                ..
            } => {
                use crate::parser::MacroDelimiter;

                // PHASE 4 OPTIMIZATION: Check for format! with capacity hints
                if name == "format" {
                    if let Some(&capacity) =
                        self.string_capacity_hints.get(&self.current_statement_idx)
                    {
                        // Clone capacity to avoid borrow issues
                        let capacity_val = capacity;
                        // Generate optimized String::with_capacity + write! instead of format!
                        self.needs_write_import = true;
                        // write! expects the first argument to be a &str format template, not String.
                        let arg_strs: Vec<String> = if args.is_empty() {
                            Vec::new()
                        } else {
                            let prev_suppress = self.suppress_string_conversion;
                            self.suppress_string_conversion = true;
                            let fmt = self.generate_expression(args[0]);
                            self.suppress_string_conversion = prev_suppress;
                            let rest: Vec<String> = args[1..]
                                .iter()
                                .map(|e| self.generate_expression(e))
                                .collect();
                            let mut v = Vec::with_capacity(1 + rest.len());
                            v.push(fmt);
                            v.extend(rest);
                            v
                        };

                        return format!(
                            "{{\n{}    let mut __s = String::with_capacity({});\n{}    write!(&mut __s, {}).unwrap();\n{}    __s\n{}}}",
                            self.indent(),
                            capacity_val,
                            self.indent(),
                            arg_strs.join(", "),
                            self.indent(),
                            self.indent()
                        );
                    }
                }

                // Special case: if this is println!/eprintln!/print!/eprint! and first arg is format!, flatten it
                let should_flatten = (name == "println"
                    || name == "eprintln"
                    || name == "print"
                    || name == "eprint")
                    && !args.is_empty()
                    && matches!(&args[0], Expression::MacroInvocation { name: macro_name, .. } if macro_name == "format");

                // Macro arguments must never have context-level string coercion applied.
                // format!("...".to_string(), ...) is invalid Rust (requires literal first arg).
                let prev_coerce = self.coerce_string_literals_to_owned;
                self.coerce_string_literals_to_owned = false;
                let prev_match_arm = self.in_match_arm_needing_string;
                self.in_match_arm_needing_string = false;

                let arg_strs: Vec<String> = if should_flatten {
                    // Flatten format! macro arguments into the print macro
                    if let Expression::MacroInvocation {
                        is_repeat: _,
                        args: format_args,
                        ..
                    } = &args[0]
                    {
                        format_args
                            .iter()
                            .map(|e| self.generate_expression(e))
                            .collect()
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                } else {
                    // Special case: if this is println!/eprintln!/print!/eprint! with a single non-literal arg,
                    // wrap it with "{}" to make it valid Rust: println!(var) -> println!("{}", var)
                    // Also wrap format!() calls: println!(format!(...)) -> println!("{}", format!(...))
                    if (name == "println"
                        || name == "eprintln"
                        || name == "print"
                        || name == "eprint")
                        && args.len() == 1
                        && !matches!(
                            &args[0],
                            Expression::Literal {
                                value: Literal::String(_),
                                ..
                            }
                        )
                    {
                        vec!["\"{}\"".to_string(), self.generate_expression(args[0])]
                    } else {
                        args.iter().map(|e| self.generate_expression(e)).collect()
                    }
                };

                self.coerce_string_literals_to_owned = prev_coerce;
                self.in_match_arm_needing_string = prev_match_arm;

                let (open, close) = match delimiter {
                    MacroDelimiter::Parens => ("(", ")"),
                    MacroDelimiter::Brackets => ("[", "]"),
                    MacroDelimiter::Braces => ("{", "}"),
                };

                // WINDJAMMER FIX: vec![value; count] repeat syntax
                // The parser sets is_repeat=true for vec![x; n] syntax
                // Use semicolon for repeat, comma for regular args
                let separator = if *is_repeat { "; " } else { ", " };

                // WINDJAMMER FIX: String literal coercion in vec![]
                // In Windjammer, `string` maps to Rust `String`, so vec!["a", "b"] must
                // become vec!["a".to_string(), "b".to_string()] for Vec<String>.
                // Only apply when: macro is vec, brackets delimiter, has string literal args.
                let final_arg_strs: Vec<String> = if name == "vec"
                    && matches!(delimiter, MacroDelimiter::Brackets)
                    && !*is_repeat
                {
                    arg_strs
                        .iter()
                        .enumerate()
                        .map(|(idx, s)| {
                            // Check if the original arg is a string literal
                            if idx < args.len() {
                                if let Expression::Literal {
                                    value: Literal::String(_),
                                    ..
                                } = &args[idx]
                                {
                                    // Add .to_string() if not already present
                                    if !s.ends_with(".to_string()") {
                                        return format!("{}.to_string()", s);
                                    }
                                }
                            }
                            s.clone()
                        })
                        .collect()
                } else {
                    arg_strs
                };

                format!(
                    "{}!{}{}{}",
                    name,
                    open,
                    final_arg_strs.join(separator),
                    close
                )
            }
            Expression::Cast { expr, type_, .. } => {
                // Add parentheses around binary expressions for correct precedence
                // because `as` has higher precedence than arithmetic in Rust:
                // `a + b as usize` is parsed as `a + (b as usize)`, not `(a + b) as usize`
                let mut expr_str = match &**expr {
                    Expression::Binary { .. } => {
                        format!("({})", self.generate_expression(expr))
                    }
                    _ => self.generate_expression(expr),
                };
                // E0606 FIX: Cannot cast &T as U (e.g. &i32 as usize).
                // When the cast source is a borrowed parameter, auto-deref first.
                if let Expression::Identifier { name, .. } = &**expr {
                    let is_borrowed_param = self.inferred_borrowed_params.contains(name)
                        || self.current_function_params.iter().any(|p| {
                            p.name == *name
                                && matches!(
                                    p.ownership,
                                    OwnershipHint::Ref | OwnershipHint::Mut
                                )
                        });
                    if is_borrowed_param && !expr_str.starts_with('*') {
                        expr_str = format!("*{}", expr_str);
                    }
                }
                let type_str = self.type_to_rust(type_);
                format!("{} as {}", expr_str, type_str)
            }
            Expression::Block {
                statements: stmts,
                is_unsafe,
                ..
            } => {
                let old_in_unsafe = self.in_unsafe_block;
                if *is_unsafe {
                    self.in_unsafe_block = true;
                }
                let block_open = if *is_unsafe { "unsafe {\n" } else { "{\n" };
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
                        let needs_clone_for_match =
                            self.match_needs_clone_for_self_field(value, arms);

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
                            Self::return_type_expects_owned_string(&self.current_function_return_type)
                                || arms.iter().any(|arm| {
                                    string_analysis::expression_produces_string(arm.body)
                                        || arm_string_analysis::arm_returns_converted_string(
                                            arm.body,
                                        )
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
                                self.local_var_types.insert(var_name.clone(), var_type.clone());
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

                        for (arm, (arm_str, is_string_literal)) in
                            arms.iter().zip(arm_strings.iter())
                        {
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
                            let scrutinee_type_has_ref =
                                self.expression_type_contains_reference(value);
                            let match_binds_refs = scrutinee_needs_ref
                                || self.match_expression_binds_refs(value)
                                || scrutinee_type_has_ref;
                            if match_binds_refs && !final_arm_str.ends_with(".clone()") {
                                let mut bound_vars = std::collections::HashSet::new();
                                self.extract_pattern_bindings(&arm.pattern, &mut bound_vars);
                                let binding_name: Option<&str> = if let Expression::Identifier { name, .. } = arm.body {
                                    Some(name)
                                } else if let Expression::Block { statements, .. } = arm.body {
                                    if let Some(Statement::Expression { expr, .. }) = statements.last() {
                                        if statements.len() == 1 {
                                            if let Expression::Identifier { name, .. } = expr {
                                                Some(name)
                                            } else { None }
                                        } else { None }
                                    } else { None }
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
                                        let is_copy = bound_type.as_ref().is_some_and(|t| self.is_type_copy(t));
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
                if *is_unsafe {
                    self.in_void_block = false;
                }

                let len = stmts.len();
                for (i, stmt) in stmts.iter().enumerate() {
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

                self.indent_level -= 1;
                output.push_str(&self.indent());
                output.push('}');
                self.in_void_block = saved_void_block;
                self.in_unsafe_block = old_in_unsafe;
                output
            }
        }
    }

    /// f32/f64 suffix for a float literal on an assignment RHS when FloatInference is Unknown.
    /// Uses codegen's `infer_expression_type` (impl `self.field`, index elements, etc.).
    fn float_literal_suffix_from_assignment_lhs(ty: &Type) -> Option<&'static str> {
        fn peel_refs<'a>(ty: &'a Type) -> &'a Type {
            match ty {
                Type::Reference(inner) | Type::MutableReference(inner) => peel_refs(inner),
                t => t,
            }
        }
        match peel_refs(ty) {
            Type::Custom(name) if name == "f32" => Some("f32"),
            Type::Custom(name) if name == "f64" => Some("f64"),
            Type::Vec(inner) => Self::float_literal_suffix_from_assignment_lhs(inner),
            Type::Array(inner, _) => Self::float_literal_suffix_from_assignment_lhs(inner),
            _ => None,
        }
    }

    /// Helper: Extract float type from a Type (handles tuples, arrays, Vec, etc.)
    /// Searches recursively for float types, prioritizing f32 over f64.
    fn extract_float_type_from_context(ty: &Type) -> &str {
        match ty {
            Type::Custom(name) if name == "f32" => "f32",
            Type::Custom(name) if name == "f64" => "f64",
            Type::Vec(inner) => {
                // Recurse into Vec<T> to find float type
                Self::extract_float_type_from_context(inner)
            }
            Type::Tuple(types) => {
                // Search ALL tuple elements for f32 (prioritize f32 over f64)
                for t in types {
                    let float_ty = Self::extract_float_type_from_context(t);
                    if float_ty == "f32" {
                        return "f32"; // Found f32 anywhere in tuple
                    }
                }
                // Check again for f64
                for t in types {
                    let float_ty = Self::extract_float_type_from_context(t);
                    if float_ty == "f64" {
                        return "f64"; // Found f64 somewhere
                    }
                }
                "f64" // No float type found, default to f64
            }
            Type::Array(inner, _) => Self::extract_float_type_from_context(inner),
            _ => "f64",
        }
    }

    /// Enclosing function/slot expects owned `String` in Rust (`string` / `String` in Windjammer).
    pub(super) fn return_type_expects_owned_string(ret: &Option<Type>) -> bool {
        match ret {
            Some(Type::String) => true,
            Some(Type::Custom(n)) if n == "String" || n == "string" => true,
            _ => false,
        }
    }

    #[inline]
    fn should_coerce_string_literal_to_owned(&self) -> bool {
        !self.suppress_string_conversion
            && (self.in_match_arm_needing_string || self.coerce_string_literals_to_owned)
    }

    pub(super) fn generate_literal_with_context(
        &self,
        lit: &Literal,
        expr: &Expression<'ast>,
    ) -> String {
        // WINDJAMMER PHILOSOPHY: Expression-level type inference for literals
        // Int: Check IntInference first (i32, i64, u32, etc.)
        // Float: Check FloatInference (f32, f64)
        match lit {
            Literal::String(s) => {
                if s.is_empty() && self.should_coerce_string_literal_to_owned() {
                    // Use `"".to_string()` (not `String::new()`) so implicit-return / match-arm
                    // post-processing does not append another `.to_string()` (E0308 / redundant call).
                    "\"\".to_string()".to_string()
                } else {
                    let base = crate::codegen::rust::literals::generate_literal(lit);
                    if self.should_coerce_string_literal_to_owned() {
                        format!("{}.to_string()", base)
                    } else {
                        base
                    }
                }
            }
            Literal::IntSuffixed(i, suffix) => {
                format!("{}_{}", i, suffix)
            }
            Literal::Int(i) => {
                if let Some(inference) = &self.int_inference {
                    use crate::type_inference::IntType;
                    let inferred = inference.get_int_type(expr);
                    if inferred != IntType::Unknown {
                        let suffix = inferred.rust_suffix();
                        return format!("{}_{}", i, suffix);
                    }
                }
                crate::codegen::rust::literals::generate_literal(lit)
            }
            Literal::Float(f) => {
                // Priority 1: Use inference engine results (most accurate)
                if let Some(inference) = &self.float_inference {
                    use crate::type_inference::FloatType;
                    let inferred = inference.get_float_type(expr);

                    let suffix: Option<&str> = match inferred {
                        FloatType::F32 => Some("f32"),
                        FloatType::F64 => Some("f64"),
                        FloatType::Unknown => self
                            .assignment_float_target_type
                            .as_ref()
                            .and_then(Self::float_literal_suffix_from_assignment_lhs)
                            .or(Some("f32")),
                    };

                    if let Some(suffix) = suffix {
                        let s = f.to_string();
                        return if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                            format!("{}.0_{}", s, suffix)
                        } else {
                            format!("{}_{}", s, suffix)
                        };
                    }

                    return self.generate_literal_context_sensitive(lit);
                }

                // Priority 2: Fallback to old context-sensitive approach
                self.generate_literal_context_sensitive(lit)
            }
            _ => crate::codegen::rust::literals::generate_literal(lit),
        }
    }

    /// Old context-sensitive approach (fallback when inference not available)
    fn generate_literal_context_sensitive(&self, lit: &Literal) -> String {
        // WINDJAMMER PHILOSOPHY: Context-sensitive float type inference
        // The compiler should infer f32 vs f64 based on the surrounding context
        // to avoid ambiguous numeric type errors (Rust E0689)
        match lit {
            Literal::Float(f) => {
                // Priority 1: Struct field type (most specific)
                let float_type = if let (Some(struct_name), Some(field_name)) = (
                    &self.current_struct_literal_name,
                    &self.current_struct_field_name,
                ) {
                    if let Some(fields) = self.struct_field_types.get(struct_name) {
                        if let Some(field_type) = fields.get(field_name) {
                            Self::extract_float_type_from_context(field_type)
                        } else {
                            "f32"
                        }
                    } else {
                        "f32"
                    }
                // Priority 2: Function return type (handles tuples like (bool, f32))
                } else if let Some(return_type) = &self.current_function_return_type {
                    Self::extract_float_type_from_context(return_type)
                } else {
                    // Default: f32 — matches game/FFI-heavy dogfooding (avoids E0308 at API boundaries).
                    "f32"
                };

                let s = f.to_string();
                if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                    format!("{}.0_{}", s, float_type)
                } else {
                    format!("{}_{}", s, float_type)
                }
            }
            // For other literal types, delegate to canonical implementation
            _ => crate::codegen::rust::literals::generate_literal(lit),
        }
    }

    /// Generate literal without expression context (used in older code paths)
    pub(super) fn generate_literal(&self, lit: &Literal) -> String {
        // Delegate to context-sensitive version
        self.generate_literal_context_sensitive(lit)
    }

    /// Generate efficient string concatenation using format! macro
    fn generate_string_concat(
        &mut self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
    ) -> String {
        // Collect all parts of the concatenation chain
        let mut parts = Vec::new();
        string_analysis::collect_concat_parts_static(left, &mut parts);
        string_analysis::collect_concat_parts_static(right, &mut parts);

        // Generate format! macro call
        let format_str = "{}".repeat(parts.len());

        // Generate expressions for each part
        let mut args = Vec::new();
        for expr in &parts {
            args.push(self.generate_expression(expr));
        }

        format!("format!(\"{}\", {})", format_str, args.join(", "))
    }

    /// Parser emits `obj.method(args)` as `Call { function: FieldAccess(obj, method), args }`.
    /// Strip a leading `&ident` for collection key methods so `should_add_ref` can re-add `&` only when needed.
    fn strip_unary_ref_for_collection_key_arg<'a>(
        method: &str,
        param_idx: usize,
        arg: &'a Expression<'a>,
    ) -> &'a Expression<'a> {
        let is_key_method =
            super::stdlib_method_traits::is_map_key_method(method) && param_idx == 0;
        if !is_key_method {
            return arg;
        }
        if let Expression::Unary {
            op: crate::parser::UnaryOp::Ref,
            operand,
            ..
        } = arg
        {
            if matches!(&**operand, Expression::Identifier { .. }) {
                return operand;
            }
        }
        arg
    }

    /// `f32`/`f64` classification for binary operand codegen (inference + casts + WJ types).
    fn float_class_for_binary_operand(
        &self,
        expr: &Expression,
    ) -> Option<crate::type_inference::FloatType> {
        use crate::parser::Literal;
        use crate::type_inference::FloatType;
        let is_float_literal = matches!(
            expr,
            Expression::Literal {
                value: Literal::Float(_),
                ..
            }
        );
        if let Some(fi) = &self.float_inference {
            match fi.get_float_type(expr) {
                FloatType::F32 => return Some(FloatType::F32),
                FloatType::F64 => return Some(FloatType::F64),
                FloatType::Unknown => {
                    // `infer_expression_type` maps float literals to `Type::Float`, which we lower to
                    // Rust `f64` in signatures — but literal *codegen* often emits `_f32`. Treating
                    // that as F64 produced (F32, F64) and promoted the *left* f32 operand to f64
                    // (dogfooding: `x.sin() * 57.29_f32` → `sin() as f64 * …`), causing E0308/E0277.
                    if is_float_literal {
                        return None;
                    }
                }
            }
        } else if is_float_literal {
            return None;
        }
        if let Expression::Cast { type_, .. } = expr {
            if let Some(ft) = Self::float_type_from_wj_ty(type_) {
                return Some(ft);
            }
        }
        if let Some(ty) = self.infer_expression_type(expr) {
            if let Some(ft) = Self::float_type_from_wj_ty(&ty) {
                return Some(ft);
            }
        }
        // Operand may be `Type::Float` (no f32/f64 distinction) while children carry F32/F64 in
        // float inference — recurse so `(f32_expr) + 0.5_f64` and similar dogfooding patterns
        // still promote (E0277).
        if let Expression::Binary { left: l, right: r, .. } = expr {
            match (
                self.float_class_for_binary_operand(l),
                self.float_class_for_binary_operand(r),
            ) {
                (Some(a), Some(b)) if a == b => return Some(a),
                (Some(a), None) | (None, Some(a)) => return Some(a),
                _ => {}
            }
        }
        None
    }

    fn float_type_from_wj_ty(ty: &Type) -> Option<crate::type_inference::FloatType> {
        use crate::type_inference::FloatType;
        match ty {
            Type::Custom(n) if n == "f32" => Some(FloatType::F32),
            Type::Custom(n) if n == "f64" => Some(FloatType::F64),
            // `Type::Float` is the analyzer's generic "float" — it is not proof the value is f64.
            // Treating it as F64 made `(f32_expr, subexpr)` look like (F32, F64) and inserted
            // `f32_side as f64` while the other operand was still emitted as f32 → E0308.
            Type::Float => None,
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::float_type_from_wj_ty(inner)
            }
            _ => None,
        }
    }

    fn promote_mixed_f32_f64_operands(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
        prefer_cast_f64_to_f32_for_f32_assignment: bool,
    ) {
        use crate::parser::Literal;
        use crate::type_inference::FloatType;

        let left_float_lit = matches!(
            left,
            Expression::Literal {
                value: Literal::Float(_),
                ..
            }
        );
        let right_float_lit = matches!(
            right,
            Expression::Literal {
                value: Literal::Float(_),
                ..
            }
        );

        let mut lc = self.float_class_for_binary_operand(left);
        let mut rc = self.float_class_for_binary_operand(right);

        // GUARD: Non-scalar types (Vec3, Color, etc.) must NEVER be cast to f32/f64.
        // Float inference may classify expressions involving these types as F32 due to
        // float literal operands (e.g., `Vec3 * 0.5` → F32), but the result is still Vec3.
        // Clear float classification for any operand whose inferred type is a custom struct.
        {
            let is_non_scalar_custom = |expr: &Expression| -> bool {
                if let Some(ty) = self.infer_expression_type(expr) {
                    match &ty {
                        Type::Custom(name) => !matches!(name.as_str(),
                            "f32" | "f64" | "i32" | "u32" | "i64" | "u64"
                            | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                            | "bool" | "char"
                        ),
                        _ => false,
                    }
                } else {
                    false
                }
            };
            if is_non_scalar_custom(left) {
                lc = None;
            }
            if is_non_scalar_custom(right) {
                rc = None;
            }
        }

        // GUARD: Float inference may incorrectly classify integer expressions as F32/F64
        // (e.g., when an integer literal appears in a tuple alongside float elements).
        // If infer_expression_type says both operands are integers, clear the float classes
        // to prevent incorrect casting (dogfooding: `x + 1` where x: i32, 1: int).
        {
            let lt_actual = self.infer_expression_type(left);
            let rt_actual = self.infer_expression_type(right);
            let is_int_type = |t: &Type| {
                matches!(t, Type::Int)
                    || matches!(t, Type::Custom(n) if matches!(n.as_str(),
                        "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                    ))
            };
            let left_is_int = lt_actual.as_ref().is_some_and(|t| is_int_type(t));
            let right_is_int = rt_actual.as_ref().is_some_and(|t| is_int_type(t));
            // Both sides are integers: float inference is wrong, clear classifications
            if left_is_int && right_is_int && !left_float_lit && !right_float_lit {
                lc = None;
                rc = None;
            }
            // One side is integer but float inference classified it as float: clear that side
            if left_is_int && !left_float_lit && lc.is_some() {
                if !matches!(left, Expression::Cast { .. }) {
                    lc = None;
                }
            }
            if right_is_int && !right_float_lit && rc.is_some() {
                if !matches!(right, Expression::Cast { .. }) {
                    rc = None;
                }
            }
        }

        // Float literal with Unknown classification: match sibling so mixed ops still get casts
        // (dogfooding: `PI_f64 * ring as f32`, `f32_expr * 0.0005_f64`).
        if lc.is_none() && left_float_lit {
            lc = rc;
        }
        if rc.is_none() && right_float_lit {
            rc = lc;
        }

        // Float inference may mark a literal as F64 while literal codegen emits `_f32` in f32
        // context; with a definite F32 sibling that becomes (F32, F64) and we wrongly cast f32→f64.
        if right_float_lit && lc == Some(FloatType::F32) && rc == Some(FloatType::F64) {
            rc = Some(FloatType::F32);
        }
        if left_float_lit && rc == Some(FloatType::F32) && lc == Some(FloatType::F64) {
            lc = Some(FloatType::F32);
        }

        let cast_f32_to_f64 = |s: &str, e: &Expression| {
            let inner = if matches!(e, Expression::Binary { .. }) || s.contains(" as ") {
                format!("({})", s)
            } else {
                s.to_string()
            };
            format!("{} as f64", inner)
        };
        let cast_to_f32 = |s: &str, e: &Expression| {
            let inner = if matches!(e, Expression::Binary { .. }) || s.contains(" as ") {
                format!("({})", s)
            } else {
                s.to_string()
            };
            format!("{} as f32", inner)
        };
        // Compound / simple assignment to `f32`: keep arithmetic in f32 (cast f64 operand down).
        if prefer_cast_f64_to_f32_for_f32_assignment {
            match (lc, rc) {
                (Some(FloatType::F32), Some(FloatType::F64)) => {
                    *right_str = cast_to_f32(right_str, right);
                    return;
                }
                (Some(FloatType::F64), Some(FloatType::F32)) => {
                    *left_str = cast_to_f32(left_str, left);
                    return;
                }
                _ => {}
            }
        }
        // Both classified as f32 but a literal still has `_f64` suffix in generated Rust (inference
        // vs literal codegen mismatch) — cast the literal side down (mesh3d / trading patterns).
        if matches!((lc, rc), (Some(FloatType::F32), Some(FloatType::F32))) {
            if left_float_lit && left_str.contains("_f64") {
                *left_str = cast_to_f32(left_str, left);
                return;
            }
            if right_float_lit && right_str.contains("_f64") {
                *right_str = cast_to_f32(right_str, right);
                return;
            }
            // Cast + integer variable: one side is an explicit `as f32` Cast, the other is
            // an integer identifier that float inference marks f32 but codegen emits as integer.
            // Without the explicit cast, Rust rejects `f32 + i32` (E0277).
            let left_is_cast = matches!(left, Expression::Cast { .. });
            let right_is_cast = matches!(right, Expression::Cast { .. });
            if left_is_cast && !right_is_cast && !right_float_lit {
                *right_str = cast_to_f32(right_str, right);
                return;
            }
            if right_is_cast && !left_is_cast && !left_float_lit {
                *left_str = cast_to_f32(left_str, left);
                return;
            }
        }
        if matches!((lc, rc), (Some(FloatType::F64), Some(FloatType::F64))) {
            let left_is_cast = matches!(left, Expression::Cast { .. });
            let right_is_cast = matches!(right, Expression::Cast { .. });
            if left_is_cast && !right_is_cast && !right_float_lit {
                let inner = if matches!(right, Expression::Binary { .. })
                    || right_str.contains(" as ")
                {
                    format!("({})", right_str)
                } else {
                    right_str.to_string()
                };
                *right_str = format!("{} as f64", inner);
                return;
            }
            if right_is_cast && !left_is_cast && !left_float_lit {
                let inner = if matches!(left, Expression::Binary { .. })
                    || left_str.contains(" as ")
                {
                    format!("({})", left_str)
                } else {
                    left_str.to_string()
                };
                *left_str = format!("{} as f64", inner);
                return;
            }
        }
        match (lc, rc) {
            (Some(FloatType::F32), Some(FloatType::F64)) => {
                *left_str = cast_f32_to_f64(left_str, left);
            }
            (Some(FloatType::F64), Some(FloatType::F32)) => {
                *right_str = cast_f32_to_f64(right_str, right);
            }
            // Cast + integer: one operand is float (from `as f32` cast or inference),
            // the other has no float classification (integer variable). Cast the integer
            // side so Rust doesn't reject `f32 + i32` (E0277).
            //
            // GUARD: Float inference may wrongly classify integer variables as F32/F64
            // (e.g., `let r = self.brush.size` where size is i32 but inference says F32).
            // Only promote if we can CONFIRM the float side is genuinely float via:
            //   1. infer_expression_type returns a float type, OR
            //   2. The expression is an explicit Cast to float, OR
            //   3. The generated string already contains float markers
            (Some(ft), None) if !right_float_lit => {
                // Type::Float is generic "expression involves floats" — NOT proof the
                // result is f32/f64. E.g. `Vec3 * 0.5` yields Vec3, not f32, even though
                // float inference marks it F32 due to the literal.
                let is_confirmed_float = |t: &Type| {
                    matches!(t, Type::Custom(n) if n == "f32" || n == "f64")
                };
                let left_confirmed_float =
                    self.infer_expression_type(left).as_ref().is_some_and(|t| is_confirmed_float(t))
                    || matches!(left, Expression::Cast { type_, .. }
                        if matches!(type_, Type::Custom(n) if n == "f32" || n == "f64"))
                    || left_str.contains("_f32")
                    || left_str.contains("_f64")
                    || left_str.contains(" as f32")
                    || left_str.contains(" as f64");

                if !left_confirmed_float {
                    return;
                }

                // GUARD: Never cast non-scalar types (Vec3, Color, etc.) to float.
                let right_is_non_scalar = self.infer_expression_type(right).as_ref().is_some_and(|t| {
                    matches!(t, Type::Custom(n) if !matches!(n.as_str(),
                        "f32" | "f64" | "i32" | "u32" | "i64" | "u64"
                        | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                        | "bool" | "char"
                    ))
                });
                if right_is_non_scalar {
                    return;
                }

                let target = match ft {
                    FloatType::F32 => "f32",
                    FloatType::F64 => "f64",
                    _ => return,
                };
                let inner = if matches!(right, Expression::Binary { .. })
                    || right_str.contains(" as ")
                {
                    format!("({})", right_str)
                } else {
                    right_str.to_string()
                };
                *right_str = format!("{} as {}", inner, target);
            }
            (None, Some(ft)) if !left_float_lit => {
                let is_confirmed_float = |t: &Type| {
                    matches!(t, Type::Custom(n) if n == "f32" || n == "f64")
                };
                let right_confirmed_float =
                    self.infer_expression_type(right).as_ref().is_some_and(|t| is_confirmed_float(t))
                    || matches!(right, Expression::Cast { type_, .. }
                        if matches!(type_, Type::Custom(n) if n == "f32" || n == "f64"))
                    || right_str.contains("_f32")
                    || right_str.contains("_f64")
                    || right_str.contains(" as f32")
                    || right_str.contains(" as f64");

                if !right_confirmed_float {
                    return;
                }

                // GUARD: Never cast non-scalar types (Vec3, Color, etc.) to float.
                let left_is_non_scalar = self.infer_expression_type(left).as_ref().is_some_and(|t| {
                    matches!(t, Type::Custom(n) if !matches!(n.as_str(),
                        "f32" | "f64" | "i32" | "u32" | "i64" | "u64"
                        | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                        | "bool" | "char"
                    ))
                });
                if left_is_non_scalar {
                    return;
                }

                let target = match ft {
                    FloatType::F32 => "f32",
                    FloatType::F64 => "f64",
                    _ => return,
                };
                let inner = if matches!(left, Expression::Binary { .. })
                    || left_str.contains(" as ")
                {
                    format!("({})", left_str)
                } else {
                    left_str.to_string()
                };
                *left_str = format!("{} as {}", inner, target);
            }
            _ => {}
        }
    }

    fn promote_usize_i32_mixed_add_sub(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
    ) {
        let lt = self.infer_expression_type(left);
        let rt = self.infer_expression_type(right);
        let is_usize = |t: &Type| matches!(t, Type::Custom(n) if n == "usize");
        let is_i32 = |t: &Type| matches!(t, Type::Custom(n) if n == "i32");
        match (lt.as_ref(), rt.as_ref()) {
            (Some(l), Some(r)) if is_usize(l) && is_i32(r) => {
                *right_str =
                    if matches!(right, Expression::Binary { .. }) || right_str.contains(" as ") {
                        format!("({} as usize)", right_str)
                    } else {
                        format!("{} as usize", right_str)
                    };
            }
            (Some(l), Some(r)) if is_i32(l) && is_usize(r) => {
                *left_str =
                    if matches!(left, Expression::Binary { .. }) || left_str.contains(" as ") {
                        format!("({} as usize)", left_str)
                    } else {
                        format!("{} as usize", left_str)
                    };
            }
            _ => {}
        }
    }

    /// Auto-cast integer operands to float when mixed with float operands in arithmetic.
    /// Windjammer Philosophy: the compiler handles type conversions automatically.
    /// Rust rejects `i32 + f32`, `usize * f32`, etc. — we insert the cast.
    fn promote_int_to_float_in_mixed_arithmetic(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
    ) {
        fn is_integer_type(t: &Type) -> bool {
            match t {
                Type::Int => true,
                Type::Custom(n) => matches!(
                    n.as_str(),
                    "i32" | "u32" | "i64" | "u64" | "usize" | "isize" | "i8" | "u8" | "i16" | "u16"
                ),
                _ => false,
            }
        }
        fn is_float_type(t: &Type) -> bool {
            match t {
                Type::Float => true,
                Type::Custom(n) => matches!(n.as_str(), "f32" | "f64"),
                _ => false,
            }
        }
        fn float_target(t: &Type) -> &str {
            match t {
                Type::Custom(n) if n == "f64" => "f64",
                _ => "f32",
            }
        }
        fn cast_int_to_float(s: &str, expr: &Expression, target: &str) -> String {
            if s.contains(" as ") || matches!(expr, Expression::Binary { .. }) {
                format!("({}) as {}", s, target)
            } else {
                format!("{} as {}", s, target)
            }
        }

        let lt = self.infer_expression_type(left);
        let rt = self.infer_expression_type(right);

        match (lt.as_ref(), rt.as_ref()) {
            (Some(l), Some(r)) if is_integer_type(l) && is_float_type(r) => {
                let target = float_target(r);
                *left_str = cast_int_to_float(left_str, left, target);
            }
            (Some(l), Some(r)) if is_float_type(l) && is_integer_type(r) => {
                let target = float_target(l);
                *right_str = cast_int_to_float(right_str, right, target);
            }
            // One side is typed (int), other side is a float literal (generated with _f32/_f64)
            (Some(l), None) if is_integer_type(l) => {
                if right_str.contains("_f32") || right_str.ends_with("f32") {
                    *left_str = cast_int_to_float(left_str, left, "f32");
                } else if right_str.contains("_f64") || right_str.ends_with("f64") {
                    *left_str = cast_int_to_float(left_str, left, "f64");
                }
            }
            (None, Some(r)) if is_integer_type(r) => {
                if left_str.contains("_f32") || left_str.ends_with("f32") {
                    *right_str = cast_int_to_float(right_str, right, "f32");
                } else if left_str.contains("_f64") || left_str.ends_with("f64") {
                    *right_str = cast_int_to_float(right_str, right, "f64");
                }
            }
            _ => {}
        }
    }

    fn star_for_deref_compare(expr: &Expression, s: &str) -> String {
        if s.starts_with('*') {
            return s.to_string();
        }
        let inner = if matches!(expr, Expression::Binary { .. }) {
            format!("({})", s)
        } else {
            s.to_string()
        };
        format!("*{}", inner)
    }

    /// Fix E0277 `PartialEq` mismatches: `&T` vs `T` (Copy), `&u8` vs int literal.
    /// TDD FIX: Added handling for String == &String comparisons after changing
    /// borrowed parameters from &str to &String. String == &String doesn't work
    /// in Rust (no PartialEq impl), so we need to deref: String == *&String
    fn balance_eq_operands_for_rust(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
    ) {
        use crate::parser::Literal;
        let lt = self.infer_expression_type(left);
        let rt = self.infer_expression_type(right);

        // TDD FIX: Handle explicit * deref of &String in comparisons
        // Problem: User writes *id == flag_id where both could be &String or mixed
        // Case 1: id: &String, flag_id: &String → Remove * → id == flag_id (both &String) ✓
        // Case 2: id: &String, flag_id: String → Keep * or add to other → *id == flag_id (String == String) ✓
        // Solution: Check if BOTH operands are borrowed strings, only then remove *
        
        let left_is_explicit_deref = matches!(left, Expression::Unary { op: crate::parser::UnaryOp::Deref, .. });
        let right_is_explicit_deref = matches!(right, Expression::Unary { op: crate::parser::UnaryOp::Deref, .. });
        
        // Helper: Check if an identifier is a borrowed string
        let is_borrowed_string_identifier = |expr: &Expression| -> bool {
            if let Expression::Identifier { name, .. } = expr {
                // Check if it's a borrowed parameter
                let is_borrowed_param = self.inferred_borrowed_params.contains(name.as_str())
                    && self.current_function_params.iter().any(|p| {
                        p.name == *name && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    });
                
                // Check if it's a borrowed iterator variable
                let is_borrowed_iter = self.borrowed_iterator_vars.contains(name)
                    && self.local_var_types.get(name.as_str())
                        .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t));
                
                is_borrowed_param || is_borrowed_iter
            } else {
                false
            }
        };
        
        // Helper: Check if an identifier is an owned string (from local vars, not borrowed)
        let is_owned_string_identifier = |expr: &Expression| -> bool {
            if let Expression::Identifier { name, .. } = expr {
                // Check if it's a local variable (not borrowed param/iter)
                let is_local_var = self.local_var_types.get(name.as_str())
                    .is_some_and(|t| crate::codegen::rust::types::is_windjammer_text_type(t))
                    && !self.inferred_borrowed_params.contains(name.as_str())
                    && !self.borrowed_iterator_vars.contains(name);
                is_local_var
            } else {
                false
            }
        };
        
        // Check the operands of explicit deref expressions
        let left_deref_operand_borrowed = if let Expression::Unary { op: crate::parser::UnaryOp::Deref, operand, .. } = left {
            is_borrowed_string_identifier(operand)
        } else {
            false
        };
        
        let left_deref_operand_owned = if let Expression::Unary { op: crate::parser::UnaryOp::Deref, operand, .. } = left {
            is_owned_string_identifier(operand)
        } else {
            false
        };
        
        let right_deref_operand_borrowed = if let Expression::Unary { op: crate::parser::UnaryOp::Deref, operand, .. } = right {
            is_borrowed_string_identifier(operand)
        } else {
            false
        };
        
        let right_deref_operand_owned = if let Expression::Unary { op: crate::parser::UnaryOp::Deref, operand, .. } = right {
            is_owned_string_identifier(operand)
        } else {
            false
        };
        
        // Check if non-deref side is borrowed
        let left_is_borrowed = if !left_is_explicit_deref {
            is_borrowed_string_identifier(left)
        } else {
            false
        };
        
        let right_is_borrowed = if !right_is_explicit_deref {
            is_borrowed_string_identifier(right)
        } else {
            false
        };
        
        // Remove * ONLY when comparing two borrowed strings
        // If left has *id (borrowed) and right is borrowed param/iter → Remove *
        if left_is_explicit_deref && left_deref_operand_borrowed && right_is_borrowed {
            // Both sides are borrowed strings, remove the *
            if left_str.starts_with("(*") && left_str.ends_with(')') {
                *left_str = left_str[2..left_str.len()-1].to_string();
            } else if left_str.starts_with('*') {
                *left_str = left_str[1..].to_string();
            }
        }
        
        if right_is_explicit_deref && right_deref_operand_borrowed && left_is_borrowed {
            // Both sides are borrowed strings, remove the *
            if right_str.starts_with("(*") && right_str.ends_with(')') {
                *right_str = right_str[2..right_str.len()-1].to_string();
            } else if right_str.starts_with('*') {
                *right_str = right_str[1..].to_string();
            }
        }
        
        // Handle mixed cases: one side has explicit deref, other doesn't
        // Case A: *borrowed == borrowed → Remove * (both &String)
        // Case B: *borrowed == owned → Add * to owned (*borrowed → &str, need owned → String for deref)
        // Case C: *owned == borrowed → Add * to borrowed (*owned → String, need borrowed → String for deref)
        
        // Case B: left is *borrowed (&str after deref), right is borrowed (&String) → Add * to right
        if left_is_explicit_deref && left_deref_operand_borrowed && right_is_borrowed && !right_is_explicit_deref {
            // This contradicts Case A, so skip (already handled above by removing *)
        }
        
        // Case C: left is *owned (String after deref), right is borrowed (&String) → Add * to right
        if left_is_explicit_deref && left_deref_operand_owned && right_is_borrowed && !right_is_explicit_deref {
            // left: *id (String), right: borrowed_param (&String) → Add * to right
            if !right_str.starts_with('*') {
                *right_str = format!("*{}", right_str);
            }
        }
        
        // Mirror cases for right side
        if right_is_explicit_deref && right_deref_operand_owned && left_is_borrowed && !left_is_explicit_deref {
            // right: *id (String), left: borrowed_param (&String) → Add * to left
            if !left_str.starts_with('*') {
                *left_str = format!("*{}", left_str);
            }
        }
        
        let is_text = |t: Option<&Type>| {
            t.is_some_and(|t| {
                crate::codegen::rust::types::is_windjammer_text_type(t)
                    || matches!(
                        t,
                        Type::Reference(inner)
                            if crate::codegen::rust::types::is_windjammer_text_type(inner)
                    )
            })
        };

        // TDD FIX: Handle String == &String comparisons (after &str → &String change)
        // Rust has: String==&str, &str==String, &String==&str
        // Rust LACKS: String==&String, &String==String
        // So we need to deref the &String side
        if is_text(lt.as_ref()) && is_text(rt.as_ref()) {
            // Check if type is owned String (not &String, not &str)
            let is_owned_string = |t: Option<&Type>| -> bool {
                match t {
                    Some(Type::String) => true,
                    Some(Type::Custom(s)) => s == "String" || s == "string",
                    _ => false,
                }
            };
            
            // Check if expression is an explicit deref (*x) that produces owned String
            let is_explicit_deref_string = |expr: &Expression| -> bool {
                if let Expression::Unary { op: crate::parser::UnaryOp::Deref, operand, .. } = expr {
                    // *operand produces String if operand is &String
                    if let Some(operand_type) = self.infer_expression_type(operand) {
                        return matches!(operand_type, Type::Reference(inner) 
                            if crate::codegen::rust::types::is_windjammer_text_type(&inner));
                    }
                }
                false
            };
            
            // Check if expression is a borrowed string parameter
            let is_borrowed_string_identifier = |expr: &Expression| -> bool {
                if let Expression::Identifier { name, .. } = expr {
                    return self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                            && self.inferred_borrowed_params.contains(name.as_str())
                    });
                }
                false
            };
            
            // Check if type is &String reference
            let is_ref_type = |t: Option<&Type>| -> bool {
                matches!(t, Some(Type::Reference(inner)) 
                    if crate::codegen::rust::types::is_windjammer_text_type(inner))
            };
            
            // Check if type is &String (not String, not &str)
            let is_ref_string = |t: Option<&Type>| -> bool {
                match t {
                    Some(Type::Reference(inner)) => match &**inner {
                        Type::String => true,
                        Type::Custom(s) => s == "String" || s == "string",
                        _ => false,
                    },
                    _ => false,
                }
            };
            
            // TDD FIX: Also check expressions directly for borrowed parameters or match arm bindings
            // When type inference fails for field access, check if right/left is a borrowed string parameter
            let is_borrowed_string_param = |expr: &Expression| -> bool {
                if let Expression::Identifier { name, .. } = expr {
                    // Check current function params for &String parameters (Borrowed ownership)
                    self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                            && self.inferred_borrowed_params.contains(name.as_str())
                    })
                } else {
                    false
                }
            };
            
            // Check if identifier is an owned String variable (match arm binding, local var)
            let is_owned_string_var = |expr: &Expression| -> bool {
                if let Expression::Identifier { name, .. } = expr {
                    // Check local_var_types for owned String
                    if let Some(var_type) = self.local_var_types.get(name.as_str()) {
                        return crate::codegen::rust::types::is_windjammer_text_type(var_type);
                    }
                }
                false
            };
            
            // Determine ownership explicitly, being careful about match arm bindings
            let left_is_owned = is_owned_string(lt.as_ref()) || is_owned_string_var(left);
            let right_is_owned = is_owned_string(rt.as_ref()) || is_owned_string_var(right);
            let left_is_ref = is_ref_string(lt.as_ref());
            let right_is_ref = is_ref_string(rt.as_ref());
            let right_is_borrowed_param = is_borrowed_string_param(right);
            let left_is_borrowed_param = is_borrowed_string_param(left);
            let left_type_unknown = lt.is_none();
            let right_type_unknown = rt.is_none();
            
            // TDD FIX: ONLY deref when we're CERTAIN about type mismatch
            // Rules (from most specific to most general):
            
            // Rule 0: One unknown + one &String (likely closure param + match arm binding)
            // Both are probably &String, so NO deref
            if left_type_unknown && right_is_ref && !right_is_owned {
                return; // &String == &String works natively
            }
            if right_type_unknown && left_is_ref && !left_is_owned {
                return; // &String == &String works natively
            }
            
            // Rule 1: Both types KNOWN and mismatch
            // String (owned, known) == &String (ref, known) → deref right
            if left_is_owned && !left_type_unknown && (right_is_ref || right_is_borrowed_param) && !right_type_unknown {
                *right_str = Self::star_for_deref_compare(right, right_str);
                return;
            }
            // &String (ref, known) == String (owned, known) → deref left
            if (left_is_ref || left_is_borrowed_param) && !left_type_unknown && right_is_owned && !right_type_unknown {
                *left_str = Self::star_for_deref_compare(left, left_str);
                return;
            }
            
            // Rule 2: One borrowed param (known), one unknown
            // Unknown closure params default to &T, so deref the borrowed param side
            if left_is_borrowed_param && !left_type_unknown && right_type_unknown {
                *left_str = Self::star_for_deref_compare(left, left_str);
                return;
            }
            if right_is_borrowed_param && !right_type_unknown && left_type_unknown {
                *right_str = Self::star_for_deref_compare(right, right_str);
                return;
            }
            
            // Rule 3: Both types unknown (closure params, etc.)
            // Trust Rust's PartialEq - no deref needed
            if left_type_unknown && right_type_unknown {
                return;
            }
            
            // All other string combinations work natively (&str, etc.)
            return;
        }

        fn peel_reference_layer<'a>(t: &'a Type) -> &'a Type {
            match t {
                Type::Reference(inner) => inner.as_ref(),
                _ => t,
            }
        }
        let lhs_base = lt.as_ref().map(peel_reference_layer);
        let rhs_base = rt.as_ref().map(peel_reference_layer);
        let left_is_ref = matches!(lt.as_ref(), Some(Type::Reference(_)));
        let right_is_ref = matches!(rt.as_ref(), Some(Type::Reference(_)));

        if let (Some(lb), Some(rb)) = (lhs_base, rhs_base) {
            if lb == rb && self.is_type_copy(lb) {
                if left_is_ref && !right_is_ref {
                    *left_str = Self::star_for_deref_compare(left, left_str);
                    return;
                }
                if right_is_ref && !left_is_ref {
                    *right_str = Self::star_for_deref_compare(right, right_str);
                    return;
                }
            }
        }

        if let Some(Type::Reference(inner)) = lt.as_ref() {
            if self.is_type_copy(inner)
                && matches!(
                    right,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                )
            {
                *left_str = Self::star_for_deref_compare(left, left_str);
            }
        }
    }

    /// Check if expression traces to self (self.field, self.field.subfield, etc.)
    /// Used for E0507 Option::map fix - self.children.map() needs .as_ref()
    fn codegen_expression_traces_to_self(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FieldAccess { object, .. } => {
                matches!(&**object, Expression::Identifier { name, .. } if name == "self")
                    || self.codegen_expression_traces_to_self(object)
            }
            Expression::Index { object, .. } => self.codegen_expression_traces_to_self(object),
            _ => false,
        }
    }
}

//! Binary expression generation
//!
//! Handles code generation for binary operations including:
//! - Arithmetic operators (+, -, *, /, %)
//! - Comparison operators (<, <=, >, >=, ==, !=)
//! - Logical operators (&&, ||)
//! - Bitwise operators (&, |, ^, <<, >>)
//! - Type balancing and promotion
//! - Operator precedence handling

use crate::parser::{BinaryOp, Expression, Literal, Type, UnaryOp};

use super::{operators, string_analysis, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for a binary expression
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn generate_binary_expression(
        &mut self,
        left: &Expression<'ast>,
        op: &BinaryOp,
        right: &Expression<'ast>,
    ) -> String {
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
            if matches!(method.as_str(), "len" | "capacity" | "count") && arguments.is_empty() {
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
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Eq | BinaryOp::Ne
        );
        let is_arithmetic = matches!(
            op,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
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
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
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
                let needs_parens = operators::op_precedence(left_op) < operators::op_precedence(op)
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
            (left_str, right_str) =
                super::type_casting::cast_for_usize_binary_op(&left_str, &right_str, true, false);
        } else if is_comparison
            && right_is_usize
            && !left_is_usize
            && !left_is_int_literal
            && self.comparison_other_side_needs_len_as_i64(left)
        {
            (left_str, right_str) =
                super::type_casting::cast_for_usize_binary_op(&left_str, &right_str, false, true);
        }
        // If both are usize: no cast (usize == usize is fine)
        // If neither is usize: no cast (i64 == i64 is fine)

        // AUTO-CAST: When doing arithmetic between usize and int literal, Rust infers
        // the literal type from context. So `items.len() - 1` works without casting.
        // Only cast if the literal is negative (usize can't represent negative values).
        if is_arithmetic && left_is_usize && right_is_int_literal && !right_is_usize {
            let is_negative =
                matches!(right, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
            if is_negative {
                right_str = format!("{} as usize", right_str);
            }
        } else if is_arithmetic && right_is_usize && left_is_int_literal && !left_is_usize {
            let is_negative =
                matches!(left, Expression::Literal { value: Literal::Int(n), .. } if *n < 0);
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
                && ((left_is_usize && right_is_int_literal && !right_is_usize)
                    || (right_is_usize && left_is_int_literal && !left_is_usize));
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
                                if left_ty != promoted && is_safe_implicit_cast(left_ty, promoted) {
                                    let suffix = get_cast_suffix(promoted);
                                    let needs_inner = matches!(left, Expression::Binary { .. })
                                        || left_str.contains(" as ");
                                    left_str = if needs_inner {
                                        format!("({}) as {}", left_str, suffix)
                                    } else {
                                        format!("{} as {}", left_str, suffix)
                                    };
                                }
                                if right_ty != promoted && is_safe_implicit_cast(right_ty, promoted)
                                {
                                    let suffix = get_cast_suffix(promoted);
                                    let needs_inner = matches!(right, Expression::Binary { .. })
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
            self.promote_usize_i32_mixed_add_sub(left, right, &mut left_str, &mut right_str);
        }

        // E0277: mixed f32/f64 (inference + `as f32` vs default `_f64` literals).
        if (is_arithmetic || is_comparison)
            && matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
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
        let needs_cast_parens_for_op = matches!(op_str, "<" | ">" | "<<" | ">>" | "|" | "&" | "^");
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
                        && (
                            // Explicit &str type (Type::Reference(Custom("str")))
                            matches!(&p.type_, Type::Reference(inner)
                                    if matches!(&**inner, Type::Custom(s) if s == "str"))
                                // Inferred borrowed string (Type::String with inferred borrow)
                                || ((matches!(p.type_, Type::String)
                                    || matches!(p.type_, Type::Custom(ref n) if n == "string"))
                                    && self.inferred_borrowed_params.contains(name))
                        )
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
                    super::rust_stdlib_annotations::is_strip_redundant(method)
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
                    super::rust_stdlib_annotations::is_strip_redundant(method)
                }
                _ => false,
            };

            // Check if one side is an explicit deref of a borrowed value
            // Example: *id == flag_id where id is &String
            let left_is_explicit_deref = matches!(
                left,
                Expression::Unary {
                    op: UnaryOp::Deref,
                    ..
                }
            );
            let right_is_explicit_deref = matches!(
                right,
                Expression::Unary {
                    op: UnaryOp::Deref,
                    ..
                }
            );

            // TDD FIX for E0614: Check if either side is a match arm binding (owned value)
            let left_is_match_binding = if let Expression::Identifier { name, .. } = left {
                self.match_arm_bindings.contains(name.as_str())
            } else {
                false
            };
            let right_is_match_binding = if let Expression::Identifier { name, .. } = right {
                self.match_arm_bindings.contains(name.as_str())
            } else {
                false
            };

            // TDD FIX: Check if either side is an explicit &str parameter (has explicit & in source)
            // These should NEVER get * derefs - Rust handles &str == &String natively
            let left_is_explicit_str_ref = if let Expression::Identifier { name, .. } = left {
                self.current_function_params.iter().any(|p| {
                    let matches_name = p.name == *name;
                    let is_ref_ownership = matches!(p.ownership, crate::parser::OwnershipHint::Ref);
                    // Check if the inner type (after removing Reference) is a text type
                    let is_text = match &p.type_ {
                        Type::Reference(inner) => {
                            crate::codegen::rust::types::is_windjammer_text_type(inner)
                        }
                        _ => crate::codegen::rust::types::is_windjammer_text_type(&p.type_),
                    };
                    matches_name && is_ref_ownership && is_text
                })
            } else {
                false
            };
            let right_is_explicit_str_ref = if let Expression::Identifier { name, .. } = right {
                self.current_function_params.iter().any(|p| {
                    let matches_name = p.name == *name;
                    let is_ref_ownership = matches!(p.ownership, crate::parser::OwnershipHint::Ref);
                    // Check if the inner type (after removing Reference) is a text type
                    let is_text = match &p.type_ {
                        Type::Reference(inner) => {
                            crate::codegen::rust::types::is_windjammer_text_type(inner)
                        }
                        _ => crate::codegen::rust::types::is_windjammer_text_type(&p.type_),
                    };
                    matches_name && is_ref_ownership && is_text
                })
            } else {
                false
            };

            // TDD FIX: XOR logic for borrowed/owned mismatch ONLY when BOTH sides are tracked
            // Skip when one side is untracked (closure param, etc.) - likely BOTH are borrowed
            // ALSO skip when one side is explicit deref - handle in balance_eq_operands_for_rust
            // ALSO skip when one side is match arm binding that is NOT borrowed (owned Copy values)
            // Match arm bindings from ref scrutinee (e.g. match &self) ARE borrowed — deref them.
            // ALSO skip when one side is explicit &str parameter - Rust handles &str comparisons natively
            let left_skip_match = left_is_match_binding && !left_is_borrowed;
            let right_skip_match = right_is_match_binding && !right_is_borrowed;
            if left_is_tracked
                && right_is_tracked
                && left_is_borrowed != right_is_borrowed
                && !left_is_explicit_deref
                && !right_is_explicit_deref
                && !left_skip_match
                && !right_skip_match
                && !left_is_explicit_str_ref
                && !right_is_explicit_str_ref
            {
                if left_is_borrowed {
                    left_str = format!("*{}", left_str);
                } else {
                    right_str = format!("*{}", right_str);
                }
            }
        } // end is_comparison guard

        // TDD FIX for E0614: Call balance_eq for ALL comparisons, not just == and !=
        // This handles match arm bindings (owned Copy types like i32) in >=, <=, >, < too
        if is_comparison {
            self.balance_eq_operands_for_rust(left, right, &mut left_str, &mut right_str);
            left_str = self.peel_copy_ref_match_binding_for_value(left, &left_str);
            right_str = self.peel_copy_ref_match_binding_for_value(right, &right_str);
        }

        // Auto-deref borrowed bool operands in logical ops (&&, ||).
        // Rust requires `bool`, not `&bool`, for these operators.
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            let deref_if_borrowed_bool = |expr: &Expression, s: &str, gen: &Self| -> String {
                if let Expression::Identifier { name, .. } = expr {
                    if (gen.inferred_borrowed_params.contains(name.as_str())
                        || gen.borrowed_iterator_vars.contains(name))
                        && !s.starts_with('*')
                    {
                        return format!("*{}", s);
                    }
                }
                s.to_string()
            };
            left_str = deref_if_borrowed_bool(left, &left_str, self);
            right_str = deref_if_borrowed_bool(right, &right_str, self);
        }

        format!("{} {} {}", left_str, op_str, right_str)
    }
}

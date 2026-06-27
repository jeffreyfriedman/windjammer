//! Type balancing and promotion for Rust codegen
//!
//! Handles automatic type conversions and comparisons:
//! - Float type promotion (f32 ↔ f64)
//! - Mixed integer/float arithmetic (i32 + f32 → f32)
//! - Equality operator type balancing (String == &String, &T == T)
//! - Reference/deref normalization for comparisons

use crate::parser::{Expression, Literal, OwnershipHint, Type};
use crate::type_inference::FloatType;

use super::{
    expression_utilities, float_type_utilities, type_classification_utilities, CodeGenerator,
};

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn float_class_for_binary_operand(
        &self,
        expr: &Expression,
    ) -> Option<FloatType> {
        // Shadowed loop counters (e.g. inner `dx: i32` vs outer `dx: f32`) must use the
        // inner binding from local_var_types — float inference is name-based and can misclassify.
        if let Expression::Identifier { name, .. } = expr {
            if let Some(ty) = self.local_var_types.get(name) {
                if Self::is_int_numeric_type(ty) {
                    return None;
                }
            }
        }

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
            if let Some(ft) = float_type_utilities::float_type_from_wj_ty(type_) {
                return Some(ft);
            }
        }
        if let Some(ty) = self.infer_expression_type(expr) {
            if let Some(ft) = float_type_utilities::float_type_from_wj_ty(&ty) {
                return Some(ft);
            }
        }
        // Operand may be `Type::Float` (no f32/f64 distinction) while children carry F32/F64 in
        // float inference — recurse so `(f32_expr) + 0.5_f64` and similar dogfooding patterns
        // still promote (E0277).
        if let Expression::Binary {
            left: l, right: r, ..
        } = expr
        {
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

    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn promote_mixed_f32_f64_operands(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
        prefer_cast_f64_to_f32_for_f32_assignment: bool,
    ) {
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
                        Type::Custom(name) => !matches!(
                            name.as_str(),
                            "f32"
                                | "f64"
                                | "i32"
                                | "u32"
                                | "i64"
                                | "u64"
                                | "usize"
                                | "isize"
                                | "i8"
                                | "u8"
                                | "i16"
                                | "u16"
                                | "bool"
                                | "char"
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
                    || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
            };
            let left_is_int = lt_actual.as_ref().is_some_and(&is_int_type);
            let right_is_int = rt_actual.as_ref().is_some_and(is_int_type);
            // Both sides are integers: float inference is wrong, clear classifications
            if left_is_int && right_is_int && !left_float_lit && !right_float_lit {
                lc = None;
                rc = None;
            }
            // One side is integer but float inference classified it as float: clear that side
            if left_is_int
                && !left_float_lit
                && lc.is_some()
                && !matches!(left, Expression::Cast { .. })
            {
                lc = None;
            }
            if right_is_int
                && !right_float_lit
                && rc.is_some()
                && !matches!(right, Expression::Cast { .. })
            {
                rc = None;
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

        let deref_if_ref_for_cast = |this: &Self, s: &str, e: &Expression| -> String {
            if let Expression::Identifier { name, .. } = e {
                if !s.starts_with('*')
                    && (this.local_var_types.get(name).is_some_and(|t| {
                        matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    }) || this.borrowed_iterator_vars.contains(name)
                        || this.inferred_borrowed_params.contains(name))
                {
                    return format!("*{}", s);
                }
            }
            if !s.starts_with('*') {
                if let Some(Type::Reference(inner) | Type::MutableReference(inner)) =
                    this.infer_expression_type(e)
                {
                    if this.is_type_copy(inner.as_ref()) {
                        return format!("*{}", s);
                    }
                }
            }
            if matches!(e, Expression::Binary { .. }) || s.contains(" as ") {
                format!("({})", s)
            } else {
                s.to_string()
            }
        };
        let cast_f32_to_f64 = |s: &str, e: &Expression| {
            let inner = deref_if_ref_for_cast(self, s, e);
            format!("{} as f64", inner)
        };
        let cast_to_f32 = |s: &str, e: &Expression| {
            let inner = deref_if_ref_for_cast(self, s, e);
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
                let inner =
                    if matches!(right, Expression::Binary { .. }) || right_str.contains(" as ") {
                        format!("({})", right_str)
                    } else {
                        right_str.to_string()
                    };
                *right_str = format!("{} as f64", inner);
                return;
            }
            if right_is_cast && !left_is_cast && !left_float_lit {
                let inner =
                    if matches!(left, Expression::Binary { .. }) || left_str.contains(" as ") {
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
                let is_int_type = |t: &Type| {
                    matches!(t, Type::Int)
                        || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                };
                let right_is_int = self
                    .infer_expression_type(right)
                    .as_ref()
                    .is_some_and(is_int_type)
                    || if let Expression::Identifier { name, .. } = right {
                        self.local_var_types.get(name).is_some_and(is_int_type)
                    } else {
                        false
                    };
                if right_is_int {
                    return;
                }
                let left_is_int = self
                    .infer_expression_type(left)
                    .as_ref()
                    .is_some_and(is_int_type)
                    || if let Expression::Identifier { name, .. } = left {
                        self.local_var_types.get(name).is_some_and(is_int_type)
                    } else {
                        false
                    };
                if left_is_int {
                    return;
                }

                // Type::Float is generic "expression involves floats" — NOT proof the
                // result is f32/f64. E.g. `Vec3 * 0.5` yields Vec3, not f32, even though
                // float inference marks it F32 due to the literal.
                let is_confirmed_float =
                    |t: &Type| matches!(t, Type::Custom(n) if n == "f32" || n == "f64");
                let left_confirmed_float = self
                    .infer_expression_type(left)
                    .as_ref()
                    .is_some_and(is_confirmed_float)
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
                let right_is_non_scalar =
                    self.infer_expression_type(right).as_ref().is_some_and(|t| {
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
                *right_str = if target == "f32" {
                    cast_to_f32(right_str, right)
                } else {
                    cast_f32_to_f64(right_str, right)
                };
            }
            (None, Some(ft)) if !left_float_lit => {
                let is_int_type = |t: &Type| {
                    matches!(t, Type::Int)
                        || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
                };
                let right_is_int = self
                    .infer_expression_type(right)
                    .as_ref()
                    .is_some_and(is_int_type)
                    || if let Expression::Identifier { name, .. } = right {
                        self.local_var_types.get(name).is_some_and(is_int_type)
                    } else {
                        false
                    };
                if right_is_int {
                    return;
                }
                let left_is_int = self
                    .infer_expression_type(left)
                    .as_ref()
                    .is_some_and(is_int_type)
                    || if let Expression::Identifier { name, .. } = left {
                        self.local_var_types.get(name).is_some_and(is_int_type)
                    } else {
                        false
                    };
                if left_is_int {
                    return;
                }

                let is_confirmed_float =
                    |t: &Type| matches!(t, Type::Custom(n) if n == "f32" || n == "f64");
                let right_confirmed_float = self
                    .infer_expression_type(right)
                    .as_ref()
                    .is_some_and(is_confirmed_float)
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
                let left_is_non_scalar =
                    self.infer_expression_type(left).as_ref().is_some_and(|t| {
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
                *left_str = if target == "f32" {
                    cast_to_f32(left_str, left)
                } else {
                    cast_f32_to_f64(left_str, left)
                };
            }
            _ => {}
        }
    }

    pub(in crate::codegen::rust) fn promote_usize_i32_mixed_add_sub(
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
    /// Resolve operand type for int/float promotion, preferring inner `local_var_types`
    /// over global float inference (shadowed `dx: i32` vs outer `dx: f32`).
    fn effective_type_for_mixed_arithmetic(&self, expr: &Expression) -> Option<Type> {
        if let Expression::Identifier { name, .. } = expr {
            if let Some(ty) = self.local_var_types.get(name) {
                return Some(ty.clone());
            }
        }
        self.infer_expression_type(expr)
    }

    /// Windjammer Philosophy: the compiler handles type conversions automatically.
    /// Rust rejects `i32 + f32`, `usize * f32`, etc. — we insert the cast.
    pub(in crate::codegen::rust) fn promote_int_to_float_in_mixed_arithmetic(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
    ) {
        let lt = self.effective_type_for_mixed_arithmetic(left);
        let rt = self.effective_type_for_mixed_arithmetic(right);

        match (lt.as_ref(), rt.as_ref()) {
            (Some(l), Some(r))
                if type_classification_utilities::is_integer_type(l)
                    && type_classification_utilities::is_float_type(r) =>
            {
                let target = type_classification_utilities::float_target(r);
                *left_str =
                    type_classification_utilities::cast_int_to_float(left_str, left, target);
            }
            (Some(l), Some(r))
                if type_classification_utilities::is_float_type(l)
                    && type_classification_utilities::is_integer_type(r) =>
            {
                let target = type_classification_utilities::float_target(l);
                *right_str =
                    type_classification_utilities::cast_int_to_float(right_str, right, target);
            }
            // One side is typed (int), other side is a float literal (generated with _f32/_f64)
            (Some(l), None) if type_classification_utilities::is_integer_type(l) => {
                if right_str.contains("_f32") || right_str.ends_with("f32") {
                    *left_str =
                        type_classification_utilities::cast_int_to_float(left_str, left, "f32");
                } else if right_str.contains("_f64") || right_str.ends_with("f64") {
                    *left_str =
                        type_classification_utilities::cast_int_to_float(left_str, left, "f64");
                }
            }
            (None, Some(r)) if type_classification_utilities::is_integer_type(r) => {
                if left_str.contains("_f32") || left_str.ends_with("f32") {
                    *right_str =
                        type_classification_utilities::cast_int_to_float(right_str, right, "f32");
                } else if left_str.contains("_f64") || left_str.ends_with("f64") {
                    *right_str =
                        type_classification_utilities::cast_int_to_float(right_str, right, "f64");
                }
            }
            // One side is typed float, other side is unresolved (integer literal with no type)
            (Some(l), None) if type_classification_utilities::is_float_type(l) => {
                let is_int_literal = matches!(
                    right,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );
                if is_int_literal {
                    let target = type_classification_utilities::float_target(l);
                    *right_str =
                        type_classification_utilities::cast_int_to_float(right_str, right, target);
                }
            }
            (None, Some(r)) if type_classification_utilities::is_float_type(r) => {
                let is_int_literal = matches!(
                    left,
                    Expression::Literal {
                        value: Literal::Int(_),
                        ..
                    }
                );
                if is_int_literal {
                    let target = type_classification_utilities::float_target(r);
                    *left_str =
                        type_classification_utilities::cast_int_to_float(left_str, left, target);
                }
            }
            _ => {}
        }
    }

    /// Fix E0277 `PartialEq` mismatches: `&T` vs `T` (Copy), `&u8` vs int literal.
    /// TDD FIX: Added handling for String == &String comparisons after changing
    /// borrowed parameters from &str to &String. String == &String doesn't work
    /// in Rust (no PartialEq impl), so we need to deref: String == *&String
    #[allow(clippy::too_many_lines)]
    pub(in crate::codegen::rust) fn balance_eq_operands_for_rust(
        &self,
        left: &Expression<'ast>,
        right: &Expression<'ast>,
        left_str: &mut String,
        right_str: &mut String,
    ) {
        let lt = self.infer_expression_type(left);
        let rt = self.infer_expression_type(right);

        // TDD FIX: Handle explicit * deref of &String in comparisons
        // Problem: User writes *id == flag_id where both could be &String or mixed
        // Case 1: id: &String, flag_id: &String → Remove * → id == flag_id (both &String) ✓
        // Case 2: id: &String, flag_id: String → Keep * or add to other → *id == flag_id (String == String) ✓
        // Solution: Check if BOTH operands are borrowed strings, only then remove *

        let left_is_explicit_deref = matches!(
            left,
            Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                ..
            }
        );
        let right_is_explicit_deref = matches!(
            right,
            Expression::Unary {
                op: crate::parser::UnaryOp::Deref,
                ..
            }
        );

        // Helper: Check if an identifier is a borrowed string
        let is_borrowed_string_identifier = |expr: &Expression| -> bool {
            if let Expression::Identifier { name, .. } = expr {
                // Check if it's a borrowed parameter
                let is_borrowed_param = self.inferred_borrowed_params.contains(name.as_str())
                    && self.current_function_params.iter().any(|p| {
                        p.name == *name
                            && crate::codegen::rust::types::is_windjammer_text_type(&p.type_)
                    });

                // Check if it's a borrowed iterator variable
                let is_borrowed_iter = self.borrowed_iterator_vars.contains(name)
                    && self
                        .local_var_types
                        .get(name.as_str())
                        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);

                is_borrowed_param || is_borrowed_iter
            } else {
                false
            }
        };

        // Helper: Check if an identifier is an owned string (from local vars, not borrowed)
        let is_owned_string_identifier = |expr: &Expression| -> bool {
            if let Expression::Identifier { name, .. } = expr {
                // Check if it's a local variable (not borrowed param/iter)
                let is_local_var = self
                    .local_var_types
                    .get(name.as_str())
                    .is_some_and(crate::codegen::rust::types::is_windjammer_text_type)
                    && !self.inferred_borrowed_params.contains(name.as_str())
                    && !self.borrowed_iterator_vars.contains(name);
                is_local_var
            } else {
                false
            }
        };

        // Check the operands of explicit deref expressions
        let left_deref_operand_borrowed = if let Expression::Unary {
            op: crate::parser::UnaryOp::Deref,
            operand,
            ..
        } = left
        {
            is_borrowed_string_identifier(operand)
        } else {
            false
        };

        let left_deref_operand_owned = if let Expression::Unary {
            op: crate::parser::UnaryOp::Deref,
            operand,
            ..
        } = left
        {
            is_owned_string_identifier(operand)
        } else {
            false
        };

        let right_deref_operand_borrowed = if let Expression::Unary {
            op: crate::parser::UnaryOp::Deref,
            operand,
            ..
        } = right
        {
            is_borrowed_string_identifier(operand)
        } else {
            false
        };

        let right_deref_operand_owned = if let Expression::Unary {
            op: crate::parser::UnaryOp::Deref,
            operand,
            ..
        } = right
        {
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
                *left_str = left_str[2..left_str.len() - 1].to_string();
            } else if left_str.starts_with('*') {
                *left_str = left_str[1..].to_string();
            }
        }

        if right_is_explicit_deref && right_deref_operand_borrowed && left_is_borrowed {
            // Both sides are borrowed strings, remove the *
            if right_str.starts_with("(*") && right_str.ends_with(')') {
                *right_str = right_str[2..right_str.len() - 1].to_string();
            } else if right_str.starts_with('*') {
                *right_str = right_str[1..].to_string();
            }
        }

        // Handle mixed cases: one side has explicit deref, other doesn't
        // Case A: *borrowed == borrowed → Remove * (both &String)
        // Case B: *borrowed == owned → Add * to owned (*borrowed → &str, need owned → String for deref)
        // Case C: *owned == borrowed → Add * to borrowed (*owned → String, need borrowed → String for deref)

        // Case B: left is *borrowed (&str after deref), right is borrowed (&String) → Add * to right
        if left_is_explicit_deref
            && left_deref_operand_borrowed
            && right_is_borrowed
            && !right_is_explicit_deref
        {
            // This contradicts Case A, so skip (already handled above by removing *)
        }

        // Case C: left is *owned (String after deref), right is borrowed (&String) → Add * to right
        if left_is_explicit_deref
            && left_deref_operand_owned
            && right_is_borrowed
            && !right_is_explicit_deref
        {
            // left: *id (String), right: borrowed_param (&String) → Add * to right
            if !right_str.starts_with('*') {
                *right_str = format!("*{}", right_str);
            }
        }

        // Mirror cases for right side
        if right_is_explicit_deref
            && right_deref_operand_owned
            && left_is_borrowed
            && !left_is_explicit_deref
        {
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
            // TDD FIX: Check for explicit &str parameters FIRST (before any other logic)
            // Remove any * derefs that were added - Rust's PartialEq handles &str naturally
            let left_is_explicit_str = if let Expression::Identifier { name, .. } = left {
                self.current_function_params.iter().any(|p| {
                    let matches_name = p.name == *name;
                    let is_ref_ownership = matches!(p.ownership, OwnershipHint::Ref);
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

            let right_is_explicit_str = if let Expression::Identifier { name, .. } = right {
                self.current_function_params.iter().any(|p| {
                    let matches_name = p.name == *name;
                    let is_ref_ownership = matches!(p.ownership, OwnershipHint::Ref);
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

            // Also check Phase 2 &str-optimized parameters (inferred borrowed string → &str)
            let left_is_str_optimized = if let Expression::Identifier { name, .. } = left {
                self.str_ref_optimized_params.contains(name.as_str())
            } else {
                false
            };
            let right_is_str_optimized = if let Expression::Identifier { name, .. } = right {
                self.str_ref_optimized_params.contains(name.as_str())
            } else {
                false
            };

            if left_is_explicit_str
                || right_is_explicit_str
                || left_is_str_optimized
                || right_is_str_optimized
            {
                // &str parameters compare naturally with everything - no deref logic needed
                return;
            }

            // Check if type is owned String (not &String, not &str)
            let is_owned_string = |t: Option<&Type>| -> bool {
                match t {
                    Some(Type::String) => true,
                    Some(Type::Custom(s)) => s == "String" || s == "string",
                    _ => false,
                }
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

            // Rule -1: Explicit &str parameters and Phase 2 &str-optimized params NEVER need deref
            // Rust's PartialEq handles: &str == &String, &str == &str, &str == String
            if left_is_explicit_str
                || right_is_explicit_str
                || left_is_str_optimized
                || right_is_str_optimized
            {
                return; // &str compares natively with everything
            }

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
            if left_is_owned
                && !left_type_unknown
                && (right_is_ref || right_is_borrowed_param)
                && !right_type_unknown
            {
                *right_str = expression_utilities::star_for_deref_compare(right, right_str);
                return;
            }
            // &String (ref, known) == String (owned, known) → deref left
            if (left_is_ref || left_is_borrowed_param)
                && !left_type_unknown
                && right_is_owned
                && !right_type_unknown
            {
                *left_str = expression_utilities::star_for_deref_compare(left, left_str);
                return;
            }

            // Rule 2: One borrowed param (known), one unknown
            // Unknown closure params default to &T, so deref the borrowed param side
            if left_is_borrowed_param && !left_type_unknown && right_type_unknown {
                *left_str = expression_utilities::star_for_deref_compare(left, left_str);
                return;
            }
            if right_is_borrowed_param && !right_type_unknown && left_type_unknown {
                *right_str = expression_utilities::star_for_deref_compare(right, right_str);
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

        let lhs_base = lt
            .as_ref()
            .map(type_classification_utilities::peel_reference_layer);
        let rhs_base = rt
            .as_ref()
            .map(type_classification_utilities::peel_reference_layer);
        let left_is_ref = matches!(lt.as_ref(), Some(Type::Reference(_)));
        let right_is_ref = matches!(rt.as_ref(), Some(Type::Reference(_)));

        let match_binding_skips_copy_deref = |expr: &Expression| -> bool {
            let Expression::Identifier { name, .. } = expr else {
                return false;
            };
            if !self.match_arm_bindings.contains(name.as_str()) {
                return false;
            }
            // Owned-scrutinee bindings stay `T`; only skip deref for those, not `&T` from `match &self`.
            !self
                .local_var_types
                .get(name.as_str())
                .is_some_and(|ty| matches!(ty, Type::Reference(_) | Type::MutableReference(_)))
        };

        if let (Some(lb), Some(rb)) = (lhs_base, rhs_base) {
            if lb == rb && self.is_type_copy(lb) {
                if left_is_ref && !right_is_ref && !match_binding_skips_copy_deref(right) {
                    *left_str = expression_utilities::star_for_deref_compare(left, left_str);
                    return;
                }
                if right_is_ref && !left_is_ref && !match_binding_skips_copy_deref(left) {
                    *right_str = expression_utilities::star_for_deref_compare(right, right_str);
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
                *left_str = expression_utilities::star_for_deref_compare(left, left_str);
            }
        }

        // Note: Explicit &str parameters are handled by the early return in the string comparison block above
        // No cleanup needed here since they never get * derefs in the first place
    }
}

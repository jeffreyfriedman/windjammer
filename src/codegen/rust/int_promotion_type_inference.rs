//! Mixed-integer promotion for `as T` codegen.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Literal, Type};
use crate::type_inference::IntType;

impl<'ast> CodeGenerator<'ast> {
    /// When let/assign/range set `assignment_int_target_type`, mixed-int ops unify to that width.
    pub(in crate::codegen::rust) fn promotion_int_type_from_assignment_context(
        &self,
    ) -> Option<IntType> {
        self.assignment_int_target_type
            .as_ref()
            .and_then(Self::parser_type_to_promotion_int_type)
            .filter(|t| *t != IntType::Unknown && *t != IntType::Usize)
    }

    /// P3.309: i32 loop counter vs int literal / i32 field — do not widen RHS to i64.
    pub(in crate::codegen::rust) fn comparison_should_prefer_i32_over_i64(
        &self,
        i32_side: &Expression<'ast>,
        i64_side: &Expression<'ast>,
    ) -> bool {
        if matches!(
            i64_side,
            Expression::Literal {
                value: Literal::Int(_),
                ..
            }
        ) {
            return true;
        }
        if self.expression_promotes_to_i32_in_compare(i32_side) {
            if let Expression::Identifier { name, .. } = i64_side {
                if matches!(self.local_var_types.get(name.as_str()), Some(Type::Int)) {
                    return true;
                }
            }
        }
        self.int_type_for_mixed_int_codegen(i64_side) == IntType::I32
            || self
                .infer_expression_type(i64_side)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
                })
    }

    fn expression_promotes_to_i32_in_compare(&self, expr: &Expression<'ast>) -> bool {
        self.int_type_for_mixed_int_codegen(expr) == IntType::I32
            || self
                .infer_expression_type(expr)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Int32) || matches!(t, Type::Custom(n) if n == "i32")
                })
    }

    /// P3.323 / P3.326: untyped `let mut x = 0` may emit `_i32` / `_usize` while
    /// `local_var_types` still holds WJ `Int` from a non-int return hint (e.g. `-> string`).
    /// Sync the binding to the emitted width so later assigns do not `as i64` a usize RHS.
    pub(in crate::codegen::rust) fn reconcile_ambiguous_int_local_after_let(
        &mut self,
        name: &str,
        value: &Expression<'ast>,
        emitted_rhs: &str,
    ) {
        let ambiguous_int = match self.local_var_types.get(name) {
            Some(Type::Int) | Some(Type::Int32) => true,
            Some(Type::Custom(n)) => matches!(n.as_str(), "int" | "i64" | "i32"),
            _ => false,
        };
        if !ambiguous_int {
            return;
        }
        // P3.326: numeric inference may emit `0_usize` for index locals while the let
        // still recorded WJ `Int` (String/bool return width hint). Treat as usize.
        if emitted_rhs.ends_with("_usize")
            || self.int_type_for_mixed_int_codegen(value) == IntType::Usize
        {
            self.local_var_types
                .insert(name.to_string(), Type::Custom("usize".into()));
            self.usize_variables.insert(name.to_string());
            return;
        }
        if emitted_rhs.ends_with("_i32")
            || self.int_type_for_mixed_int_codegen(value) == IntType::I32
        {
            self.local_var_types.insert(name.to_string(), Type::Int32);
        }
    }

    /// P3.323: untyped `let mut i = 0` + `while i < 4` — treat counter as i32, not WJ `int`/i64.
    pub(in crate::codegen::rust) fn promote_ambiguous_int_loop_counter_in_while_condition(
        &mut self,
        condition: &Expression<'ast>,
    ) {
        use crate::parser::BinaryOp;
        let Expression::Binary { left, right, op, .. } = condition else {
            return;
        };
        if !matches!(
            op,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        ) {
            return;
        }
        let ident_name =
            |expr: &Expression<'ast>, lit: &Expression<'ast>| -> Option<String> {
                match (expr, lit) {
                    (
                        Expression::Identifier { name, .. },
                        Expression::Literal {
                            value: Literal::Int(n),
                            ..
                        },
                    ) if (0..=4096).contains(n) => Some(name.to_string()),
                    (
                        Expression::Literal {
                            value: Literal::Int(n),
                            ..
                        },
                        Expression::Identifier { name, .. },
                    ) if (0..=4096).contains(n) => Some(name.to_string()),
                    _ => None,
                }
            };
        let Some(name) = ident_name(left, right).or_else(|| ident_name(right, left)) else {
            return;
        };
        if matches!(self.local_var_types.get(name.as_str()), Some(Type::Int)) {
            self.local_var_types.insert(name, Type::Int32);
        }
    }


    /// P3.324: u32 locals / fields vs untyped int literals — do not widen to u64.
    pub(in crate::codegen::rust) fn comparison_should_prefer_u32_over_u64(
        &self,
        u32_side: &Expression<'ast>,
        u64_side: &Expression<'ast>,
    ) -> bool {
        if matches!(
            u64_side,
            Expression::Literal {
                value: Literal::Int(n),
                ..
            } if *n >= 0
        ) {
            return true;
        }
        if self.expression_promotes_to_u32_in_compare(u32_side) {
            if let Expression::Identifier { name, .. } = u64_side {
                if matches!(self.local_var_types.get(name.as_str()), Some(Type::Int))
                    || matches!(
                        self.local_var_types.get(name.as_str()),
                        Some(Type::Custom(n)) if n == "uint"
                    )
                {
                    return true;
                }
            }
        }
        self.int_type_for_mixed_int_codegen(u64_side) == IntType::U32
            || self
                .infer_expression_type(u64_side)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Uint)
                        || matches!(t, Type::Custom(n) if n == "u32")
                })
    }

    fn expression_promotes_to_u32_in_compare(&self, expr: &Expression<'ast>) -> bool {
        self.int_type_for_mixed_int_codegen(expr) == IntType::U32
            || self
                .infer_expression_type(expr)
                .as_ref()
                .is_some_and(|t| {
                    matches!(t, Type::Uint)
                        || matches!(t, Type::Custom(n) if n == "u32")
                })
    }

    /// Operand type for driving int literal suffixes in binary ops (`idx + 1` → `1_usize`).
    pub(in crate::codegen::rust) fn peer_type_for_int_literal_operand(
        &self,
        expr: &Expression<'ast>,
    ) -> Option<Type> {
        if let Expression::Identifier { name, .. } = expr {
            // usize index/len counters beat return-inferred Int32 (P3.311/P3.314).
            if self.usize_variables.contains(name) {
                return Some(Type::Custom("usize".into()));
            }
            if let Some(w) = self.local_int_rust_type_name_excluding_ambiguous_int(name) {
                return Self::parser_type_from_rust_int_name(w);
            }
        }
        if self.infer_expression_type_is_usize(expr) {
            return Some(Type::Custom("usize".into()));
        }
        // P3.322: ambiguous WJ `int` locals may emit as i32 while `local_var_types` stays `Int`.
        if self.int_type_for_mixed_int_codegen(expr) == crate::type_inference::IntType::I32 {
            return Some(Type::Int32);
        }
        self.infer_expression_type(expr).filter(|t| {
            Self::assignment_target_needs_int_codegen_context(t)
                && Self::int_type_from_assignment_target(t).is_some()
        })
    }

    pub(in crate::codegen::rust) fn expr_is_usize_loop_counter(
        &self,
        expr: &Expression<'ast>,
    ) -> bool {
        matches!(expr, Expression::Identifier { name, .. } if self.usize_variables.contains(name))
    }

    /// Map parser [`Type`] to [`crate::type_inference::IntType`] for mixed-integer `as T` codegen.
    pub(in crate::codegen::rust) fn parser_type_to_promotion_int_type(
        ty: &Type,
    ) -> Option<crate::type_inference::IntType> {
        use crate::type_inference::IntType;
        match ty {
            // Windjammer `int` is i64 in Rust codegen — never promote as i32 (P3.267).
            Type::Int => Some(IntType::I64),
            Type::Int32 => Some(IntType::I32),
            Type::Uint => Some(IntType::U32),
            Type::Custom(name) => match name.as_str() {
                "i8" => Some(IntType::I8),
                "i16" => Some(IntType::I16),
                "i32" => Some(IntType::I32),
                "i64" => Some(IntType::I64),
                "isize" => Some(IntType::Isize),
                "u8" => Some(IntType::U8),
                "u16" => Some(IntType::U16),
                "u32" => Some(IntType::U32),
                "u64" => Some(IntType::U64),
                "usize" => Some(IntType::Usize),
                _ => None,
            },
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::parser_type_to_promotion_int_type(inner.as_ref())
            }
            _ => None,
        }
    }

    /// Integer kind for binary mixed-type casts: use annotated types for params/fields when
    /// inference disagrees (e.g. `a: u32` must not be treated as `i32`).
    pub(in crate::codegen::rust) fn int_type_for_mixed_int_codegen(
        &self,
        expr: &Expression<'ast>,
    ) -> crate::type_inference::IntType {
        let eng = if let Some(ni) = &self.numeric_inference {
            ni.get_int_type(expr)
        } else {
            return crate::type_inference::IntType::Unknown;
        };
        match expr {
            Expression::Identifier { name, .. } => {
                if self.usize_variables.contains(name) {
                    return crate::type_inference::IntType::Usize;
                }
                if let Some(t) = self.local_var_types.get(name.as_str()) {
                    if let Some(a) = Self::parser_type_to_promotion_int_type(t) {
                        if a == IntType::I64 {
                            if let Some(w) = self.local_int_rust_type_name_excluding_ambiguous_int(name)
                            {
                                if w == "i32" {
                                    return IntType::I32;
                                }
                            }
                            if eng == IntType::I32 {
                                return IntType::I32;
                            }
                        }
                        return a;
                    }
                }
                if let Some(a) = self
                    .infer_expression_type(expr)
                    .as_ref()
                    .and_then(Self::parser_type_to_promotion_int_type)
                {
                    return a;
                }
                eng
            }
            Expression::FieldAccess { .. } => {
                // For field accesses, prefer codegen type inference over int inference
                // engine. If the codegen can determine the field type (via struct_field_types),
                // use it. If not (e.g., ambiguous struct names across modules), return
                // Unknown to prevent incorrect casts. The int inference engine may resolve
                // field types through a different struct with the same name, producing
                // wrong results.
                if self.expression_produces_usize(expr) {
                    return crate::type_inference::IntType::Usize;
                }
                if let Some(a) = self
                    .infer_expression_type(expr)
                    .as_ref()
                    .and_then(Self::parser_type_to_promotion_int_type)
                {
                    return a;
                }
                crate::type_inference::IntType::Unknown
            }
            _ => {
                if self.expression_produces_usize(expr) {
                    return crate::type_inference::IntType::Usize;
                }
                eng
            }
        }
    }
}

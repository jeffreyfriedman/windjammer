//! Mixed-integer promotion for `as T` codegen.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Type};

impl<'ast> CodeGenerator<'ast> {
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

//! Mixed-integer promotion for `as T` codegen.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Type};

impl<'ast> CodeGenerator<'ast> {
    /// Map parser [`Type`] to [`crate::type_inference::IntType`] for mixed-integer `as T` codegen.
    pub(in crate::codegen::rust) fn parser_type_to_promotion_int_type(
        ty: &Type,
    ) -> Option<crate::type_inference::IntType> {
        use crate::type_inference::IntType;
        match ty {
            Type::Int => Some(IntType::I32),
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
    /// IntInference disagrees (e.g. `a: u32` must not be treated as `i32`).
    pub(in crate::codegen::rust) fn int_type_for_mixed_int_codegen(
        &self,
        expr: &Expression<'ast>,
        inference: &crate::type_inference::IntInference,
    ) -> crate::type_inference::IntType {
        let eng = inference.get_int_type(expr);
        match expr {
            Expression::Identifier { .. } => {
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
                if let Some(a) = self
                    .infer_expression_type(expr)
                    .as_ref()
                    .and_then(Self::parser_type_to_promotion_int_type)
                {
                    return a;
                }
                crate::type_inference::IntType::Unknown
            }
            _ => eng,
        }
    }
}

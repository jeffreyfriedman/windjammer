//! The integer width an assignment target expects (`self.field` from the struct's declared fields,
//! a local from its recorded type), used to convert a `usize` value at the assignment (W5, until the
//! numeric solver owns returns and assignments; ADR-0028).

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Type};

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn get_assignment_target_type(
        &self,
        target: &Expression,
    ) -> Option<String> {
        match target {
            Expression::FieldAccess { object, field, .. } => {
                if matches!(&**object, Expression::Identifier { name, .. } if name == "self") {
                    if let Some(struct_name) = &self.current_struct_name {
                        let base_name = struct_name.split('<').next().unwrap_or(struct_name);
                        if let Some(usize_fields) = self.usize_struct_fields.get(base_name) {
                            if usize_fields.contains(field) {
                                return Some("usize".to_string());
                            }
                        }
                        if let Some(fields) = self.lookup_struct_field_types(base_name) {
                            if let Some(ty) = fields.get(field.as_str()) {
                                if matches!(ty, Type::Custom(n) if n == "usize") {
                                    return Some("usize".to_string());
                                }
                                if matches!(ty, Type::Custom(n) if n == "f32") {
                                    return Some("f32".to_string());
                                }
                                if matches!(ty, Type::Custom(n) if n == "i32") {
                                    return Some("i32".to_string());
                                }
                                if matches!(ty, Type::Custom(n) if n == "i64" || n == "int") {
                                    return Some("i64".to_string());
                                }
                            }
                        }
                    }
                }
            }
            Expression::Identifier { name, .. } => {
                // Signed WJ `int` / i64 locals beat `usize_variables` back-prop
                // (`let mut colon_at = -1` then `colon_at = j`).
                if let Some(ty) = self.local_var_types.get(name) {
                    if crate::codegen::rust::type_casting::type_is_wj_int_formal(ty)
                        || matches!(ty, Type::Int32)
                    {
                        return Self::assignment_cast_target_for_type(ty);
                    }
                }
                if self.identifier_is_wj_int_i64_binding(name) {
                    return Some("int".to_string());
                }
                if self.usize_variables.contains(name) {
                    return Some("usize".to_string());
                }
                if let Some(ty) = self.local_var_types.get(name) {
                    return Self::assignment_cast_target_for_type(ty);
                }
                return None;
            }
            _ => {}
        }
        None
    }

    fn assignment_cast_target_for_type(ty: &Type) -> Option<String> {
        match Self::peel_option_result_payload(ty) {
            Type::Int => Some("int".to_string()),
            Type::Custom(name) if name == "int" || name == "i64" => Some("int".to_string()),
            Type::Custom(name) if name == "i32" => Some("i32".to_string()),
            Type::Custom(name) if name == "usize" => Some("usize".to_string()),
            Type::Int32 => Some("i32".to_string()),
            _ => None,
        }
    }

    /// W5 at an assignment: a `usize` value stored into an `int` / `i32` target is converted.
    pub(in crate::codegen::rust) fn cast_usize_to_assignment_target(
        &self,
        value_str: &mut String,
        value: &Expression<'ast>,
        target: &Expression,
    ) {
        let target_type = self.get_assignment_target_type(target);
        self.maybe_cast_usize_to_int_target(value_str, value, target_type.as_deref());
    }

    /// W5 at a match arm: a `usize` arm value in a function returning `int` / `i32` is converted.
    pub(in crate::codegen::rust) fn cast_usize_arm_to_return_int(&self, arm_str: &mut String, arm_body: &Expression<'ast>) {
        let target = match &self.current_function_return_type {
            Some(Type::Int) => Some("int"),
            Some(Type::Custom(name)) if name == "i64" || name == "int" => Some("int"),
            Some(Type::Custom(name)) if name == "i32" => Some("i32"),
            _ => None,
        };
        if let Some(t) = target {
            self.maybe_cast_usize_to_int_target(arm_str, arm_body, Some(t));
        }
    }
}

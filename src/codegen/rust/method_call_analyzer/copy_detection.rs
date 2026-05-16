//! Copy-type detection for method arguments and parameter annotations.

use crate::parser::{Expression, Parameter, Type};
use std::collections::HashSet;

use super::MethodCallAnalyzer;

impl MethodCallAnalyzer {
    /// Check if expression represents a Copy type
    pub fn is_copy_type(
        arg: &Expression,
        usize_variables: &HashSet<String>,
        current_function_params: &[Parameter],
    ) -> bool {
        match arg {
            Expression::Identifier { name, .. } => {
                if usize_variables.contains(name) {
                    return true;
                }

                if current_function_params.iter().any(|p| {
                    if &p.name == name {
                        if let Type::Custom(t) = &p.type_ {
                            return crate::type_classification::is_copy_primitive(t);
                        }
                    }
                    false
                }) {
                    return true;
                }

                false
            }
            Expression::FieldAccess { .. } => false,
            _ => false,
        }
    }

    /// Public wrapper for [`Self::is_copy_type_annotation_internal`].
    pub fn is_copy_type_annotation_pub(type_: &Type) -> bool {
        Self::is_copy_type_annotation_internal(type_)
    }

    pub(super) fn is_copy_type_annotation_internal(type_: &Type) -> bool {
        match type_ {
            Type::Custom(name) => {
                crate::type_classification::is_copy_primitive(name) || name == "int"
            }
            Type::Reference(_) | Type::MutableReference(_) => true,
            _ => false,
        }
    }
}

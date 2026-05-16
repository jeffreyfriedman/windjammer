//! Copy-ness classification (`is_copy_type`) for ownership inference.

use crate::parser::Type;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
            Type::Reference(_) => true,
            Type::MutableReference(_) => false,
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            Type::Option(inner) => self.is_copy_type(inner),
            Type::Result(ok_type, err_type) => {
                self.is_copy_type(ok_type) && self.is_copy_type(err_type)
            }
            Type::Array(inner, _size) => self.is_copy_type(inner),
            Type::Vec(_) => false,
            Type::FunctionPointer { .. } => true,
            Type::RawPointer { .. } => true,
            Type::Custom(name) => {
                if self.copy_enums.contains(name) {
                    return true;
                }
                if self.copy_structs.contains(name) {
                    return true;
                }
                crate::type_classification::is_copy_primitive(name)
            }
            _ => false,
        }
    }
}

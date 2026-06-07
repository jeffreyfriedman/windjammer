//! Resolves expressions to Rust type names and iterator element types.

use crate::codegen::rust::CodeGenerator;
use crate::parser::{Expression, Type};

#[allow(clippy::collapsible_match, clippy::collapsible_if)]
impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn peeled_collection_element_type(ty: &Type) -> Option<&Type> {
        let mut t = ty;
        loop {
            match t {
                Type::Reference(inner) | Type::MutableReference(inner) => t = inner.as_ref(),
                Type::Vec(inner) => return Some(inner.as_ref()),
                Type::Array(inner, _) => return Some(inner.as_ref()),
                Type::Parameterized(name, params) if (name == "Vec" || name == "VecDeque") && !params.is_empty() => {
                    return Some(&params[0]);
                }
                _ => return None,
            }
        }
    }

    /// BUG #8 FIX: Infer the type name from an expression
    /// This enables qualified method signature lookup (Type::method)
    pub(in crate::codegen::rust) fn infer_type_name(&self, expr: &Expression) -> Option<String> {
        match expr {
            Expression::Identifier { name, .. } => {
                // "self" refers to the current struct type
                if name == "self" && self.in_impl_block {
                    return self.current_struct_name.clone();
                }
                // Try to infer from struct name if we're in an impl block
                if self.in_impl_block {
                    if let Some(struct_name) = &self.current_struct_name {
                        if self.current_struct_fields.contains(name) {
                            return Some(struct_name.clone());
                        }
                    }
                }
                // TDD FIX: Check function parameters for type info
                // e.g., fn test(validator: Validator) → infer_type_name("validator") = "Validator"
                for param in &self.current_function_params {
                    if param.name == *name {
                        if let Some(tn) = Self::type_to_name(&param.type_) {
                            if tn == "Self" && self.in_impl_block {
                                return self.current_struct_name.clone();
                            }
                            return Some(tn);
                        }
                    }
                }
                // TDD FIX: Check local variable types
                // e.g., let stack = Stack { .. } → infer_type_name("stack") = "Stack"
                if let Some(var_type) = self.local_var_types.get(name) {
                    if let Some(tn) = Self::type_to_name(var_type) {
                        if tn == "Self" && self.in_impl_block {
                            return self.current_struct_name.clone();
                        }
                        return Some(tn);
                    }
                }
                // TDD FIX: Recognize CamelCase identifiers as type names for static method calls.
                // In `Builder::create("hello")`, `Builder` is a type name, not a variable.
                // Without this, signature lookup fails and string literal coercion is wrong.
                if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                    return Some(name.clone());
                }
                None
            }
            Expression::FieldAccess { object, field, .. } => {
                // TDD FIX: Try to resolve field type from struct field type tracking
                // e.g., self.transforms → World.transforms → ComponentArray<int> → "ComponentArray"
                let owner_type = self.infer_type_name(object);
                if let Some(ref owner) = owner_type {
                    // TDD FIX: For generic types like "ComponentArray<T>", also try base name "ComponentArray"
                    if let Some(field_types) =
                        self.struct_field_types.get(owner.as_str()).or_else(|| {
                            owner
                                .split('<')
                                .next()
                                .and_then(|base| self.struct_field_types.get(base))
                        })
                    {
                        if let Some(field_type) = field_types.get(field) {
                            if let Some(name) = Self::type_to_name(field_type) {
                                return Some(name);
                            }
                        }
                    }
                }
                // Fallback: use the owner type (for self.field_name → current struct type)
                owner_type
            }
            Expression::Unary {
                op:
                    crate::parser::UnaryOp::Deref
                    | crate::parser::UnaryOp::Ref
                    | crate::parser::UnaryOp::MutRef,
                operand,
                ..
            } => {
                // Look through references/derefs
                self.infer_type_name(operand)
            }
            Expression::MethodCall { object, method, .. } => {
                // For method chains, try to resolve the return type of the method.
                // If the method returns Self (or the same type), the type propagates.
                let obj_type = self.infer_type_name(object);
                if let Some(ref tn) = obj_type {
                    let qualified = format!("{}::{}", tn, method);
                    if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                        if let Some(ref ret_type) = sig.return_type {
                            if let Some(ret_name) = Self::type_to_name(ret_type) {
                                return Some(ret_name);
                            }
                        }
                    }
                }
                obj_type
            }
            Expression::Index { object, .. } => {
                // For collection[i], resolve the element type rather than the collection type.
                // e.g. self.enemies[i] where enemies: Vec<Enemy> → "Enemy"
                if let Expression::FieldAccess {
                    object: field_obj,
                    field,
                    ..
                } = &**object
                {
                    let owner_type = self.infer_type_name(field_obj);
                    if let Some(ref owner) = owner_type {
                        if let Some(field_types) =
                            self.struct_field_types.get(owner.as_str()).or_else(|| {
                                owner
                                    .split('<')
                                    .next()
                                    .and_then(|base| self.struct_field_types.get(base))
                            })
                        {
                            if let Some(field_type) = field_types.get(field.as_str()) {
                                if let Some(elem_type) =
                                    Self::extract_iterator_element_type(field_type)
                                {
                                    if let Some(name) = Self::type_to_name(&elem_type) {
                                        return Some(name);
                                    }
                                }
                            }
                        }
                    }
                }
                if let Expression::Identifier { name, .. } = &**object {
                    let var_type = self
                        .local_var_types
                        .get(name.as_str())
                        .cloned()
                        .or_else(|| {
                            self.current_function_params
                                .iter()
                                .find(|p| p.name == *name)
                                .map(|p| p.type_.clone())
                        });
                    if let Some(vt) = var_type {
                        if let Some(elem_type) = Self::extract_iterator_element_type(&vt) {
                            if let Some(name) = Self::type_to_name(&elem_type) {
                                return Some(name);
                            }
                        }
                    }
                }
                self.infer_type_name(object)
            }
            Expression::Call { function, .. } => {
                // For constructor calls like Config::new(...), infer return type.
                // Config::new() → FieldAccess(Identifier("Config"), "new") → type is "Config"
                match &**function {
                    Expression::FieldAccess { object, field, .. } => {
                        // Type::method() — infer type from the object (Type name)
                        if let Some(type_name) = self.infer_type_name(object) {
                            let qualified = format!("{}::{}", type_name, field);
                            if let Some(sig) = self.signature_registry.get_signature(&qualified) {
                                if let Some(ref ret_type) = sig.return_type {
                                    if let Some(ret_name) = Self::type_to_name(ret_type) {
                                        return Some(ret_name);
                                    }
                                }
                            }
                            // Constructors conventionally return Self
                            return Some(type_name);
                        }
                        None
                    }
                    Expression::Identifier { name, .. } => {
                        if let Some(sig) = self.signature_registry.get_signature(name) {
                            if let Some(ref ret_type) = sig.return_type {
                                return Self::type_to_name(ret_type);
                            }
                        }
                        None
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Extract a type name from a Type enum (for signature lookup)
    pub(in crate::codegen::rust) fn type_to_name(type_: &Type) -> Option<String> {
        match type_ {
            // WJ `string` / `String` must resolve to the stdlib registry key "String" so
            // `infer_type_name` can find e.g. `String::contains` (Pattern/&str) signatures.
            Type::String => Some("String".to_string()),
            Type::Custom(name) if name == "string" => Some("String".to_string()),
            Type::Custom(name) => Some(name.clone()),
            Type::Parameterized(name, _) => Some(name.clone()),
            Type::Reference(inner) | Type::MutableReference(inner) => Self::type_to_name(inner),
            // TDD FIX: Handle stdlib container types for method signature lookup
            // Without this, self.dense (Vec<T>) can't resolve to "Vec" for Vec::remove lookup
            Type::Vec(_) => Some("Vec".to_string()),
            Type::Option(_) => Some("Option".to_string()),
            Type::Result(_, _) => Some("Result".to_string()),
            Type::Array(_, _) => Some("Array".to_string()),
            _ => None,
        }
    }

    /// Extract the element type from an iterable type.
    /// Vec<T> → T, &Vec<T> → T, &mut Vec<T> → T, Array(T, _) → T
    pub(in crate::codegen::rust) fn extract_iterator_element_type(
        iterable_type: &Type,
    ) -> Option<Type> {
        match iterable_type {
            Type::Vec(inner) => Some(inner.as_ref().clone()),
            Type::Array(inner, _) => Some(inner.as_ref().clone()),
            Type::Reference(inner) | Type::MutableReference(inner) => {
                Self::extract_iterator_element_type(inner)
            }
            _ => None,
        }
    }
}

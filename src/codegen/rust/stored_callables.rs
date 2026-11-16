//! Callables kept in struct fields.
//!
//! A field declared `fn(A) -> R` (or `Vec<fn(A) -> R>`, `Option<..>`) holds any callable: a function item or a
//! capturing closure, as a `fn(A) -> R` parameter accepts. A Rust `fn` pointer cannot hold a capture, so such a field
//! is emitted as `Box<dyn Fn(A) -> R>`, decided from the field's declared type. The pieces that follow from it:
//! a value stored into the field is boxed (`Box::new(f)`), a `fn(A) -> R` parameter that the function stores in a field
//! is `impl Fn(A) -> R + 'static` (a box owns its closure), a field read as a value is borrowed (`&h.run`: the box is
//! not Copy) and `(h.run)(x)` calls the field when the struct declares no method of that name.

use crate::parser::{Expression, Type};

use super::CodeGenerator;

/// The declared type is a callable (`fn(A) -> R`, `dyn Fn(A) -> R`).
pub(in crate::codegen::rust) fn is_callable_type(t: &Type) -> bool {
    matches!(t, Type::FunctionPointer { .. } | Type::ClosureTrait { .. })
}

/// A field type whose emitted value is a box around a callable: `fn(A) -> R` (boxed by `stored_field_type`) or a
/// written `Box<dyn Fn(A) -> R>`.
pub(in crate::codegen::rust) fn holds_boxed_callable(t: &Type) -> bool {
    matches!(t, Type::FunctionPointer { .. }) || matches!(t, Type::Parameterized(_, args) if matches!(args.as_slice(), [Type::ClosureTrait { .. }]))
}

/// The emitted type of a struct field: a `fn(A) -> R` (alone or as the element of a `Vec` / `Option` / map value) is
/// a boxed closure trait object.
pub(in crate::codegen::rust) fn stored_field_type(t: &Type) -> Type {
    match t {
        Type::FunctionPointer { params, return_type } => Type::Parameterized(
            "Box".to_string(),
            vec![Type::ClosureTrait { kind: "Fn".to_string(), params: params.clone(), return_type: return_type.clone(), dynamic: true }],
        ),
        Type::Vec(inner) => Type::Vec(Box::new(stored_field_type(inner))),
        Type::Option(inner) => Type::Option(Box::new(stored_field_type(inner))),
        Type::Parameterized(name, args) => Type::Parameterized(name.clone(), args.iter().map(stored_field_type).collect()),
        other => other.clone(),
    }
}

/// The element a stored collection of callables holds (`Vec<fn(A) -> R>`: `fn(A) -> R`).
fn stored_element_is_callable(t: &Type) -> bool {
    match t {
        Type::Vec(inner) | Type::Option(inner) => matches!(inner.as_ref(), Type::FunctionPointer { .. }),
        Type::Parameterized(_, args) => args.last().is_some_and(|a| matches!(a, Type::FunctionPointer { .. })),
        _ => false,
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// The declared type of `object.field` when `object` is a struct this program declares.
    pub(in crate::codegen::rust) fn declared_field_type(&self, object: &Expression<'ast>, field: &str) -> Option<Type> {
        let name = self.struct_name_of(object)?;
        self.lookup_struct_field_types(&name)?.get(field).cloned()
    }

    /// The struct a value is (`self` in a method is the impl's struct).
    fn struct_name_of(&self, object: &Expression<'ast>) -> Option<String> {
        if matches!(object, Expression::Identifier { name, .. } if name == "self") && self.in_impl_block {
            if let Some(n) = &self.current_struct_name {
                return Some(n.split('<').next().unwrap_or(n).to_string());
            }
        }
        let ty = self.infer_expression_type(object)?;
        Self::type_to_name(crate::codegen::rust::collection_detection::peel_type_reference(&ty))
    }

    /// `Box::new(value)` for a value stored into a field declared `fn(A) -> R` (a value already typed `Box<..>` stays).
    pub(in crate::codegen::rust) fn box_stored_callable(&self, field_type: &Type, value: &Expression<'ast>, value_str: String) -> String {
        let already_boxed = self.infer_expression_type(value).is_some_and(|t| crate::codegen::rust::recursive_struct_layout::type_is_box(&t));
        if matches!(field_type, Type::FunctionPointer { .. }) && !already_boxed {
            format!("Box::new({value_str})")
        } else {
            value_str
        }
    }

    /// A struct literal's field value: a callable stored into a `fn(A) -> R` field (or a list of them) is boxed.
    pub(in crate::codegen::rust) fn store_into_struct_field(&mut self, field_name: &str, value: &Expression<'ast>, value_str: String) -> String {
        let Some(field_type) = self.current_struct_literal_name.clone().and_then(|sn| self.lookup_struct_field_types(&sn)).and_then(|m| m.get(field_name).cloned()) else {
            return value_str;
        };
        match self.stored_callable_list(&field_type, value) {
            Some(list) => list,
            None => self.box_stored_callable(&field_type, value, value_str),
        }
    }

    /// The list literal stored into a `Vec<fn(A) -> R>` field: each element boxed.
    pub(in crate::codegen::rust) fn stored_callable_list(&mut self, field_type: &Type, value: &Expression<'ast>) -> Option<String> {
        let Type::Vec(inner) = field_type else { return None };
        if !matches!(inner.as_ref(), Type::FunctionPointer { .. }) {
            return None;
        }
        let Expression::Array { elements, .. } = value else { return None };
        let items: Vec<String> = elements.iter().map(|e| format!("Box::new({})", self.generate_expression(e))).collect();
        Some(format!("vec![{}]", items.join(", ")))
    }

    /// Every argument of `receiver.method(..)`, boxed where it is stored into a field that holds callables.
    pub(in crate::codegen::rust) fn box_callables_stored_through_field(&self, receiver: &Expression<'ast>, arguments: &[(Option<String>, &'ast Expression<'ast>)], args: Vec<String>) -> Vec<String> {
        args.into_iter().zip(arguments).map(|(a, (_, e))| self.box_callable_stored_through_field(receiver, e, a)).collect()
    }

    /// `self.handlers.push(f)`: an argument stored into a field that holds callables is boxed.
    pub(in crate::codegen::rust) fn box_callable_stored_through_field(&self, receiver: &Expression<'ast>, arg: &Expression<'ast>, arg_str: String) -> String {
        let Expression::FieldAccess { object, field, .. } = receiver else { return arg_str };
        let stores_callables = self.declared_field_type(object, field).is_some_and(|t| stored_element_is_callable(&t));
        let is_value = matches!(arg, Expression::Identifier { .. } | Expression::Closure { .. });
        let already_boxed = self.infer_expression_type(arg).is_some_and(|t| crate::codegen::rust::recursive_struct_layout::type_is_box(&t));
        if stores_callables && is_value && !already_boxed {
            format!("Box::new({arg_str})")
        } else {
            arg_str
        }
    }

    /// The struct value a field path starts from is held by reference here (`self` in a method, a parameter emitted
    /// `&T`): a box in its field can be borrowed, not moved out.
    pub(in crate::codegen::rust) fn place_is_borrowed(&self, object: &Expression<'ast>) -> bool {
        match object {
            Expression::Identifier { name, .. } => {
                self.current_function_params.iter().any(|p| p.name == *name && p.name == "self")
                    || self.inferred_borrowed_params.contains(name.as_str())
                    || self.emitted_rust_ref_formals.contains(name.as_str())
                    || matches!(self.infer_expression_type(object), Some(Type::Reference(_) | Type::MutableReference(_)))
            }
            Expression::FieldAccess { object, .. } => self.place_is_borrowed(object),
            _ => false,
        }
    }

    /// `(h.run)(x)`: `h`'s struct declares a callable field `run` and no method `run`.
    pub(in crate::codegen::rust) fn calls_callable_field(&self, object: &Expression<'ast>, field: &str) -> bool {
        if !self.declared_field_type(object, field).is_some_and(|t| is_callable_type(&t) || holds_boxed_callable(&t)) {
            return false;
        }
        let Some(name) = self.struct_name_of(object) else { return false };
        self.lookup_method_signature(&name, field).is_none()
    }
}

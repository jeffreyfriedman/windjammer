//! Resolve the Windjammer type name of a method / delegation receiver.
//!
//! Field projections (`deps.writer`) use the field's type, not the outer param.
//! Used by constraint generation and delegation ownership so both stay DRY.

use crate::parser::ast::core::{Expression, FunctionDecl};
use crate::parser::Type;
use std::collections::HashMap;

/// Struct-base name for registry / field-map lookup (`Vec<T>` → `Vec`, `&Foo` → `Foo`).
pub fn type_to_struct_base(ty: &Type) -> Option<String> {
    crate::type_classification::type_to_registry_base(ty)
}

fn strip_generics(name: &str) -> String {
    name.split('<').next().unwrap_or(name).to_string()
}

/// Type name of the expression that is the method receiver.
///
/// - `c.increment()` → param `c`'s type
/// - `deps.writer.append(...)` → field `writer`'s type (`Writer`), not `AppDeps`
/// - `self.parts[n].advance()` → the element type of field `parts` (`Part`), not `Vec`
pub fn infer_receiver_type_name<'ast>(
    object: &Expression<'ast>,
    func: &FunctionDecl<'ast>,
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
) -> Option<String> {
    match object {
        Expression::Identifier { name, .. } if name == "self" => {
            func.parent_type.as_ref().map(|p| strip_generics(p))
        }
        _ => infer_receiver_type(object, func, struct_field_types).as_ref().and_then(type_to_struct_base),
    }
}

/// Declared type of a receiver path (`p`, `p.f`, `p.f[i]`, `self.f.g`); `None` when any step is unknown.
fn infer_receiver_type<'ast>(
    object: &Expression<'ast>,
    func: &FunctionDecl<'ast>,
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
) -> Option<Type> {
    match object {
        Expression::Identifier { name, .. } if name == "self" => {
            func.parent_type.as_ref().map(|p| Type::Custom(strip_generics(p)))
        }
        Expression::Identifier { name, .. } => func.parameters.iter().find(|p| &p.name == name).map(|p| p.type_.clone()),
        Expression::FieldAccess { object: inner, field, .. } => {
            let inner_base = infer_receiver_type(inner, func, struct_field_types).as_ref().and_then(type_to_struct_base)?;
            crate::type_inference::struct_field_registry::lookup_struct_field_map(
                struct_field_types,
                &inner_base,
                &HashMap::new(),
                &HashMap::new(),
            )
            .and_then(|fields| fields.get(field.as_str()))
            .cloned()
        }
        Expression::Index { object: inner, .. } => element_type(&infer_receiver_type(inner, func, struct_field_types)?),
        _ => None,
    }
}

/// Element type of an indexable sequence (`Vec<T>`, `[T; N]`, through references).
fn element_type(ty: &Type) -> Option<Type> {
    match ty {
        Type::Vec(e) | Type::Array(e, _) => Some((**e).clone()),
        Type::Parameterized(name, args) if strip_generics(name) == "Vec" && args.len() == 1 => Some(args[0].clone()),
        Type::Reference(inner) | Type::MutableReference(inner) => element_type(inner),
        _ => None,
    }
}

//! Declared type of a field/index chain rooted at a parameter (`draft.lines[i].amount_cents`).
//!
//! The solver used to give every field read a fresh unconstrained type, so an `int` accumulator over a
//! field defined in another module fell back to the legacy `i32` default (plan C4: widths come from the
//! field's declared type, resolved through the cross-module struct field table).

use crate::parser::ast::core::{Expression, FunctionDecl};
use crate::parser::Type;
use std::collections::HashMap;

fn strip_refs(t: &Type) -> &Type {
    match t {
        Type::Reference(i) | Type::MutableReference(i) => strip_refs(i),
        other => other,
    }
}

/// Declared type of `expr` when it is a parameter, a field of a known struct, or an element of a
/// `Vec`/array typed value. `None` when any link of the chain is unknown.
pub fn declared_type_of<'ast>(
    expr: &Expression<'ast>,
    func: &FunctionDecl<'ast>,
    struct_field_types: &HashMap<String, HashMap<String, Type>>,
) -> Option<Type> {
    match expr {
        Expression::Identifier { name, .. } => func
            .parameters
            .iter()
            .find(|p| &p.name == name)
            .map(|p| strip_refs(&p.type_).clone()),
        Expression::FieldAccess { object, field, .. } => {
            let obj = declared_type_of(object, func, struct_field_types)?;
            let base = crate::ir::receiver_type::type_to_struct_base(&obj)?;
            crate::type_inference::struct_field_registry::lookup_struct_field_map(
                struct_field_types,
                &base,
                &HashMap::new(),
                &HashMap::new(),
            )
            .and_then(|fields| fields.get(field.as_str()))
            .map(|t| strip_refs(t).clone())
        }
        Expression::Index { object, .. } => match declared_type_of(object, func, struct_field_types)? {
            Type::Vec(inner) | Type::Array(inner, _) => Some(strip_refs(&inner).clone()),
            Type::Parameterized(n, args) if n == "Vec" && args.len() == 1 => Some(strip_refs(&args[0]).clone()),
            _ => None,
        },
        _ => None,
    }
}

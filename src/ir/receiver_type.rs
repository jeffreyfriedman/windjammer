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
        Expression::Identifier { name, .. } => func
            .parameters
            .iter()
            .find(|p| &p.name == name)
            .map(|p| p.type_.clone())
            .or_else(|| local_binding_type(name, func)),
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

/// Declared type of a local bound once in `func` by `let`: the annotation, else the type of its initializer
/// when that is itself known by declaration (`let mut host = self`, `let p = param`, `let c = Csr { .. }`).
/// `None` when the name is bound more than once (shadowing) or the initializer's type is not declared.
fn local_binding_type<'ast>(
    name: &str,
    func: &FunctionDecl<'ast>,
) -> Option<Type> {
    let mut lets = Vec::new();
    collect_lets(&func.body, name, &mut lets);
    let [(annotation, value)] = lets.as_slice() else {
        return None;
    };
    if let Some(ty) = annotation {
        return Some(ty.clone());
    }
    match value {
        // `self` or a parameter only: following another local could cycle (`let a = b` ... `let b = a`).
        Expression::Identifier { name: source, .. } if source == "self" => func.parent_type.as_ref().map(|p| Type::Custom(strip_generics(p))),
        Expression::Identifier { name: source, .. } => func.parameters.iter().find(|p| &p.name == source).map(|p| p.type_.clone()),
        Expression::StructLiteral { name: struct_name, .. } => Some(Type::Custom(strip_generics(struct_name))),
        _ => None,
    }
}

/// Every `let <name> = value` (plain identifier pattern) in `stmts`, nested blocks included.
fn collect_lets<'a, 'ast>(
    stmts: &'a [&'ast crate::parser::ast::core::Statement<'ast>],
    name: &str,
    out: &mut Vec<(&'a Option<Type>, &'ast Expression<'ast>)>,
) {
    use crate::parser::ast::core::{Pattern, Statement};
    for stmt in stmts {
        match stmt {
            Statement::Let { pattern: Pattern::Identifier(n), type_, value, .. } if n == name => out.push((type_, value)),
            Statement::If { then_block, else_block, .. } => {
                collect_lets(then_block, name, out);
                if let Some(b) = else_block {
                    collect_lets(b, name, out);
                }
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } | Statement::For { body, .. } => {
                collect_lets(body, name, out)
            }
            _ => {}
        }
    }
}

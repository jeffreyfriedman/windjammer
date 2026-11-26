//! Static types of local expressions, as far as declarations decide them: annotations, struct literals, declared
//! results of functions and methods (user and `std/rust_core.wji`), declared field and payload types. `None` when
//! any step is unknown; the check then reports nothing about that value.

use super::facts::Facts;
use crate::parser::ast::core::Expression;
use crate::parser::{Literal, Type, UnaryOp};
use crate::type_classification::type_name_leaf;

/// What the types need from the walker: the type of a binding in scope, and whether a name is a binding.
pub trait Env {
    fn binding_type(&self, name: &str) -> Option<Option<Type>>;
}

pub fn strip_refs(t: &Type) -> &Type {
    match t {
        Type::Reference(x) | Type::MutableReference(x) => strip_refs(x),
        other => other,
    }
}

pub fn is_reference(t: &Type) -> bool {
    matches!(t, Type::Reference(_) | Type::MutableReference(_))
}

/// The user type a (dereferenced) type names, by its last path segment, when the program declares it.
pub fn user_type<'t>(facts: &Facts, t: &'t Type) -> Option<&'t str> {
    match strip_refs(t) {
        Type::Custom(n) | Type::Parameterized(n, _) => {
            let leaf = type_name_leaf(n);
            facts.types.contains(leaf).then_some(leaf)
        }
        _ => None,
    }
}

/// `Type::f` / `module::Type::f` / `module::f` split into (type, function); the type is `None` for a free function.
pub fn path_call<'n>(facts: &Facts, path: &'n str) -> (Option<&'n str>, &'n str) {
    let mut parts = path.rsplitn(3, "::");
    let f = parts.next().unwrap_or(path);
    let owner = parts.next().filter(|t| facts.types.contains(type_name_leaf(t)) || is_core_type(facts, t));
    (owner, f)
}

/// A type `std/rust_core.wji` declares (`Vec`, `HashMap`, `String`, ...).
fn is_core_type(_facts: &Facts, name: &str) -> bool {
    crate::typing::core_surface::surface_type_name(&Type::Parameterized(type_name_leaf(name).to_string(), Vec::new())).is_some()
}

pub fn type_of(facts: &Facts, env: &dyn Env, e: &Expression<'_>) -> Option<Type> {
    match e {
        Expression::Identifier { name, .. } => env.binding_type(name).flatten(),
        Expression::Literal { value, .. } => match value {
            Literal::String(_) => Some(Type::String),
            Literal::Int(_) => Some(Type::Int),
            Literal::Wide(_) => Some(Type::Custom("u128".to_string())),
            Literal::IntSuffixed(_, s) => Some(Type::Custom(s.clone())),
            Literal::Float(_) => Some(Type::Float),
            Literal::Bool(_) => Some(Type::Bool),
            Literal::Char(_) => Some(Type::Custom("char".to_string())),
        },
        Expression::StructLiteral { name, .. } => {
            let leaf = type_name_leaf(name);
            facts.types.contains(leaf).then(|| Type::Custom(leaf.to_string()))
        }
        Expression::Array { elements, .. } => {
            let elem = elements.first().and_then(|x| type_of(facts, env, x)).unwrap_or(Type::Infer);
            Some(Type::Vec(Box::new(elem)))
        }
        Expression::MapLiteral { .. } => Some(Type::Parameterized("HashMap".to_string(), vec![Type::Infer, Type::Infer])),
        Expression::Tuple { elements, .. } => {
            Some(Type::Tuple(elements.iter().map(|x| type_of(facts, env, x).unwrap_or(Type::Infer)).collect()))
        }
        Expression::Cast { type_, .. } => Some(type_.clone()),
        Expression::Unary { op, operand, .. } => {
            let t = type_of(facts, env, operand)?;
            match op {
                UnaryOp::Ref => Some(Type::Reference(Box::new(t))),
                UnaryOp::MutRef => Some(Type::MutableReference(Box::new(t))),
                UnaryOp::Deref => match t {
                    Type::Reference(x) | Type::MutableReference(x) => Some(*x),
                    _ => None,
                },
                UnaryOp::Not | UnaryOp::Neg => Some(t),
            }
        }
        Expression::TryOp { expr, .. } => match strip_refs(&type_of(facts, env, expr)?) {
            Type::Option(t) | Type::Result(t, _) => Some((**t).clone()),
            _ => None,
        },
        Expression::FieldAccess { object, field, .. } => {
            let t = type_of(facts, env, object)?;
            let owner = user_type(facts, &t)?;
            facts.fields.get(owner)?.get(field).cloned()
        }
        Expression::Index { object, index, .. } => {
            // a range index is a slice of the same sequence, not an element
            if matches!(index, Expression::Range { .. }) {
                return None;
            }
            match strip_refs(&type_of(facts, env, object)?) {
                Type::Vec(t) | Type::Array(t, _) => Some((**t).clone()),
                Type::Parameterized(n, args) if type_name_leaf(n) == "Vec" && args.len() == 1 => Some(args[0].clone()),
                Type::Parameterized(n, args) if crate::typing::core_surface::is_map_container(n) && args.len() == 2 => Some(args[1].clone()),
                _ => None,
            }
        }
        Expression::Call { function, arguments, .. } => {
            let Expression::Identifier { name, .. } = &**function else { return None };
            if env.binding_type(name).is_some() {
                return None; // a closure held in a local
            }
            // `Some(x)`: the language's own optional constructor
            if name == "Some" && arguments.len() == 1 {
                let inner = type_of(facts, env, arguments[0].1).unwrap_or(Type::Infer);
                return Some(Type::Option(Box::new(inner)));
            }
            let (owner, f) = path_call(facts, name);
            match owner {
                Some(o) => static_result(facts, o, f),
                None => facts.function(name).and_then(|c| c.ret.clone()),
            }
        }
        Expression::MethodCall { object, method, type_args, .. } => {
            // `Counter::new()` parsed as a call on the type name
            if let Expression::Identifier { name, .. } = &**object {
                if env.binding_type(name).is_none() && (facts.types.contains(type_name_leaf(name)) || is_core_type(facts, name)) {
                    return static_result(facts, name, method);
                }
            }
            let recv = type_of(facts, env, object)?;
            // `x.clone()` is a copy of the value `x` holds
            if crate::type_classification::is_language_level_explicit_clone(method) {
                return Some(strip_refs(&recv).clone());
            }
            if crate::type_classification::is_language_level_owned_string_convert(method) {
                return Some(Type::String);
            }
            if let Some(owner) = user_type(facts, &recv) {
                return facts.method(owner, method).and_then(|c| c.ret.clone());
            }
            let sig = facts.core.as_ref()?.method_signature(&recv, method, type_args.as_deref().unwrap_or(&[]))?;
            sig.ret.filter(|t| !matches!(t, Type::Generic(_)))
        }
        _ => None,
    }
}

/// The result of an associated function `Type::f(..)`.
fn static_result(facts: &Facts, owner: &str, f: &str) -> Option<Type> {
    if let Some(c) = facts.method(owner, f) {
        return c.ret.clone();
    }
    let core = facts.core.as_ref()?;
    let leaf = type_name_leaf(owner);
    let (generics, _, _, ret) = core.declared_static(leaf, f)?;
    ret.or_else(|| Some(crate::typing::core_surface::self_type(leaf, &generics)))
}

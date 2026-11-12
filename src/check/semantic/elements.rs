//! Element kinds of loop and `let` bindings, taken from declarations only: the `String` methods of
//! `std/rust_core.wji` (`chars(self) -> Iterator<char>`, read through `typing::CoreSurface`), the std module
//! functions of `std/<module>.wj` (`strings::chars(s) -> Vec<char>`) and annotated types. A name or shape that no
//! declaration covers has no element kind, and nothing is judged about it.

use super::rules::{self, Kind};
use crate::parser::{Expression, Type};
use super::declared::Declared;
use std::collections::{HashMap, HashSet};

/// What a function body has bound so far, as far as kinds go.
pub struct Scope<'a> {
    /// `let x = ..` / parameter scalar kinds (kind, declared).
    pub scalars: &'a HashMap<String, (Kind, bool)>,
    /// Bindings known to hold a sequence of one scalar kind (`let cs = strings.chars(s)`).
    pub elems: &'a HashMap<String, Kind>,
    /// `use std::strings` aliases, mapped to the module.
    pub aliases: &'a HashMap<String, String>,
    /// Every name bound in the function (a local hides a module alias).
    pub names: &'a HashSet<String>,
    pub declared: &'a Declared,
}

/// The scalar kind of the elements of a declared sequence type (`Vec<char>`, `Iterator<char>`).
pub fn element_of_type(t: &Type) -> Option<Kind> {
    match t {
        Type::Vec(e) | Type::Array(e, _) => rules::type_kind(e),
        Type::Parameterized(n, args) if crate::type_classification::type_name_leaf(n) == "Iterator" => rules::type_kind(args.first()?),
        Type::Reference(t) => element_of_type(t),
        _ => None,
    }
}

fn is_string(scope: &Scope<'_>, e: &Expression<'_>) -> bool {
    match e {
        Expression::Literal { value, .. } => rules::literal_kind(value) == Some(Kind::Str),
        Expression::Identifier { name, .. } => matches!(scope.scalars.get(name), Some((Kind::Str, true))),
        _ => false,
    }
}

/// The element kind of what `e` iterates over, if a declaration says.
pub fn iterable_element(scope: &Scope<'_>, e: &Expression<'_>) -> Option<Kind> {
    match e {
        Expression::Identifier { name, .. } => scope.elems.get(name).copied(),
        Expression::Call { function: Expression::Identifier { name, .. }, .. } => {
            let (head, func) = name.rsplit_once("::")?;
            let module = scope.aliases.get(head).filter(|_| !scope.names.contains(head))?;
            element_of_type(scope.declared.std_return(module, func)?)
        }
        Expression::MethodCall { object, method, .. } => {
            if let Expression::Identifier { name, .. } = &**object {
                if let Some(module) = scope.aliases.get(name).filter(|_| !scope.names.contains(name)) {
                    return element_of_type(scope.declared.std_return(module, method)?);
                }
            }
            if !is_string(scope, object) {
                return None;
            }
            element_of_type(&scope.declared.string_method_return(method)?)
        }
        _ => None,
    }
}

//! A method called on a std iterator that the std declaration (`std/rust_core.wji`) gives only to lists.
//!
//! `let parts = "a,b".split(",")` is an iterator (`String::split` is declared to return an `Iterator`); `parts.contains("a")`
//! has no such method in Rust (`Split`) and the interpreter refuses it. The declaration decides: the method is declared on
//! `Vec` and not on `Iterator`. Nothing here matches a method name except the one documented exemption below.

use crate::ir::stdlib_tables::{surface_type_name, CoreSurface, ITERATOR};
use crate::parser::{Expression, Literal};
use std::collections::HashMap;

/// The std surface, parsed once per process (kept with the stdlib registry, shared with the declarations).
fn surface() -> &'static CoreSurface {
    super::declared::core()
}

/// Iterators that know their length (`ExactSizeIterator`): the surface does not model sizes, so `len` is not judged on
/// a chain that starts at one of these producers.
const EXACT_SIZE_PRODUCERS: [&str; 6] = ["bytes", "keys", "values", "iter", "iter_mut", "into_iter"];

/// Bindings known to hold an iterator, with the producer method the chain started at (`split`, `bytes`, ..).
pub type IterLocals = HashMap<String, String>;

/// The producer method at the root of `e` when `e` evaluates to an iterator by declaration: `recv` gives the surface
/// type an expression is declared to have (`String`, `HashMap`, ..); `locals` are the bindings known to hold an iterator.
pub fn iterator_root(e: &Expression<'_>, recv: &dyn Fn(&Expression<'_>) -> Option<&'static str>, locals: &IterLocals) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => locals.get(name).cloned(),
        Expression::MethodCall { object, method, .. } => {
            let declared_ret = |ty: &str| {
                surface()
                    .declared(ty, method)
                    .and_then(|(_, _, _, ret)| ret)
                    .is_some_and(|r| surface_type_name(&r).is_some_and(|h| h == ITERATOR))
            };
            if let Some(ty) = recv(object) {
                // the receiver's own declaration decides (`HashMap::keys`, `String::split`), not a sequence fall-back
                (surface().has(ty, method) && declared_ret(ty)).then(|| method.clone())
            } else if declared_ret(ITERATOR) {
                iterator_root(object, recv, locals)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// A string literal.
pub fn is_string_literal(e: &Expression<'_>) -> bool {
    matches!(e, Expression::Literal { value: Literal::String(_), .. })
}

/// The list type declares `method` and `Iterator` does not (collecting first is the fix); `len` is not judged on a
/// chain that starts at an exact-size producer.
pub fn lists_only(method: &str, root: &str) -> bool {
    !(method == "len" && EXACT_SIZE_PRODUCERS.contains(&root))
        && surface().declared("Vec", method).is_some()
        && surface().declared(ITERATOR, method).is_none()
}

/// The surface type of a string literal or of a binding declared with a surface type (`HashMap<string, int>` -> `HashMap`).
pub fn receiver_type(e: &Expression<'_>, declared: &HashMap<String, &'static str>) -> Option<&'static str> {
    match e {
        _ if is_string_literal(e) => Some("String"),
        Expression::Identifier { name, .. } => declared.get(name).copied(),
        _ => None,
    }
}

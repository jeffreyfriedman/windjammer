//! A method called on a std iterator that the std declaration (`std/rust_core.wji`) gives only to lists.
//!
//! `let parts = "a,b".split(",")` is an iterator (`String::split` is declared to return an `Iterator`); `parts.contains("a")`
//! has no such method in Rust (`Split`) and the interpreter refuses it. The declaration decides: the method is declared on
//! `Vec` and not on `Iterator`. Nothing here matches a method name except the one documented exemption below.

use crate::ir::stdlib_tables::{surface_type_name, CoreSurface, ITERATOR};
use crate::parser::{Expression, Literal};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The std surface, parsed once per process.
fn surface() -> &'static CoreSurface {
    static S: OnceLock<CoreSurface> = OnceLock::new();
    S.get_or_init(CoreSurface::parse)
}

/// Iterators that know their length (`ExactSizeIterator`): the surface does not model sizes, so `len` is not judged on
/// a chain that starts at one of these producers.
const EXACT_SIZE_PRODUCERS: [&str; 1] = ["bytes"];

/// Bindings known to hold an iterator, with the producer method the chain started at (`split`, `bytes`, ..).
pub type IterLocals = HashMap<String, String>;

/// The producer method at the root of `e` when `e` evaluates to an iterator by declaration: `text` says whether an
/// expression is a string; `locals` are the bindings known to hold an iterator.
pub fn iterator_root(e: &Expression<'_>, text: &dyn Fn(&Expression<'_>) -> bool, locals: &IterLocals) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => locals.get(name).cloned(),
        Expression::MethodCall { object, method, .. } => {
            let declared_ret = |ty: &str| {
                surface()
                    .declared(ty, method)
                    .and_then(|(_, _, _, ret)| ret)
                    .is_some_and(|r| surface_type_name(&r).is_some_and(|h| h == ITERATOR))
            };
            if text(object) {
                declared_ret("String").then(|| method.clone())
            } else if declared_ret(ITERATOR) {
                iterator_root(object, text, locals)
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

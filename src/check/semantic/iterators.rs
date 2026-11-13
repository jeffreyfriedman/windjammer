//! A method called on a std iterator that the std declaration (`std/rust_core.wji`) gives only to lists.
//!
//! `let parts = "a,b".split(",")` is an iterator (`String::split` is declared to return an `Iterator`); `parts.contains("a")`
//! has no such method in Rust (`Split`) and the interpreter refuses it. The declaration decides: the method is declared on
//! `Vec` and not on `Iterator`. Nothing here matches a method name except the one documented exemption below.

use crate::ir::stdlib_tables::{surface_type_name, CoreSurface, ITERATOR};
use crate::parser::{Expression, Literal};
use std::collections::HashSet;
use std::sync::OnceLock;

/// The std surface, parsed once per process.
fn surface() -> &'static CoreSurface {
    static S: OnceLock<CoreSurface> = OnceLock::new();
    S.get_or_init(CoreSurface::parse)
}

/// `ExactSizeIterator::len` exists on sized iterators (`bytes()`, `vec.iter()`); the surface does not model sizes.
const EXACT_SIZE: [&str; 1] = ["len"];

/// Does `e` evaluate to an iterator, by declaration? `text` says whether an expression is a string, `locals` are the
/// bindings known to hold an iterator.
pub fn yields_iterator(e: &Expression<'_>, text: &dyn Fn(&Expression<'_>) -> bool, locals: &HashSet<String>) -> bool {
    match e {
        Expression::Identifier { name, .. } => locals.contains(name),
        Expression::MethodCall { object, method, .. } => {
            let declared_ret = |ty: &str| {
                surface()
                    .declared(ty, method)
                    .and_then(|(_, _, _, ret)| ret)
                    .is_some_and(|r| surface_type_name(&r).is_some_and(|h| h == ITERATOR))
            };
            if text(object) {
                declared_ret("String")
            } else {
                yields_iterator(object, text, locals) && declared_ret(ITERATOR)
            }
        }
        _ => false,
    }
}

/// A string literal or a binding the caller knows to be text.
pub fn is_string_literal(e: &Expression<'_>) -> bool {
    matches!(e, Expression::Literal { value: Literal::String(_), .. })
}

/// `Some(true)` when the list type declares `method` and `Iterator` does not (collecting first is the fix).
pub fn lists_only(method: &str) -> bool {
    !EXACT_SIZE.contains(&method)
        && surface().declared("Vec", method).is_some()
        && surface().declared(ITERATOR, method).is_none()
}

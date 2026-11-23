//! Typing rules for built-in std methods and constructors.
//!
//! `arg_want` gives the expected type of argument `i` (so closures get typed parameters);
//! `ret_ty` gives the result type. A `None` from `ret_ty` means "not a supported built-in". Both read the declaration in
//! `std/rust_core.wji` (`StdSurface`): no std method name is spelled here.

use super::env::StdSurface;
use super::ir::PExpr;
use super::ty::Ty;

/// Element type when the receiver can be iterated directly or via `.iter()`.
fn iter_elem(recv: &Ty) -> Option<Ty> {
    super::env::iterated(recv)
}

pub fn arg_want(surface: &StdSurface, recv: &Ty, name: &str, i: usize, prev: &[PExpr]) -> Option<Ty> {
    if let Some(t) = super::ordering::arg_want(recv, iter_elem(recv).as_ref(), name, i) {
        return Some(t);
    }
    if let Some(t) = super::enumerable::arg_want(recv, name, i, prev) {
        return Some(t);
    }
    surface.declared_arg_after(recv, name, i, prev)
}

pub fn ret_ty(surface: &StdSurface, recv: &Ty, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
    if let Some(t) = super::ordering::ret_ty(recv, iter_elem(recv).as_ref(), name) {
        return Some(t);
    }
    if let Some(t) = super::enumerable::ret_ty(recv, name, args) {
        return Some(t);
    }
    surface.declared_ret(recv, name, args, want)
}

/// Static constructors, constants and associated functions on std types (`Vec::new`, `i64::MAX`, `char::from_u32`): the
/// declaration's result (`std/rust_core.wji`); the ordering constants are the enumerable stage's.
pub fn static_ret(surface: &StdSurface, owner: &str, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
    if let Some(t) = surface.declared_static_ret(owner, name, args, want) {
        return Some(t);
    }
    (super::ordering::is_path(owner) && super::ordering::variant_value(name).is_some()).then(super::ordering::ty)
}

/// Whether argument `i` of a call is stored by it (a place passed there is copied): the declared parameter is owned.
pub fn stores_arg(surface: &StdSurface, recv: &Ty, name: &str, i: usize) -> bool {
    surface.stores_arg(recv, name, i)
}

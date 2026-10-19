//! `std::cmp::Ordering` and the comparison family (`cmp`, `partial_cmp`, `sort_by`, `max_by_key`, ...).
//!
//! `Ordering` is the built-in type `Ty::Adt("Ordering")` (unless the program declares its own
//! `Ordering`); its values are `Less`/`Equal`/`Greater`. Emitters represent it as -1/0/1.

use super::ty::Ty;

pub const NAME: &str = "Ordering";

pub fn ty() -> Ty {
    Ty::Adt(NAME.into(), vec![])
}

pub fn is_ordering(t: &Ty) -> bool {
    matches!(t, Ty::Adt(n, a) if n == NAME && a.is_empty())
}

/// -1 / 0 / 1 for `Ordering::Less` / `Equal` / `Greater`.
pub fn variant_value(variant: &str) -> Option<i8> {
    match variant {
        "Less" => Some(-1),
        "Equal" => Some(0),
        "Greater" => Some(1),
        _ => None,
    }
}

/// `Ordering` / `cmp::Ordering` / `std::cmp::Ordering`.
pub fn is_path(owner: &str) -> bool {
    matches!(owner, "Ordering" | "cmp::Ordering" | "std::cmp::Ordering")
}

/// Types whose `cmp` is a total order the runtimes implement structurally.
fn totally_ordered(t: &Ty) -> bool {
    match t {
        Ty::Int(_) | Ty::Str | Ty::Char | Ty::Bool | Ty::Unit => true,
        Ty::Tuple(ts) => ts.iter().all(totally_ordered),
        Ty::Vec(e) | Ty::Option(e) => totally_ordered(e),
        _ => false,
    }
}

fn partially_ordered(t: &Ty) -> bool {
    match t {
        Ty::F64 | Ty::F32 => true,
        Ty::Tuple(ts) => ts.iter().all(partially_ordered),
        Ty::Vec(e) | Ty::Option(e) => partially_ordered(e),
        other => totally_ordered(other),
    }
}

/// Expected type of argument `i` of a comparison-family method, when `name` is one.
pub fn arg_want(recv: &Ty, elem: Option<&Ty>, name: &str, i: usize) -> Option<Ty> {
    let f = |p: Vec<Ty>, r: Ty| Ty::Fn(p, Box::new(r));
    match (name, i, elem) {
        ("cmp" | "partial_cmp", 0, _) if partially_ordered(recv) => Some(recv.clone()),
        ("then", 0, _) if is_ordering(recv) => Some(ty()),
        ("then_with", 0, _) if is_ordering(recv) => Some(f(vec![], ty())),
        ("sort_by" | "max_by" | "min_by", 0, Some(e)) => Some(f(vec![e.clone(), e.clone()], ty())),
        ("sort_by_key" | "max_by_key" | "min_by_key", 0, Some(e)) => Some(f(vec![e.clone()], Ty::Unknown)),
        _ => None,
    }
}

/// Result type of a comparison-family method, when `name` is one for this receiver.
pub fn ret_ty(recv: &Ty, elem: Option<&Ty>, name: &str) -> Option<Ty> {
    if is_ordering(recv) {
        return match name {
            "reverse" | "then" | "then_with" => Some(ty()),
            "is_lt" | "is_le" | "is_gt" | "is_ge" | "is_eq" | "is_ne" => Some(Ty::Bool),
            _ => None,
        };
    }
    match (name, elem) {
        ("cmp", _) if totally_ordered(recv) => Some(ty()),
        ("partial_cmp", _) if partially_ordered(recv) => Some(Ty::Option(Box::new(ty()))),
        ("sort_by" | "sort_by_key", Some(_)) if matches!(recv, Ty::Vec(_)) => Some(Ty::Unit),
        ("max_by" | "min_by" | "max_by_key" | "min_by_key", Some(e)) => Some(Ty::Option(Box::new(e.clone()))),
        _ => None,
    }
}

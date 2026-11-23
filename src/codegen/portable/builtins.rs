//! Typing rules for built-in std methods and constructors.
//!
//! `arg_want` gives the expected type of argument `i` (so closures get typed parameters);
//! `ret_ty` gives the result type. A `None` from `ret_ty` means "not a supported built-in".

use super::env::StdSurface;
use super::ir::PExpr;
use super::ty::{IntKind, Ty};

fn b(t: Ty) -> Box<Ty> {
    Box::new(t)
}

fn iter_of(t: Ty) -> Ty {
    Ty::Iter(b(t))
}

fn pair(a: Ty, c: Ty) -> Ty {
    Ty::Tuple(vec![a, c])
}

fn fnty(params: Vec<Ty>, ret: Ty) -> Ty {
    Ty::Fn(params, b(ret))
}

/// Element type when the receiver can be iterated directly or via `.iter()`.
fn iter_elem(recv: &Ty) -> Option<Ty> {
    match recv {
        Ty::Vec(t) | Ty::Iter(t) | Ty::Set(t) | Ty::Range(t) => Some((**t).clone()),
        Ty::Map(k, v) => Some(pair((**k).clone(), (**v).clone())),
        Ty::Option(t) => Some((**t).clone()),
        _ => None,
    }
}

pub fn arg_want(surface: &StdSurface, recv: &Ty, name: &str, i: usize, prev: &[PExpr]) -> Option<Ty> {
    if let Some(t) = super::ordering::arg_want(recv, iter_elem(recv).as_ref(), name, i) {
        return Some(t);
    }
    if let Some(t) = super::enumerable::arg_want(recv, name, i, prev) {
        return Some(t);
    }
    match recv {
        Ty::Vec(t) => vec_arg_want(t, name, i, prev),
        Ty::Iter(_) | Ty::Range(_) => iter_arg_want(&iter_elem(recv)?, name, i, prev),
        Ty::Str | Ty::Option(_) | Ty::Result(..) | Ty::Map(..) | Ty::Set(_) => surface.declared_arg(recv, name, i),
        Ty::Int(_) | Ty::F64 | Ty::F32 | Ty::Char => numeric_arg_want(recv, name, i),
        _ => None,
    }
}

fn vec_arg_want(t: &Ty, name: &str, i: usize, prev: &[PExpr]) -> Option<Ty> {
    match (name, i) {
        ("push", 0) | ("contains", 0) | ("push_front", 0) | ("push_back", 0) | ("fill", 0) => Some(t.clone()),
        ("insert", 0) | ("remove", 0) | ("get", 0) | ("swap", _) | ("truncate", 0) | ("windows", 0) | ("chunks", 0) | ("swap_remove", 0) | ("split_at", 0) => Some(Ty::usize()),
        ("insert", 1) => Some(t.clone()),
        ("extend", 0) | ("append", 0) | ("extend_from_slice", 0) => Some(Ty::Vec(b(t.clone()))),
        ("retain", 0) => Some(fnty(vec![t.clone()], Ty::Bool)),
        ("join", 0) => Some(Ty::Str),
        ("binary_search", 0) => Some(t.clone()),
        // W0003: iterator methods on lists (`xs.filter(f)`) type their closures like `xs.iter().filter(f)`.
        _ => iter_arg_want(t, name, i, prev),
    }
}

fn iter_arg_want(e: &Ty, name: &str, i: usize, prev: &[PExpr]) -> Option<Ty> {
    match (name, i) {
        ("map", 0) | ("for_each", 0) | ("flat_map", 0) | ("filter_map", 0) | ("map_while", 0) | ("inspect", 0) => Some(fnty(vec![e.clone()], Ty::Unknown)),
        ("filter", 0) | ("any", 0) | ("all", 0) | ("take_while", 0) | ("skip_while", 0) | ("find", 0) | ("position", 0) => Some(fnty(vec![e.clone()], Ty::Bool)),
        ("take", 0) | ("skip", 0) | ("step_by", 0) | ("nth", 0) => Some(Ty::usize()),
        ("zip", 0) | ("chain", 0) => Some(Ty::Unknown),
        ("fold", 1) => prev.first().map(|init| fnty(vec![init.ty.clone(), e.clone()], init.ty.clone())),
        _ => None,
    }
}

fn numeric_arg_want(recv: &Ty, name: &str, i: usize) -> Option<Ty> {
    match (name, i) {
        ("pow", 0) => Some(Ty::Int(IntKind::U32)),
        ("powi", 0) => Some(Ty::Int(IntKind::I32)),
        ("rotate_left", 0) | ("rotate_right", 0) => Some(Ty::Int(IntKind::U32)),
        _ if recv.is_numeric() => Some(recv.clone()),
        _ => None,
    }
}

pub fn ret_ty(surface: &StdSurface, recv: &Ty, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
    match name {
        "clone" | "to_owned" | "cloned" | "copied" if !matches!(recv, Ty::Iter(_)) => return Some(recv.clone()),
        "to_string" => return Some(Ty::Str),
        _ => {}
    }
    if let Some(t) = super::ordering::ret_ty(recv, iter_elem(recv).as_ref(), name) {
        return Some(t);
    }
    if let Some(t) = super::enumerable::ret_ty(recv, name, args) {
        return Some(t);
    }
    match recv {
        Ty::Vec(t) => vec_ret(t, name, args, want),
        Ty::Iter(_) | Ty::Range(_) => iter_ret(&iter_elem(recv)?, name, args, want),
        Ty::Int(_) | Ty::F64 | Ty::F32 | Ty::Char | Ty::Str | Ty::Option(_) | Ty::Result(..) | Ty::Map(..) | Ty::Set(_) => surface.declared_ret(recv, name, args, want),
        Ty::Bool | Ty::Tuple(_) | Ty::Adt(..) | Ty::Param(_) => None,
        _ => None,
    }
}

fn vec_ret(t: &Ty, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
    Some(match name {
        "len" | "capacity" => Ty::usize(),
        "is_empty" | "contains" => Ty::Bool,
        "push" | "clear" | "sort" | "reverse" | "dedup" | "truncate" | "swap" | "insert" | "extend" | "append" | "sort_by" | "sort_by_key" | "retain" | "fill" | "push_back" | "push_front" | "extend_from_slice" | "sort_unstable" => Ty::Unit,
        "pop" | "first" | "last" | "get" | "pop_front" | "pop_back" => Ty::Option(b(t.clone())),
        "remove" | "swap_remove" => t.clone(),
        "iter" | "iter_mut" | "into_iter" | "drain" => iter_of(t.clone()),
        "windows" | "chunks" => iter_of(Ty::Vec(b(t.clone()))),
        "to_vec" | "split_off" => Ty::Vec(b(t.clone())),
        "join" => Ty::Str,
        "concat" => t.clone(),
        "binary_search" => Ty::Result(b(Ty::usize()), b(Ty::usize())),
        "split_at" => pair(Ty::Vec(b(t.clone())), Ty::Vec(b(t.clone()))),
        "min" | "max" => Ty::Option(b(t.clone())),
        // W0003: iterator methods are called on lists directly (`xs.map(f)`), so they type like `xs.iter().map(f)`.
        _ => return iter_ret(t, name, args, want),
    })
}

fn collect_ty(e: &Ty, want: Option<&Ty>) -> Ty {
    match want {
        Some(Ty::Vec(_)) | Some(Ty::Set(_)) | Some(Ty::Map(..)) | Some(Ty::Str) => want.unwrap().clone(),
        Some(Ty::Result(ok, err)) => Ty::Result(ok.clone(), err.clone()),
        Some(Ty::Option(inner)) => Ty::Option(inner.clone()),
        _ => Ty::Vec(b(e.clone())),
    }
}

fn iter_ret(e: &Ty, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
    let closure_ret = |i: usize| -> Option<Ty> {
        match args.get(i).map(|a| &a.ty) {
            Some(Ty::Fn(_, r)) => Some((**r).clone()),
            _ => None,
        }
    };
    Some(match name {
        "iter" | "into_iter" | "iter_mut" | "rev" | "take" | "skip" | "step_by" | "filter" | "take_while" | "skip_while" | "cloned" | "copied" | "peekable" | "inspect" => iter_of(e.clone()),
        "map" => iter_of(closure_ret(0)?),
        "filter_map" | "flat_map" => match closure_ret(0)? {
            Ty::Option(t) | Ty::Vec(t) | Ty::Iter(t) => iter_of(*t),
            _ => return None,
        },
        "enumerate" => iter_of(pair(Ty::usize(), e.clone())),
        "zip" => iter_of(pair(e.clone(), args.first()?.ty.elem().or_else(|| iter_elem(&args.first()?.ty))?)),
        "chain" => iter_of(e.clone()),
        "collect" => collect_ty(e, want),
        "sum" | "product" => want.cloned().filter(|w| w.is_numeric()).unwrap_or_else(|| e.clone()),
        "count" => Ty::usize(),
        "fold" => args.first()?.ty.clone(),
        "any" | "all" => Ty::Bool,
        "min" | "max" | "last" | "next" | "find" | "nth" => Ty::Option(b(e.clone())),
        "position" => Ty::Option(b(Ty::usize())),
        "for_each" => Ty::Unit,
        _ => return None,
    })
}

/// Static constructors, constants and associated functions on std types (`Vec::new`, `i64::MAX`, `char::from_u32`): the
/// declaration's result (`std/rust_core.wji`); the ordering constants are the enumerable stage's.
pub fn static_ret(surface: &StdSurface, owner: &str, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
    if let Some(t) = surface.declared_static_ret(owner, name, args, want) {
        return Some(t);
    }
    (super::ordering::is_path(owner) && super::ordering::variant_value(name).is_some()).then(super::ordering::ty)
}

/// Methods whose arguments are stored (so place arguments must be copied).
pub fn stores_args(name: &str) -> bool {
    matches!(name, "push" | "insert" | "push_back" | "push_front" | "extend" | "append" | "fill" | "entry" | "or_insert" | "unwrap_or" | "ok_or" | "replace")
}

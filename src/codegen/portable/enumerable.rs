//! Typing of the enumerable collection surface for the portable backends (Go, JS, TS), from the shared table
//! `crate::stdlib_surface::enumerable` (ADR-0018 feature 1), plus the keyed std methods that the surface
//! relies on (`sort_by_key`, `min_by_key`, `max_by_key`, `partition`).

use super::ir::PExpr;
use super::ty::{IntKind, Ty};
use crate::stdlib_surface::enumerable::{lookup, Arg, Ret, METHODS};

fn b(t: Ty) -> Box<Ty> {
    Box::new(t)
}

/// Element type of a receiver that enumerable methods accept.
pub fn elem(recv: &Ty) -> Option<Ty> {
    match recv {
        Ty::Vec(t) | Ty::Iter(t) | Ty::Set(t) | Ty::Range(t) => Some((**t).clone()),
        Ty::Map(k, v) => Some(Ty::Tuple(vec![(**k).clone(), (**v).clone()])),
        _ => None,
    }
}

fn key_fn(e: &Ty, ret: Ty) -> Ty {
    Ty::Fn(vec![e.clone()], b(ret))
}

/// Methods typed here even though they are Rust std methods (one closure over the element).
fn keyed_std(name: &str) -> Option<Ty> {
    match name {
        "sort_by_key" | "min_by_key" | "max_by_key" => Some(Ty::Unknown),
        "partition" => Some(Ty::Bool),
        _ => None,
    }
}

/// Expected type of argument `i`; `None` when `name` is not handled here.
pub fn arg_want(recv: &Ty, name: &str, i: usize, prev: &[PExpr]) -> Option<Ty> {
    let e = elem(recv)?;
    if let Some(ret) = keyed_std(name) {
        return (i == 0).then(|| key_fn(&e, ret));
    }
    let m = METHODS
        .iter()
        .find(|m| m.name == name && m.args.len() > i)?;
    Some(match m.args.get(i)? {
        Arg::KeyFn => key_fn(
            &e,
            if name == "partition_by" {
                Ty::Bool
            } else {
                Ty::Unknown
            },
        ),
        Arg::Zip2Fn => {
            let other = prev
                .first()
                .and_then(|p| elem(&p.ty))
                .unwrap_or(Ty::Unknown);
            Ty::Fn(vec![e, other], b(Ty::Unknown))
        }
        Arg::Elem => e,
        Arg::Count => Ty::usize(),
        Arg::Seq => Ty::Unknown,
    })
}

fn closure_ret(args: &[PExpr], i: usize) -> Option<Ty> {
    match args.get(i).map(|a| &a.ty) {
        Some(Ty::Fn(_, r)) => Some((**r).clone()),
        _ => None,
    }
}

/// Result type; `None` when `name` is not handled here.
pub fn ret_ty(recv: &Ty, name: &str, args: &[PExpr]) -> Option<Ty> {
    let e = elem(recv)?;
    match name {
        "sort_by_key" if args.len() == 1 => return Some(Ty::Unit),
        "min_by_key" | "max_by_key" if args.len() == 1 => return Some(Ty::Option(b(e))),
        "partition" if args.len() == 1 => {
            return Some(Ty::Tuple(vec![Ty::Vec(b(e.clone())), Ty::Vec(b(e))]))
        }
        _ => {}
    }
    let m = lookup(name, args.len())?;
    let list = Ty::Vec(b(e.clone()));
    Some(match m.ret {
        Ret::GroupMap => Ty::Map(b(closure_ret(args, 0)?), b(list)),
        Ret::CountMap => Ty::Map(b(closure_ret(args, 0)?), b(Ty::Int(IntKind::I64))),
        Ret::TallyMap => Ty::Map(b(e), b(Ty::Int(IntKind::I64))),
        Ret::ClosureResult => closure_ret(args, 0)?,
        Ret::List => list,
        Ret::ListOfLists => Ty::Vec(b(list)),
        Ret::ListPair => Ty::Tuple(vec![list.clone(), list]),
        Ret::ClosureList => Ty::Vec(b(closure_ret(args, 1)?)),
        Ret::IndexOpt => Ty::Option(b(Ty::usize())),
        Ret::Unit => Ty::Unit,
    })
}

//! Typing rules for built-in std methods and constructors.
//!
//! `arg_want` gives the expected type of argument `i` (so closures get typed parameters);
//! `ret_ty` gives the result type. A `None` from `ret_ty` means "not a supported built-in".

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

pub fn arg_want(recv: &Ty, name: &str, i: usize, prev: &[PExpr]) -> Option<Ty> {
    if let Some(t) = super::ordering::arg_want(recv, iter_elem(recv).as_ref(), name, i) {
        return Some(t);
    }
    if let Some(t) = super::enumerable::arg_want(recv, name, i, prev) {
        return Some(t);
    }
    match recv {
        Ty::Vec(t) => vec_arg_want(t, name, i, prev),
        Ty::Iter(_) | Ty::Range(_) => iter_arg_want(&iter_elem(recv)?, name, i, prev),
        Ty::Str => str_arg_want(name, i),
        Ty::Map(k, v) => match (name, i) {
            ("insert", 0) | ("get", 0) | ("contains_key", 0) | ("remove", 0) | ("entry", 0) | ("get_mut", 0) => Some((**k).clone()),
            ("insert", 1) => Some((**v).clone()),
            _ => None,
        },
        Ty::Set(t) => (i == 0).then(|| (**t).clone()),
        Ty::Option(t) => option_arg_want(t, name, i),
        Ty::Result(t, e) => result_arg_want(t, e, name, i),
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

fn str_arg_want(name: &str, i: usize) -> Option<Ty> {
    match (name, i) {
        ("push", 0) => Some(Ty::Char),
        ("push_str", 0) | ("replace", _) | ("starts_with", 0) | ("ends_with", 0) | ("contains", 0) | ("find", 0) | ("split", 0) | ("trim_matches", 0) => Some(Ty::Str),
        ("repeat", 0) | ("substring", _) | ("truncate", 0) | ("remove", 0) | ("char_at", 0) => Some(Ty::usize()),
        _ => None,
    }
}

fn option_arg_want(t: &Ty, name: &str, i: usize) -> Option<Ty> {
    match (name, i) {
        ("unwrap_or", 0) => Some(t.clone()),
        ("map", 0) | ("and_then", 0) | ("map_or", 1) | ("filter", 0) | ("unwrap_or_else", 0) | ("is_some_and", 0) => {
            Some(fnty(if name == "unwrap_or_else" { vec![] } else { vec![t.clone()] }, Ty::Unknown))
        }
        ("expect", 0) => Some(Ty::Str),
        _ => None,
    }
}

fn result_arg_want(t: &Ty, e: &Ty, name: &str, i: usize) -> Option<Ty> {
    match (name, i) {
        ("unwrap_or", 0) => Some(t.clone()),
        ("map", 0) | ("and_then", 0) => Some(fnty(vec![t.clone()], Ty::Unknown)),
        ("map_err", 0) | ("unwrap_or_else", 0) => Some(fnty(vec![e.clone()], Ty::Unknown)),
        ("expect", 0) => Some(Ty::Str),
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

pub fn ret_ty(recv: &Ty, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
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
        Ty::Str => str_ret(name, want),
        Ty::Map(k, v) => map_ret(k, v, name),
        Ty::Set(t) => set_ret(t, name),
        Ty::Option(t) => option_ret(t, name, args),
        Ty::Result(t, e) => result_ret(t, e, name, args),
        Ty::Int(_) => int_ret(recv, name),
        Ty::F64 | Ty::F32 => float_ret(recv, name),
        Ty::Char => char_ret(name),
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

fn str_ret(name: &str, want: Option<&Ty>) -> Option<Ty> {
    Some(match name {
        "len" => Ty::usize(),
        "is_empty" | "contains" | "starts_with" | "ends_with" | "is_numeric" => Ty::Bool,
        "to_uppercase" | "to_lowercase" | "trim" | "trim_start" | "trim_end" | "as_str" | "replace" | "repeat" | "substring" | "into_string" | "to_ascii_uppercase" | "to_ascii_lowercase" | "trim_matches" => Ty::Str,
        "push" | "push_str" | "clear" | "truncate" => Ty::Unit,
        "chars" => iter_of(Ty::Char),
        "bytes" | "as_bytes" => iter_of(Ty::Int(IntKind::U8)),
        "split" | "lines" | "split_whitespace" => iter_of(Ty::Str),
        "char_indices" => iter_of(pair(Ty::usize(), Ty::Char)),
        "find" => Ty::Option(b(Ty::usize())),
        "pop" => Ty::Option(b(Ty::Char)),
        "parse" => parse_ty(want)?,
        _ => return None,
    })
}

fn parse_ty(want: Option<&Ty>) -> Option<Ty> {
    match want {
        Some(Ty::Result(ok, _)) => Some(Ty::Result(ok.clone(), b(Ty::Str))),
        _ => None,
    }
}

fn map_ret(k: &Ty, v: &Ty, name: &str) -> Option<Ty> {
    Some(match name {
        "len" => Ty::usize(),
        "is_empty" | "contains_key" => Ty::Bool,
        "insert" | "remove" => Ty::Option(b(v.clone())),
        "get" | "get_mut" => Ty::Option(b(v.clone())),
        "clear" => Ty::Unit,
        "keys" | "into_keys" => iter_of(k.clone()),
        "values" | "values_mut" | "into_values" => iter_of(v.clone()),
        "iter" | "iter_mut" | "into_iter" => iter_of(pair(k.clone(), v.clone())),
        _ => return None,
    })
}

fn set_ret(t: &Ty, name: &str) -> Option<Ty> {
    Some(match name {
        "len" => Ty::usize(),
        "is_empty" | "contains" | "insert" | "remove" => Ty::Bool,
        "clear" => Ty::Unit,
        "iter" | "into_iter" => iter_of(t.clone()),
        _ => return None,
    })
}

fn option_ret(t: &Ty, name: &str, args: &[PExpr]) -> Option<Ty> {
    let closure_ret = || match args.first().map(|a| &a.ty) {
        Some(Ty::Fn(_, r)) => Some((**r).clone()),
        _ => None,
    };
    Some(match name {
        "is_some" | "is_none" | "is_some_and" => Ty::Bool,
        "unwrap" | "expect" | "unwrap_or" | "unwrap_or_default" | "unwrap_or_else" => t.clone(),
        "map" => Ty::Option(b(closure_ret()?)),
        "and_then" => closure_ret()?,
        "filter" | "take" => Ty::Option(b(t.clone())),
        "ok_or" => Ty::Result(b(t.clone()), b(args.first()?.ty.clone())),
        "map_or" => args.first()?.ty.clone(),
        _ => return None,
    })
}

fn result_ret(t: &Ty, e: &Ty, name: &str, args: &[PExpr]) -> Option<Ty> {
    let closure_ret = || match args.first().map(|a| &a.ty) {
        Some(Ty::Fn(_, r)) => Some((**r).clone()),
        _ => None,
    };
    Some(match name {
        "is_ok" | "is_err" => Ty::Bool,
        "unwrap" | "expect" | "unwrap_or" | "unwrap_or_default" | "unwrap_or_else" => t.clone(),
        "unwrap_err" | "expect_err" => e.clone(),
        "map" => Ty::Result(b(closure_ret()?), b(e.clone())),
        "map_err" => Ty::Result(b(t.clone()), b(closure_ret()?)),
        "and_then" => closure_ret()?,
        "ok" => Ty::Option(b(t.clone())),
        "err" => Ty::Option(b(e.clone())),
        _ => return None,
    })
}

fn int_ret(recv: &Ty, name: &str) -> Option<Ty> {
    Some(match name {
        "abs" | "pow" | "min" | "max" | "signum" | "wrapping_add" | "wrapping_sub" | "wrapping_mul" | "wrapping_neg" | "saturating_add" | "saturating_sub" | "saturating_mul" | "rem_euclid" | "div_euclid" | "rotate_left" | "rotate_right" | "clamp" | "swap_bytes" | "abs_diff" | "isqrt" | "wrapping_shl" | "wrapping_shr" => recv.clone(),
        "checked_add" | "checked_sub" | "checked_mul" | "checked_div" | "checked_rem" => Ty::Option(b(recv.clone())),
        "count_ones" | "count_zeros" | "leading_zeros" | "trailing_zeros" => Ty::Int(IntKind::U32),
        "is_positive" | "is_negative" | "is_power_of_two" => Ty::Bool,
        _ => return None,
    })
}

fn float_ret(recv: &Ty, name: &str) -> Option<Ty> {
    Some(match name {
        "sqrt" | "abs" | "floor" | "ceil" | "round" | "trunc" | "powi" | "powf" | "sin" | "cos" | "tan" | "ln" | "log10" | "log2" | "exp" | "min" | "max" | "signum" | "fract" | "atan2" | "atan" | "asin" | "acos" | "hypot" | "clamp" | "cbrt" | "to_degrees" | "to_radians" | "mul_add" => recv.clone(),
        "is_nan" | "is_infinite" | "is_finite" | "is_sign_negative" | "is_sign_positive" => Ty::Bool,
        _ => return None,
    })
}

fn char_ret(name: &str) -> Option<Ty> {
    Some(match name {
        "is_alphabetic" | "is_numeric" | "is_alphanumeric" | "is_whitespace" | "is_uppercase" | "is_lowercase" | "is_ascii_digit" | "is_ascii_alphabetic" | "is_ascii_alphanumeric" | "is_ascii_uppercase" | "is_ascii_lowercase" | "is_ascii_punctuation" | "is_ascii" | "is_digit" => Ty::Bool,
        "to_ascii_uppercase" | "to_ascii_lowercase" => Ty::Char,
        "to_uppercase" | "to_lowercase" => Ty::Str,
        "to_digit" => Ty::Option(b(Ty::Int(IntKind::U32))),
        "len_utf8" => Ty::usize(),
        _ => return None,
    })
}

/// `f64::MAX`, `f32::EPSILON`, `f64::NAN`, ...: the exact value (an f32 constant is its f32 value widened).
pub fn float_const(owner: &str, member: &str) -> Option<f64> {
    let is32 = match owner {
        "f32" => true,
        "f64" => false,
        _ => return None,
    };
    Some(match (member, is32) {
        ("MAX", false) => f64::MAX,
        ("MIN", false) => f64::MIN,
        ("EPSILON", false) => f64::EPSILON,
        ("MIN_POSITIVE", false) => f64::MIN_POSITIVE,
        ("MAX", true) => f64::from(f32::MAX),
        ("MIN", true) => f64::from(f32::MIN),
        ("EPSILON", true) => f64::from(f32::EPSILON),
        ("MIN_POSITIVE", true) => f64::from(f32::MIN_POSITIVE),
        ("INFINITY", _) => f64::INFINITY,
        ("NEG_INFINITY", _) => f64::NEG_INFINITY,
        ("NAN", _) => f64::NAN,
        _ => return None,
    })
}

/// Static constructors / associated functions on std types, e.g. `Vec::new`, `String::from`.
pub fn static_ret(owner: &str, name: &str, args: &[PExpr], want: Option<&Ty>) -> Option<Ty> {
    let from_want = |pick: fn(&Ty) -> bool, dflt: Ty| want.filter(|w| pick(w)).cloned().unwrap_or(dflt);
    Some(match (owner, name) {
        ("Vec", "new") | ("Vec", "with_capacity") => from_want(|w| matches!(w, Ty::Vec(_)), Ty::Vec(b(Ty::Unknown))),
        ("Vec", "from") => args.first()?.ty.clone(),
        ("HashMap", "new") | ("HashMap", "with_capacity") | ("BTreeMap", "new") => from_want(|w| matches!(w, Ty::Map(..)), Ty::Map(b(Ty::Unknown), b(Ty::Unknown))),
        ("HashSet", "new") | ("HashSet", "with_capacity") | ("BTreeSet", "new") => from_want(|w| matches!(w, Ty::Set(_)), Ty::Set(b(Ty::Unknown))),
        ("String", "new") | ("String", "from") | ("String", "with_capacity") | ("string", "new") | ("string", "from") => Ty::Str,
        ("i64" | "i32" | "u64" | "u32" | "u8" | "usize" | "isize" | "i8" | "i16" | "u16", "from") => Ty::Int(IntKind::from_name(owner)?),
        ("i64" | "i32" | "u64" | "u32" | "u8" | "usize" | "isize" | "i8" | "i16" | "u16", "MAX" | "MIN") => Ty::Int(IntKind::from_name(owner)?),
        ("i64" | "i32" | "u64" | "u32" | "u8" | "usize" | "isize" | "i8" | "i16" | "u16", "from_str_radix") => {
            Ty::Result(b(Ty::Int(IntKind::from_name(owner)?)), b(Ty::Str))
        }
        ("f64", "from") => Ty::F64,
        ("f32", m) if float_const("f32", m).is_some() => Ty::F32,
        ("f64", m) if float_const("f64", m).is_some() => Ty::F64,
        ("char", "from_u32") => Ty::Option(b(Ty::Char)),
        (o, v) if super::ordering::is_path(o) && super::ordering::variant_value(v).is_some() => super::ordering::ty(),
        _ => return None,
    })
}

/// Methods whose arguments are stored (so place arguments must be copied).
pub fn stores_args(name: &str) -> bool {
    matches!(name, "push" | "insert" | "push_back" | "push_front" | "extend" | "append" | "fill" | "entry" | "or_insert" | "unwrap_or" | "ok_or" | "replace")
}

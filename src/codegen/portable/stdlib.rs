//! Portable subset of `std::*` modules shared by the Go and JS/TS backends.
//!
//! Each entry mirrors the signature of the corresponding `std/<module>.wj` function (which wraps
//! `windjammer_runtime`). Emitters map `module::name` to runtime shims
//! (`std/runtime/js/wj_std.mjs`, `std/runtime/go/wj_std.go`). Modules not listed here are Rust-only
//! (see std/README.md for the portability tier of each module).

use super::ty::{IntKind, Ty};

pub struct StdSig {
    pub params: Vec<Ty>,
    pub ret: Ty,
}

fn f() -> Ty {
    Ty::F64
}
fn s() -> Ty {
    Ty::Str
}
fn i() -> Ty {
    Ty::Int(IntKind::I64)
}
fn u() -> Ty {
    Ty::Int(IntKind::Usize)
}
fn res(ok: Ty) -> Ty {
    Ty::Result(Box::new(ok), Box::new(Ty::Str))
}
fn sig(params: Vec<Ty>, ret: Ty) -> Option<StdSig> {
    Some(StdSig { params, ret })
}

/// Modules with a portable implementation.
pub const PORTABLE_MODULES: &[&str] = &["math", "strings", "fs", "time"];

pub fn lookup(module: &str, name: &str) -> Option<StdSig> {
    match module {
        "math" => math(name),
        "strings" => strings(name),
        "fs" => fs(name),
        "time" => time(name),
        _ => None,
    }
}

fn math(name: &str) -> Option<StdSig> {
    match name {
        "sqrt" | "cbrt" | "round" | "floor" | "ceil" | "trunc" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "ln" | "log2" | "log10" | "abs_f64" | "signum" | "to_radians" | "to_degrees" => sig(vec![f()], f()),
        "pow" | "atan2" | "hypot" | "min_f64" | "max_f64" => sig(vec![f(), f()], f()),
        "clamp_f64" => sig(vec![f(), f(), f()], f()),
        "abs_i64" => sig(vec![i()], i()),
        "min_i64" | "max_i64" => sig(vec![i(), i()], i()),
        "clamp_i64" => sig(vec![i(), i(), i()], i()),
        "is_nan" | "is_infinite" | "is_finite" => sig(vec![f()], Ty::Bool),
        _ => None,
    }
}

fn strings(name: &str) -> Option<StdSig> {
    match name {
        "to_upper" | "to_lower" | "trim" | "trim_start" | "trim_end" => sig(vec![s()], s()),
        "split" => sig(vec![s(), s()], Ty::Vec(Box::new(s()))),
        "split_lines" | "split_whitespace" => sig(vec![s()], Ty::Vec(Box::new(s()))),
        "join" => sig(vec![Ty::Vec(Box::new(s())), s()], s()),
        "chars" => sig(vec![s()], Ty::Vec(Box::new(Ty::Char))),
        "from_chars" => sig(vec![Ty::Vec(Box::new(Ty::Char))], s()),
        "substring_chars" | "substring" => sig(vec![s(), u(), u()], s()),
        "is_empty" => sig(vec![s()], Ty::Bool),
        "starts_with" | "ends_with" | "contains" => sig(vec![s(), s()], Ty::Bool),
        "replace" => sig(vec![s(), s(), s()], s()),
        "len" | "char_count" => sig(vec![s()], u()),
        "repeat" => sig(vec![s(), u()], s()),
        "byte_at" => sig(vec![s(), u()], Ty::Int(IntKind::U8)),
        _ => None,
    }
}

fn fs(name: &str) -> Option<StdSig> {
    match name {
        "read_to_string" => sig(vec![s()], res(s())),
        "write" | "append" => sig(vec![s(), s()], res(Ty::Unit)),
        "remove_file" | "create_dir_all" | "remove_dir_all" => sig(vec![s()], res(Ty::Unit)),
        "exists" | "is_file" | "is_dir" => sig(vec![s()], Ty::Bool),
        _ => None,
    }
}

fn time(name: &str) -> Option<StdSig> {
    match name {
        "now_millis" | "now_unix" | "timestamp" => sig(vec![], i()),
        "sleep_millis" => sig(vec![Ty::Int(IntKind::U64)], Ty::Unit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_sqrt_is_f64_to_f64() {
        let s = lookup("math", "sqrt").unwrap();
        assert_eq!(s.params, vec![Ty::F64]);
        assert_eq!(s.ret, Ty::F64);
    }

    #[test]
    fn unknown_modules_are_rust_only() {
        assert!(lookup("http", "get").is_none());
        assert!(lookup("math", "no_such_fn").is_none());
    }
}

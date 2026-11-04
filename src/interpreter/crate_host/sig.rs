//! Surface functions as the crate host sees them: the wire type of every parameter and of the result.
//!
//! A surface (ADR-0026) declares a crate function in Windjammer types. The passing convention is the compiled
//! build's (`compiler::rust_surface_metadata`): `string` parameters are `&str`, `&T` / `&mut T` borrow a `T`,
//! everything else is owned. The host decodes owned values and borrows them for the call, so a `&mut T`
//! parameter is copy-in/copy-out: the host sends the value back and the interpreter writes it to the place.

use crate::ir::effect_facts::EffectKind;
use crate::parser::{FunctionDecl, Type};
use std::collections::BTreeSet;

/// The types that cross the crate-host pipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireTy {
    Unit,
    /// A Rust integer type by name (`i64`, `usize`, ...).
    Int(&'static str),
    /// `f32` or `f64`.
    Float(&'static str),
    Bool,
    Char,
    Str,
    Vec(Box<WireTy>),
    Option(Box<WireTy>),
    Result(Box<WireTy>, Box<WireTy>),
    Tuple(Vec<WireTy>),
}

/// How a parameter is handed to the crate function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    Owned,
    /// `&T` (and `string` as `&str`): the host borrows its decoded copy.
    Borrow,
    /// `&mut T`: copy-in/copy-out.
    BorrowMut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateFnSig {
    /// Crate name as a path names it (`-` written `_`).
    pub krate: String,
    pub name: String,
    pub params: Vec<(String, Pass, WireTy)>,
    pub ret: WireTy,
    /// Declared effects: `Some(empty)` for `@pure`, `Some(set)` for `@uses(..)`, `None` for `ffi(<crate>)`.
    pub effects: Option<BTreeSet<EffectKind>>,
}

impl CrateFnSig {
    /// `crate::name`, the key of the host's dispatch table and of the effect log.
    pub fn path(&self) -> String {
        format!("{}::{}", self.krate, self.name)
    }
}

const INTS: &[&str] = &["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "usize"];

/// The wire type of an owned surface type, or the type as written when it cannot cross.
pub fn wire_of(t: &Type) -> Result<WireTy, String> {
    let shown = || crate::type_display::format_wj_type(t);
    Ok(match t {
        Type::Int => WireTy::Int("i64"),
        Type::Int32 => WireTy::Int("i32"),
        Type::Uint => WireTy::Int("u64"),
        Type::Float => WireTy::Float("f64"),
        Type::Bool => WireTy::Bool,
        Type::String => WireTy::Str,
        Type::Custom(n) => match n.as_str() {
            "int" => WireTy::Int("i64"),
            "uint" => WireTy::Int("u64"),
            "float" | "f64" => WireTy::Float("f64"),
            "f32" => WireTy::Float("f32"),
            "bool" => WireTy::Bool,
            "char" => WireTy::Char,
            "string" | "String" | "str" => WireTy::Str,
            n => match INTS.iter().find(|i| **i == n) {
                Some(i) => WireTy::Int(i),
                None => return Err(shown()),
            },
        },
        Type::Vec(inner) => WireTy::Vec(Box::new(wire_of(inner)?)),
        Type::Option(inner) => WireTy::Option(Box::new(wire_of(inner)?)),
        Type::Result(a, b) => WireTy::Result(Box::new(wire_of(a)?), Box::new(wire_of(b)?)),
        Type::Tuple(ts) if ts.is_empty() => WireTy::Unit,
        Type::Tuple(ts) => WireTy::Tuple(ts.iter().map(wire_of).collect::<Result<_, _>>()?),
        Type::Parameterized(n, args) => match (n.as_str(), args.as_slice()) {
            ("Vec", [a]) => WireTy::Vec(Box::new(wire_of(a)?)),
            ("Option", [a]) => WireTy::Option(Box::new(wire_of(a)?)),
            ("Result", [a, b]) => WireTy::Result(Box::new(wire_of(a)?), Box::new(wire_of(b)?)),
            _ => return Err(shown()),
        },
        _ => return Err(shown()),
    })
}

/// A parameter: its passing mode and the wire type of the value the host decodes.
pub fn param_of(t: &Type) -> Result<(Pass, WireTy), String> {
    match t {
        Type::Reference(inner) => Ok((Pass::Borrow, wire_of(inner)?)),
        Type::MutableReference(inner) => Ok((Pass::BorrowMut, wire_of(inner)?)),
        _ => {
            let w = wire_of(t)?;
            // surface `string` parameters are `&str` (ADR-0026)
            Ok((if w == WireTy::Str { Pass::Borrow } else { Pass::Owned }, w))
        }
    }
}

const SUPPORTED: &str = "the crate host carries integers, floats, bool, char, string, Vec, Option, Result, tuples and references to them";

/// The host signature of one surface function, or why it cannot cross (naming the function and the type).
pub fn crate_fn_sig(krate: &str, f: &FunctionDecl<'_>) -> Result<CrateFnSig, String> {
    let mut params = Vec::new();
    for p in &f.parameters {
        let (pass, w) = param_of(&p.type_).map_err(|t| {
            format!("`{krate}::{}`: parameter `{}: {t}` cannot cross to the crate host ({SUPPORTED})", f.name, p.name)
        })?;
        params.push((p.name.clone(), pass, w));
    }
    let ret = match &f.return_type {
        None => WireTy::Unit,
        Some(t) => wire_of(t).map_err(|t| format!("`{krate}::{}`: return type `{t}` cannot cross to the crate host ({SUPPORTED})", f.name))?,
    };
    Ok(CrateFnSig {
        krate: krate.to_string(),
        name: f.name.clone(),
        params,
        ret,
        effects: crate::ir::effect_facts::analysis::declared_effects(f),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sigs(text: &str) -> Vec<Result<CrateFnSig, String>> {
        let s = crate::package::rust_surface::parse_surface("demo", PathBuf::from("rust/demo.wji"), text.to_string());
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        s.fns.iter().map(|f| crate_fn_sig("demo", f)).collect()
    }

    #[test]
    fn the_generated_surface_shapes_map() {
        let got = sigs(
            "@pure\npub fn levenshtein(a: string, b: string) -> usize\npub fn split(s: string) -> Option<Vec<string>>\npub fn hash(buf: &Vec<u8>) -> u32\npub fn fill(v: &mut Vec<i64>, n: i64)\npub fn pair(x: f32) -> Result<(i32, char), string>\n",
        );
        let got: Vec<CrateFnSig> = got.into_iter().map(|r| r.unwrap()).collect();
        assert_eq!(got[0].params[0], ("a".into(), Pass::Borrow, WireTy::Str));
        assert_eq!(got[0].ret, WireTy::Int("usize"));
        assert_eq!(got[0].effects, Some(BTreeSet::new()));
        assert_eq!(got[1].ret, WireTy::Option(Box::new(WireTy::Vec(Box::new(WireTy::Str)))));
        assert_eq!(got[1].effects, None);
        assert_eq!(got[2].params[0], ("buf".into(), Pass::Borrow, WireTy::Vec(Box::new(WireTy::Int("u8")))));
        assert_eq!(got[3].params[0].1, Pass::BorrowMut);
        assert_eq!(got[3].params[1], ("n".into(), Pass::Owned, WireTy::Int("i64")));
        assert_eq!(got[3].ret, WireTy::Unit);
        assert_eq!(
            got[4].ret,
            WireTy::Result(Box::new(WireTy::Tuple(vec![WireTy::Int("i32"), WireTy::Char])), Box::new(WireTy::Str))
        );
        assert_eq!(got[4].params[0].2, WireTy::Float("f32"));
        assert_eq!(got[4].path(), "demo::pair");
    }

    #[test]
    fn a_type_that_cannot_cross_is_named() {
        let got = sigs("pub fn compile(p: string) -> Regex\npub fn take(m: HashMap<string, int>)\n");
        let e0 = got[0].clone().unwrap_err();
        assert!(e0.contains("`demo::compile`: return type `Regex` cannot cross"), "{e0}");
        let e1 = got[1].clone().unwrap_err();
        assert!(e1.contains("`demo::take`: parameter `m: HashMap<"), "{e1}");
    }
}

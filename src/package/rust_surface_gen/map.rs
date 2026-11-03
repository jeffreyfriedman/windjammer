//! Rust types (as `syn` sees them in the crate source) to the Windjammer types a surface can declare.
//!
//! Only what the surface's passing convention can express maps; everything else is an `Err` with the reason
//! the generator reports. The convention (ADR-0026): `string` parameters are passed as `&str`, returns are
//! owned, `&T`/`&mut T` of a mappable `T` are passed by reference, explicit widths mean exactly that Rust type.

use quote::ToTokens;
use std::collections::BTreeMap;
use syn::{GenericArgument, PathArguments, ReturnType, Type};

/// Non-generic type aliases of the crate (`pub type R = Result<usize, E>`), resolved before mapping.
pub type Aliases = BTreeMap<String, Type>;

const SCALARS: &[&str] = &["i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32", "f64", "bool", "char"];

/// Source text of a type, normalised (`& str`, `Vec < u8 >` become `&str`, `Vec<u8>`).
pub fn show(t: &impl ToTokens) -> String {
    let mut s = t.to_token_stream().to_string();
    for (from, to) in [(" :: ", "::"), (":: ", "::"), (" : ", ": "), ("& ", "&"), (" < ", "<"), ("< ", "<"), (" >", ">"), (" ,", ","), ("( ", "("), (" )", ")"), ("[ ", "["), (" ]", "]"), (" ;", ";")] {
        s = s.replace(from, to);
    }
    s
}

/// A parameter type, or why it cannot be declared.
pub fn param(t: &Type, aliases: &Aliases) -> Result<String, String> {
    match strip(t) {
        Type::Reference(r) => {
            let inner = strip(&r.elem);
            if r.mutability.is_none() && is_named(inner, "str") {
                return Ok("string".into());
            }
            if is_named(inner, "String") {
                return Err(format!("takes `{}`: a Windjammer `string` argument is passed as `&str`", show(t)));
            }
            let target = match inner {
                Type::Slice(s) => format!("Vec<{}>", owned(&s.elem, aliases)?),
                _ => owned(inner, aliases)?,
            };
            Ok(format!("{}{target}", if r.mutability.is_some() { "&mut " } else { "&" }))
        }
        _ if is_named(strip(t), "String") => {
            Err("takes an owned `String`: surface `string` parameters are passed as `&str`".into())
        }
        Type::ImplTrait(_) => Err(format!("takes `{}` (a trait-bound parameter)", show(t))),
        _ => owned(t, aliases),
    }
}

/// A return type (`None` for `()`), or why it cannot be declared.
pub fn ret(r: &ReturnType, aliases: &Aliases) -> Result<Option<String>, String> {
    let ReturnType::Type(_, t) = r else { return Ok(None) };
    match strip(t) {
        Type::Tuple(tt) if tt.elems.is_empty() => Ok(None),
        Type::Reference(_) => Err(format!("returns a borrow (`{}`): surface functions return owned values", show(t.as_ref()))),
        Type::ImplTrait(_) => Err(format!("returns `{}` (an opaque trait type)", show(t.as_ref()))),
        Type::Never(_) => Err("never returns (`!`)".into()),
        _ => owned(t, aliases).map(Some),
    }
}

/// An owned value type: scalars, `String`, `Vec`/`Option`/`Result` and tuples of owned value types.
fn owned(t: &Type, aliases: &Aliases) -> Result<String, String> {
    let t = strip(t);
    match t {
        Type::Tuple(tt) if tt.elems.is_empty() => Ok("()".into()),
        Type::Tuple(tt) => {
            let parts: Result<Vec<_>, _> = tt.elems.iter().map(|e| owned(e, aliases)).collect();
            Ok(format!("({})", parts?.join(", ")))
        }
        Type::Path(p) if p.qself.is_none() => {
            let seg = p.path.segments.last().expect("a path has a segment");
            let name = seg.ident.to_string();
            let std_prefix = p.path.segments.len() == 1
                || p.path.segments.first().is_some_and(|s| matches!(s.ident.to_string().as_str(), "std" | "core" | "alloc"));
            let args: Vec<&Type> = match &seg.arguments {
                PathArguments::AngleBracketed(a) => a
                    .args
                    .iter()
                    .filter_map(|g| if let GenericArgument::Type(t) = g { Some(t) } else { None })
                    .collect(),
                PathArguments::None => Vec::new(),
                PathArguments::Parenthesized(_) => return Err(format!("uses the closure type `{}`", show(t))),
            };
            if p.path.segments.len() == 1 && args.is_empty() {
                if let Some(target) = aliases.get(&name) {
                    return owned(target, &Aliases::new()).map_err(|why| format!("`{name}` is `{}`, which {why}", show(target)));
                }
            }
            if std_prefix && args.is_empty() && SCALARS.contains(&name.as_str()) {
                return Ok(name);
            }
            if std_prefix && args.is_empty() && name == "String" {
                return Ok("string".into());
            }
            match (std_prefix, name.as_str(), args.as_slice()) {
                (true, "Vec", [e]) => Ok(format!("Vec<{}>", owned(e, aliases)?)),
                (true, "Option", [e]) => Ok(format!("Option<{}>", owned(e, aliases)?)),
                (true, "Result", [a, b]) => Ok(format!("Result<{}, {}>", owned(a, aliases)?, owned(b, aliases)?)),
                _ if SCALARS.contains(&name.as_str()) || name == "String" => Err(format!("uses `{}`", show(t))),
                _ => Err(format!("uses the crate type `{}` (opaque crate types are not supported yet, ADR-0026)", show(t))),
            }
        }
        Type::Reference(_) => Err(format!("holds a borrow (`{}`) inside a value", show(t))),
        Type::Slice(_) | Type::Array(_) => Err(format!("uses the array type `{}`", show(t))),
        Type::TraitObject(_) => Err(format!("uses the trait object `{}`", show(t))),
        Type::BareFn(_) => Err(format!("uses the function pointer `{}`", show(t))),
        Type::Ptr(_) => Err(format!("uses the raw pointer `{}`", show(t))),
        Type::ImplTrait(_) => Err(format!("uses `{}` (a trait-bound type)", show(t))),
        _ => Err(format!("uses `{}`", show(t))),
    }
}

fn strip(t: &Type) -> &Type {
    match t {
        Type::Paren(p) => strip(&p.elem),
        Type::Group(g) => strip(&g.elem),
        _ => t,
    }
}

fn is_named(t: &Type, name: &str) -> bool {
    matches!(t, Type::Path(p) if p.qself.is_none() && p.path.segments.last().is_some_and(|s| s.ident == name && s.arguments.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(src: &str) -> Result<String, String> {
        param(&syn::parse_str::<Type>(src).unwrap(), &Aliases::new())
    }

    fn r(src: &str) -> Result<Option<String>, String> {
        let f: syn::ItemFn = syn::parse_str(&format!("fn f() {src} {{}}")).unwrap();
        let mut a = Aliases::new();
        a.insert("Res".into(), syn::parse_str("Result<usize, MyErr>").unwrap());
        a.insert("Count".into(), syn::parse_str("u64").unwrap());
        ret(&f.sig.output, &a)
    }

    #[test]
    fn parameters() {
        assert_eq!(p("&str"), Ok("string".into()));
        assert_eq!(p("&'a str"), Ok("string".into()));
        assert_eq!(p("usize"), Ok("usize".into()));
        assert_eq!(p("&[u8]"), Ok("&Vec<u8>".into()));
        assert_eq!(p("&mut Vec<i64>"), Ok("&mut Vec<i64>".into()));
        assert_eq!(p("Option<String>"), Ok("Option<string>".into()));
        assert_eq!(p("std::vec::Vec<f64>"), Ok("Vec<f64>".into()));
        assert!(p("String").unwrap_err().contains("owned `String`"));
        assert!(p("&String").unwrap_err().contains("`&String`"));
        assert!(p("impl AsRef<str>").unwrap_err().contains("trait-bound"));
        assert!(p("Option<&str>").unwrap_err().contains("borrow"));
        assert!(p("Regex").unwrap_err().contains("crate type `Regex`"));
        assert!(p("u128").unwrap_err().contains("`u128`"));
        assert!(p("HashMap<String, u8>").unwrap_err().contains("HashMap<String, u8>"));
    }

    #[test]
    fn returns() {
        assert_eq!(r(""), Ok(None));
        assert_eq!(r("-> ()"), Ok(None));
        assert_eq!(r("-> String"), Ok(Some("string".into())));
        assert_eq!(r("-> Result<Vec<u8>, String>"), Ok(Some("Result<Vec<u8>, string>".into())));
        assert_eq!(r("-> (usize, bool)"), Ok(Some("(usize, bool)".into())));
        assert_eq!(r("-> Count"), Ok(Some("u64".into())));
        assert!(r("-> &str").unwrap_err().contains("returns a borrow"));
        let alias = r("-> Res").unwrap_err();
        assert!(alias.contains("`Res` is `Result<usize, MyErr>`") && alias.contains("crate type `MyErr`"), "{alias}");
    }
}

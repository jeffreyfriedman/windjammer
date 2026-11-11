//! The integer-width view of a Rust type in the generated code: only what can be read off the text with
//! certainty. Anything else is `Unknown`, and an `Unknown` never takes part in a prediction.

use std::collections::{HashMap, HashSet};

const INTS: [&str; 12] = ["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize"];

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int(String),
    Ref(Box<Ty>),
    Opt(Box<Ty>),
    /// `Vec<T>`, `[T; N]` and `[T]`: indexing gives `T`, `len()` gives `usize`.
    Seq(Box<Ty>),
    /// A struct declared in the file.
    Named(String),
    /// `String` (owned).
    Str,
    /// `str` (only ever behind a reference: `&str`).
    StrSlice,
    /// `HashMap`/`BTreeMap` (lookups take `&Q`).
    Map,
    /// `HashSet`/`BTreeSet` (lookups take `&Q`).
    Set,
    /// A closure bound to a local, with its parameter types (`Unknown` where not annotated).
    Closure(Vec<Ty>),
    Unknown,
}

impl Ty {
    pub fn int(width: &str) -> Ty {
        if INTS.contains(&width) {
            Ty::Int(width.to_string())
        } else {
            Ty::Unknown
        }
    }

    /// The type with every outer reference removed (what an operator sees: no impl mixes widths, with or
    /// without references).
    pub fn bare(&self) -> &Ty {
        match self {
            Ty::Ref(t) => t.bare(),
            t => t,
        }
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Ty::Unknown)
    }

    pub fn show(&self) -> String {
        match self {
            Ty::Int(w) => w.clone(),
            Ty::Ref(t) => format!("&{}", t.show()),
            Ty::Opt(t) => format!("Option<{}>", t.show()),
            Ty::Seq(t) => format!("Vec<{}>", t.show()),
            Ty::Named(n) => n.clone(),
            Ty::Str => "String".to_string(),
            Ty::StrSlice => "str".to_string(),
            Ty::Map => "HashMap<..>".to_string(),
            Ty::Set => "HashSet<..>".to_string(),
            Ty::Closure(_) => "closure".to_string(),
            Ty::Unknown => "_".to_string(),
        }
    }
}

/// Two types rustc certainly will not accept one for the other where the first is required (a let annotation, a
/// parameter, a field, an assignment, the arms of a `match`): integer widths differ (Rust never converts them), an
/// owned value is given where a reference is required or the reverse, or `str` is given where `String` is.
/// Behind a reference one level of deref coercion may apply (`&String` for `&str`, `&&T` for `&T`), so there only
/// the widths and `String`/`str` are compared.
pub fn conflict(a: &Ty, b: &Ty) -> bool {
    conflict_at(a, b, false)
}

fn conflict_at(a: &Ty, b: &Ty, under_ref: bool) -> bool {
    let value = |t: &Ty| matches!(t, Ty::Int(_) | Ty::Str);
    let referent = |t: &Ty| matches!(t.bare(), Ty::Int(_) | Ty::Str | Ty::StrSlice);
    match (a, b) {
        (Ty::Int(x), Ty::Int(y)) => x != y,
        (Ty::Opt(x), Ty::Opt(y)) | (Ty::Seq(x), Ty::Seq(y)) => conflict_at(x, y, false),
        (Ty::Ref(x), Ty::Ref(y)) => conflict_at(x, y, true),
        (Ty::Str, Ty::StrSlice) => true,
        (Ty::Ref(x), y) if !under_ref => referent(x) && value(y),
        (x, Ty::Ref(y)) if !under_ref => value(x) && referent(y),
        _ => false,
    }
}

/// Item names of the file, so that a user item named like a prelude item (`Option`, `Vec`, `Some`) turns the
/// prelude reading off instead of being misread.
pub struct Items {
    pub defined: HashSet<String>,
    /// Names defined exactly once in the file.
    pub once: HashSet<String>,
    pub structs: HashMap<String, HashMap<String, Ty>>,
}

impl Items {
    pub fn prelude(&self, name: &str) -> bool {
        !self.defined.contains(name)
    }

    /// The width view of a written type; `self_ty` is the type `Self` names inside an `impl`.
    pub fn of(&self, ty: &syn::Type, self_ty: Option<&str>) -> Ty {
        match ty {
            syn::Type::Reference(r) => Ty::Ref(Box::new(self.of(&r.elem, self_ty))),
            syn::Type::Paren(p) => self.of(&p.elem, self_ty),
            syn::Type::Group(g) => self.of(&g.elem, self_ty),
            syn::Type::Array(a) => Ty::Seq(Box::new(self.of(&a.elem, self_ty))),
            syn::Type::Slice(s) => Ty::Seq(Box::new(self.of(&s.elem, self_ty))),
            syn::Type::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
                let seg = &p.path.segments[0];
                let name = seg.ident.to_string();
                let arg = || match &seg.arguments {
                    syn::PathArguments::AngleBracketed(a) if a.args.len() == 1 => match &a.args[0] {
                        syn::GenericArgument::Type(t) => self.of(t, self_ty),
                        _ => Ty::Unknown,
                    },
                    _ => Ty::Unknown,
                };
                let plain = matches!(seg.arguments, syn::PathArguments::None);
                if plain && INTS.contains(&name.as_str()) && self.prelude(&name) {
                    Ty::int(&name)
                } else if name == "Option" && self.prelude(&name) {
                    Ty::Opt(Box::new(arg()))
                } else if name == "Vec" && self.prelude(&name) {
                    Ty::Seq(Box::new(arg()))
                } else if plain && name == "String" && self.prelude(&name) {
                    Ty::Str
                } else if plain && name == "str" {
                    Ty::StrSlice
                } else if matches!(name.as_str(), "HashMap" | "BTreeMap") && self.prelude(&name) {
                    Ty::Map
                } else if matches!(name.as_str(), "HashSet" | "BTreeSet") && self.prelude(&name) {
                    Ty::Set
                } else if plain && name == "Self" {
                    self_ty.map_or(Ty::Unknown, |s| Ty::Named(s.to_string()))
                } else if plain && self.structs.contains_key(&name) {
                    Ty::Named(name)
                } else {
                    Ty::Unknown
                }
            }
            _ => Ty::Unknown,
        }
    }
}

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
            Ty::Unknown => "_".to_string(),
        }
    }
}

/// Two types of the same shape whose integer widths differ somewhere: rustc rejects every place that
/// requires one where the other is given (Rust never converts between integer widths implicitly).
pub fn conflict(a: &Ty, b: &Ty) -> bool {
    match (a, b) {
        (Ty::Int(x), Ty::Int(y)) => x != y,
        (Ty::Opt(x), Ty::Opt(y)) | (Ty::Seq(x), Ty::Seq(y)) | (Ty::Ref(x), Ty::Ref(y)) => conflict(x, y),
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

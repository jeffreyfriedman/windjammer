//! Per-function binding facts. A name has a *known type* only when every
//! binding occurrence of that name in the function (params, lets, closure
//! params, match/for/if-let patterns, nested items) is a declaration whose
//! type is syntactically evident and identical. Anything else is unknown.

use std::collections::HashMap;
use syn::visit::Visit;
use syn::{Block, Expr, ExprLit, FnArg, Lit, Local, Pat, Signature, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownTy {
    /// Primitive Copy scalar (`i32`, `usize`, `f64`, `bool`, `char`, ...).
    Prim(String),
    /// Owned `std::string::String`.
    String,
}

const PRIMS: &[&str] = &[
    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32",
    "f64", "bool", "char",
];

pub fn prim_name(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            let s = &p.path.segments[0];
            if s.arguments.is_none() && PRIMS.contains(&s.ident.to_string().as_str()) {
                return Some(s.ident.to_string());
            }
        }
    }
    None
}

fn known_from_type(ty: &Type) -> Option<KnownTy> {
    if let Some(p) = prim_name(ty) {
        return Some(KnownTy::Prim(p));
    }
    if let Type::Path(p) = ty {
        if p.qself.is_none() && p.path.segments.len() == 1 {
            let s = &p.path.segments[0];
            if s.arguments.is_none() && s.ident == "String" {
                return Some(KnownTy::String);
            }
        }
    }
    None
}

fn path_is(e: &Expr, segs: &[&str]) -> bool {
    if let Expr::Path(p) = e {
        let got: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
        return got.len() == segs.len() && got.iter().zip(segs).all(|(a, b)| a == b);
    }
    false
}

/// Type evident from an initializer expression alone.
fn known_from_init(e: &Expr) -> Option<KnownTy> {
    match e {
        Expr::Lit(ExprLit { lit, .. }) => match lit {
            Lit::Int(i) if !i.suffix().is_empty() => Some(KnownTy::Prim(i.suffix().to_string())),
            Lit::Float(f) if !f.suffix().is_empty() => Some(KnownTy::Prim(f.suffix().to_string())),
            Lit::Bool(_) => Some(KnownTy::Prim("bool".into())),
            _ => None,
        },
        Expr::Cast(c) => prim_name(&c.ty).map(KnownTy::Prim),
        Expr::Call(c) if path_is(&c.func, &["String", "new"]) => Some(KnownTy::String),
        Expr::Call(c) if path_is(&c.func, &["String", "from"]) => Some(KnownTy::String),
        _ => None,
    }
}

#[derive(Default)]
pub struct FnFacts {
    total: HashMap<String, usize>,
    known: HashMap<String, Vec<KnownTy>>,
}

impl FnFacts {
    pub fn of(sig: &Signature, body: &Block) -> Self {
        let mut c = Collector::default();
        for arg in &sig.inputs {
            if let FnArg::Typed(pt) = arg {
                if let Pat::Ident(pi) = &*pt.pat {
                    if let Some(k) = known_from_type(&pt.ty) {
                        c.facts.known.entry(pi.ident.to_string()).or_default().push(k);
                    }
                }
            }
            c.visit_fn_arg(arg);
        }
        c.visit_block(body);
        c.facts
    }

    /// Type of `name` iff every binding occurrence agrees (and at least one exists).
    pub fn unique(&self, name: &str) -> Option<&KnownTy> {
        let total = *self.total.get(name)?;
        let known = self.known.get(name)?;
        let first = known.first()?;
        (known.len() == total && known.iter().all(|k| k == first)).then_some(first)
    }

    /// Mutant support: type of the first declaration regardless of other bindings.
    pub fn first_declared(&self, name: &str) -> Option<&KnownTy> {
        self.known.get(name)?.first()
    }
}

#[derive(Default)]
struct Collector {
    facts: FnFacts,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_pat_ident(&mut self, pi: &'ast syn::PatIdent) {
        *self.facts.total.entry(pi.ident.to_string()).or_default() += 1;
        syn::visit::visit_pat_ident(self, pi);
    }

    fn visit_local(&mut self, l: &'ast Local) {
        let (name, ty_known) = match &l.pat {
            Pat::Type(pt) => match &*pt.pat {
                Pat::Ident(pi) => (Some(pi.ident.to_string()), known_from_type(&pt.ty)),
                _ => (None, None),
            },
            Pat::Ident(pi) => (
                Some(pi.ident.to_string()),
                l.init.as_ref().and_then(|i| known_from_init(&i.expr)),
            ),
            _ => (None, None),
        };
        if let (Some(n), Some(k)) = (name, ty_known) {
            self.facts.known.entry(n).or_default().push(k);
        }
        syn::visit::visit_local(self, l);
    }
}

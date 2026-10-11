//! AST -> portable IR lowering (typing, ownership/copy insertion, desugaring).

mod call;
mod expr;
mod fmt;
mod intrinsics;
mod items;
mod pat;
mod stmt;

use super::env::ItemEnv;
use super::ir::*;
use super::ty::Ty;
use crate::analyzer::SignatureRegistry;
use crate::parser::Program;
use std::collections::HashMap;

pub use items::lower_program;

#[derive(Clone)]
pub(super) struct Local {
    pub ty: Ty,
    /// Alias of caller-owned storage (`&`/`&mut` param or `self` by reference).
    pub alias: bool,
    /// Unique emitted name (shadowed declarations get a numeric suffix).
    pub out: String,
    /// Declared `mut` (closures snapshot such captures).
    pub mutable: bool,
}

pub(super) struct Lowerer<'e> {
    pub env: &'e ItemEnv,
    pub scopes: Vec<HashMap<String, Local>>,
    /// Return types of the enclosing function/closure chain.
    pub rets: Vec<Ty>,
    pub generics: Vec<String>,
    /// Trait bounds of in-scope generic params: param name -> trait names.
    pub bounds: HashMap<String, Vec<String>>,
    pub owner: Option<String>,
    pub tmp: usize,
    pub line: u32,
    pub used: std::collections::HashSet<String>,
    pub vars: Vec<Option<Ty>>,
    /// Inference variables introduced by integer / float literals (default to i64 / f64).
    pub int_vars: std::collections::HashSet<u32>,
    pub float_vars: std::collections::HashSet<u32>,
}

impl<'e> Lowerer<'e> {
    pub fn new(env: &'e ItemEnv) -> Self {
        Lowerer { env, scopes: vec![HashMap::new()], rets: vec![], generics: vec![], bounds: HashMap::new(), owner: None, tmp: 0, line: 0, used: Default::default(), vars: vec![], int_vars: Default::default(), float_vars: Default::default() }
    }

    pub fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    /// Declare a local; returns its unique emitted name.
    pub fn declare(&mut self, name: &str, ty: Ty, alias: bool) -> String {
        let mut out = name.to_string();
        let mut n = 1;
        while self.used.contains(&out) {
            n += 1;
            out = format!("{name}_{n}");
        }
        self.used.insert(out.clone());
        self.scopes.last_mut().unwrap().insert(name.to_string(), Local { ty, alias, out: out.clone(), mutable: false });
        out
    }

    pub fn set_mutable(&mut self, name: &str) {
        if let Some(l) = self.scopes.last_mut().and_then(|s| s.get_mut(name)) {
            l.mutable = true;
        }
    }

    pub fn new_var(&mut self) -> Ty {
        self.vars.push(None);
        Ty::Var(self.vars.len() as u32 - 1)
    }

    /// A fresh variable for an unsuffixed numeric literal.
    pub fn lit_var(&mut self, float: bool) -> Ty {
        let t = self.new_var();
        if let Ty::Var(i) = t {
            if float { self.float_vars.insert(i) } else { self.int_vars.insert(i) };
        }
        t
    }

    /// Resolve; unbound literal variables take their default type (i64 / f64).
    pub fn resolve_final(&self, t: &Ty) -> Ty {
        match self.resolve(t) {
            Ty::Var(i) if self.int_vars.contains(&i) => Ty::i64(),
            Ty::Var(i) if self.float_vars.contains(&i) => Ty::F64,
            Ty::Vec(a) => Ty::Vec(Box::new(self.resolve_final(&a))),
            Ty::Set(a) => Ty::Set(Box::new(self.resolve_final(&a))),
            Ty::Option(a) => Ty::Option(Box::new(self.resolve_final(&a))),
            Ty::Iter(a) => Ty::Iter(Box::new(self.resolve_final(&a))),
            Ty::Range(a) => Ty::Range(Box::new(self.resolve_final(&a))),
            Ty::Map(k, v) => Ty::Map(Box::new(self.resolve_final(&k)), Box::new(self.resolve_final(&v))),
            Ty::Result(a, b) => Ty::Result(Box::new(self.resolve_final(&a)), Box::new(self.resolve_final(&b))),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.resolve_final(t)).collect()),
            Ty::Adt(n, a) => Ty::Adt(n, a.iter().map(|t| self.resolve_final(t)).collect()),
            Ty::Fn(p, r) => Ty::Fn(p.iter().map(|t| self.resolve_final(t)).collect(), Box::new(self.resolve_final(&r))),
            other => other,
        }
    }

    /// Resolve for method dispatch: an unbound literal variable is fixed to its default now.
    pub fn resolve_for_dispatch(&mut self, t: &Ty) -> Ty {
        let r = self.resolve(t);
        if let Ty::Var(i) = r {
            let d = if self.int_vars.contains(&i) { Ty::i64() } else if self.float_vars.contains(&i) { Ty::F64 } else { return r };
            self.vars[i as usize] = Some(d.clone());
            return d;
        }
        r
    }

    /// Replace `Unknown` holes with fresh inference variables.
    pub fn freshen(&mut self, t: &Ty) -> Ty {
        match t {
            Ty::Unknown => self.new_var(),
            Ty::Vec(a) => Ty::Vec(Box::new(self.freshen(a))),
            Ty::Set(a) => Ty::Set(Box::new(self.freshen(a))),
            Ty::Option(a) => Ty::Option(Box::new(self.freshen(a))),
            Ty::Iter(a) => Ty::Iter(Box::new(self.freshen(a))),
            Ty::Map(k, v) => Ty::Map(Box::new(self.freshen(k)), Box::new(self.freshen(v))),
            Ty::Result(a, b) => Ty::Result(Box::new(self.freshen(a)), Box::new(self.freshen(b))),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.freshen(t)).collect()),
            Ty::Adt(n, a) => Ty::Adt(n.clone(), a.iter().map(|t| self.freshen(t)).collect()),
            other => other.clone(),
        }
    }

    pub fn resolve(&self, t: &Ty) -> Ty {
        match t {
            Ty::Var(i) => match self.vars.get(*i as usize).cloned().flatten() {
                Some(b) => self.resolve(&b),
                None => t.clone(),
            },
            Ty::Vec(a) => Ty::Vec(Box::new(self.resolve(a))),
            Ty::Set(a) => Ty::Set(Box::new(self.resolve(a))),
            Ty::Option(a) => Ty::Option(Box::new(self.resolve(a))),
            Ty::Iter(a) => Ty::Iter(Box::new(self.resolve(a))),
            Ty::Range(a) => Ty::Range(Box::new(self.resolve(a))),
            Ty::Map(k, v) => Ty::Map(Box::new(self.resolve(k)), Box::new(self.resolve(v))),
            Ty::Result(a, b) => Ty::Result(Box::new(self.resolve(a)), Box::new(self.resolve(b))),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.resolve(t)).collect()),
            Ty::Adt(n, a) => Ty::Adt(n.clone(), a.iter().map(|t| self.resolve(t)).collect()),
            Ty::Fn(p, r) => Ty::Fn(p.iter().map(|t| self.resolve(t)).collect(), Box::new(self.resolve(r))),
            other => other.clone(),
        }
    }

    /// Record what `expected` must be given an `actual` type (binds inference variables).
    pub fn constrain(&mut self, expected: &Ty, actual: &Ty) {
        let (e, a) = (self.resolve(expected), self.resolve(actual));
        match (&e, &a) {
            (Ty::Var(i), other) | (other, Ty::Var(i)) if !matches!(other, Ty::Var(j) if j == i) && !other.is_unknown() => {
                self.vars[*i as usize] = Some(other.clone());
            }
            (Ty::Var(i), Ty::Var(j)) if i != j => self.vars[*i as usize] = Some(Ty::Var(*j)),
            (Ty::Vec(x), Ty::Vec(y)) | (Ty::Set(x), Ty::Set(y)) | (Ty::Option(x), Ty::Option(y)) | (Ty::Iter(x), Ty::Iter(y)) | (Ty::Range(x), Ty::Range(y)) => self.constrain(x, y),
            (Ty::Map(a1, b1), Ty::Map(a2, b2)) | (Ty::Result(a1, b1), Ty::Result(a2, b2)) => {
                self.constrain(a1, a2);
                self.constrain(b1, b2);
            }
            (Ty::Tuple(xs), Ty::Tuple(ys)) => xs.iter().zip(ys).for_each(|(x, y)| self.constrain(x, y)),
            (Ty::Adt(n1, xs), Ty::Adt(n2, ys)) if n1 == n2 => xs.iter().zip(ys).for_each(|(x, y)| self.constrain(x, y)),
            (Ty::Fn(p1, r1), Ty::Fn(p2, r2)) => {
                p1.iter().zip(p2).for_each(|(x, y)| self.constrain(x, y));
                self.constrain(r1, r2);
            }
            _ => {}
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Local> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    pub fn fresh(&mut self, base: &str) -> String {
        self.tmp += 1;
        format!("{base}_{}", self.tmp)
    }

    pub fn err<T>(&self, what: impl Into<String>) -> PResult<T> {
        Err(Unsupported { what: what.into(), line: self.line })
    }

    /// Does an owned use of a place of this type require a defensive copy?
    pub fn needs_copy(&self, ty: &Ty) -> bool {
        match self.resolve(ty) {
            Ty::Unknown | Ty::Var(_) => return true,
            ref t => return self.needs_copy_resolved(t),
        }
    }

    fn needs_copy_resolved(&self, ty: &Ty) -> bool {
        match ty {
            Ty::Adt(n, _) => self.env.structs.contains_key(n),
            Ty::Tuple(_) => true,
            Ty::Vec(_) | Ty::Map(..) | Ty::Set(_) => true,
            Ty::Option(t) => self.needs_copy(t),
            Ty::Result(a, b) => self.needs_copy(a) || self.needs_copy(b),
            _ => false,
        }
    }

    /// Use `e` as an owned value: places of aggregate type are copied (value semantics).
    pub fn own(&self, e: PExpr) -> PExpr {
        if e.is_place() && self.needs_copy(&e.ty) {
            let (ty, line) = (e.ty.clone(), e.line);
            PExpr::new(ty, PK::Copy(e)).at(line)
        } else {
            e
        }
    }

    /// Owned use at a function exit: an own local dies there, aliases must be copied.
    pub fn own_for_return(&self, e: PExpr) -> PExpr {
        if let PK::Local(n) = &*e.kind {
            if self.lookup(n).map(|l| !l.alias).unwrap_or(false) {
                return e;
            }
        }
        self.own(e)
    }
}

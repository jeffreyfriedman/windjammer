//! A scoped walk over the generated Rust that types integer expressions where the text makes the type
//! certain (suffixed literals, casts, annotated bindings, parameters, struct fields, calls of functions and
//! inherent methods of the file, `Option` payloads) and records every place where two different widths
//! must be the same type. Everything else types as `Unknown` and is never reported.

use super::types::{conflict, Items, Ty};
use quote::ToTokens;
use std::collections::HashMap;

/// A place rustc rejects: two integer widths where one type is required.
#[derive(Debug, Clone)]
pub struct Clash {
    /// The enclosing function of the generated code.
    pub function: String,
    /// The generated expression, as Rust text.
    pub expr: String,
    pub left: Ty,
    pub right: Ty,
    /// The first variable the expression names (to place the diagnostic in the source).
    pub var: Option<String>,
}

pub struct Sig {
    pub params: Vec<Ty>,
    pub ret: Ty,
}

pub struct Walker<'a> {
    pub items: &'a Items,
    pub fns: &'a HashMap<String, Sig>,
    /// Inherent methods: (type, method) -> signature without the receiver.
    pub methods: &'a HashMap<(String, String), Sig>,
    pub self_ty: Option<String>,
    pub function: String,
    scopes: Vec<HashMap<String, Ty>>,
    pub found: Vec<Clash>,
}

impl<'a> Walker<'a> {
    pub fn new(items: &'a Items, fns: &'a HashMap<String, Sig>, methods: &'a HashMap<(String, String), Sig>) -> Self {
        Walker { items, fns, methods, self_ty: None, function: String::new(), scopes: Vec::new(), found: Vec::new() }
    }

    pub fn function(&mut self, sig: &syn::Signature, body: &syn::Block) {
        self.function = sig.ident.to_string();
        self.scopes = vec![HashMap::new()];
        for input in &sig.inputs {
            match input {
                syn::FnArg::Receiver(r) => {
                    let s = self.self_ty.clone().map_or(Ty::Unknown, Ty::Named);
                    let t = if r.reference.is_some() { Ty::Ref(Box::new(s)) } else { s };
                    self.bind("self", t);
                }
                syn::FnArg::Typed(p) => {
                    let t = self.items.of(&p.ty, self.self_ty.as_deref());
                    self.pattern(&p.pat, t);
                }
            }
        }
        self.block(body);
    }

    fn bind(&mut self, name: &str, t: Ty) {
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name.to_string(), t);
        }
    }

    fn lookup(&self, name: &str) -> Option<&Ty> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Bind a pattern matched against a value of type `t`.
    fn pattern(&mut self, pat: &syn::Pat, t: Ty) {
        match pat {
            syn::Pat::Ident(p) if p.subpat.is_none() => {
                let name = p.ident.to_string();
                // an upper-case name in a pattern may be a constant or a unit variant, not a binding
                let t = if name.starts_with(|c: char| c.is_ascii_uppercase()) || self.items.defined.contains(&name) {
                    Ty::Unknown
                } else if p.by_ref.is_some() {
                    Ty::Ref(Box::new(t))
                } else {
                    t
                };
                self.bind(&name, t);
            }
            syn::Pat::Type(p) => {
                let t = self.items.of(&p.ty, self.self_ty.as_deref());
                self.pattern(&p.pat, t);
            }
            syn::Pat::Paren(p) => self.pattern(&p.pat, t),
            syn::Pat::TupleStruct(p) if p.elems.len() == 1 && p.path.is_ident("Some") && self.items.prelude("Some") => {
                let inner = match &t {
                    Ty::Opt(x) => (**x).clone(),
                    // default binding mode: matching through a reference binds references
                    Ty::Ref(r) => match r.bare() {
                        Ty::Opt(x) => Ty::Ref(x.clone()),
                        _ => Ty::Unknown,
                    },
                    _ => Ty::Unknown,
                };
                self.pattern(&p.elems[0], inner);
            }
            other => self.unknown_bindings(other),
        }
    }

    fn unknown_bindings(&mut self, pat: &syn::Pat) {
        struct Names(Vec<String>);
        impl<'ast> syn::visit::Visit<'ast> for Names {
            fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
                self.0.push(p.ident.to_string());
                syn::visit::visit_pat_ident(self, p);
            }
        }
        let mut n = Names(Vec::new());
        syn::visit::Visit::visit_pat(&mut n, pat);
        for name in n.0 {
            self.bind(&name, Ty::Unknown);
        }
    }

    fn clash(&mut self, e: &dyn ToTokens, left: &Ty, right: &Ty) {
        let expr = e.to_token_stream().to_string();
        let var = first_var(&e.to_token_stream());
        self.found.push(Clash { function: self.function.clone(), expr, left: left.clone(), right: right.clone(), var });
    }

    /// `need` and `got` must be one type.
    fn require(&mut self, e: &dyn ToTokens, need: &Ty, got: &Ty) {
        if conflict(need, got) {
            self.clash(e, need, got);
        }
    }

    pub fn block(&mut self, b: &syn::Block) -> Ty {
        self.scopes.push(HashMap::new());
        let mut last = Ty::Unknown;
        for (i, s) in b.stmts.iter().enumerate() {
            let tail = i + 1 == b.stmts.len();
            last = Ty::Unknown;
            match s {
                syn::Stmt::Local(l) => {
                    let init = l.init.as_ref().map_or(Ty::Unknown, |i| {
                        if let Some((_, d)) = &i.diverge {
                            self.expr(d);
                        }
                        self.expr(&i.expr)
                    });
                    if let (syn::Pat::Type(p), Some(i)) = (&l.pat, &l.init) {
                        let need = self.items.of(&p.ty, self.self_ty.as_deref());
                        self.require(&i.expr, &need, &init);
                    }
                    self.pattern(&l.pat, init);
                }
                syn::Stmt::Expr(e, semi) => {
                    let t = self.expr(e);
                    if tail && semi.is_none() {
                        last = t;
                    }
                }
                syn::Stmt::Macro(m) => {
                    self.mac(&m.mac);
                }
                syn::Stmt::Item(_) => {}
            }
        }
        self.scopes.pop();
        last
    }

    fn mac(&mut self, m: &syn::Macro) -> Ty {
        use syn::punctuated::Punctuated;
        let parser = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        let args = syn::parse::Parser::parse2(parser, m.tokens.clone()).ok();
        if m.path.is_ident("vec") {
            let toks = &m.tokens;
            if let Ok(r) = syn::parse2::<syn::ExprRepeat>(quote::quote!([#toks])) {
                let t = self.expr(&r.expr);
                self.expr(&r.len);
                return Ty::Seq(Box::new(t));
            }
            return args.map_or(Ty::Unknown, |a| self.elements(&a, &m.tokens));
        }
        for a in args.iter().flatten() {
            self.expr(a);
        }
        Ty::Unknown
    }

    /// Elements of an array or `vec!`: one type for all of them.
    fn elements<'e>(&mut self, elems: impl IntoIterator<Item = &'e syn::Expr>, at: &dyn ToTokens) -> Ty {
        let mut first: Option<Ty> = None;
        for e in elems {
            let t = self.expr(e);
            match &first {
                Some(f) if conflict(f, &t) => {
                    let f = f.clone();
                    self.clash(at, &f, &t);
                }
                None if t.is_known() => first = Some(t),
                _ => {}
            }
        }
        Ty::Seq(Box::new(first.unwrap_or(Ty::Unknown)))
    }

    /// Arms of a `match` or branches of an `if`: one type for all of them.
    fn branches(&mut self, at: &dyn ToTokens, types: &[Ty]) -> Ty {
        let known: Vec<&Ty> = types.iter().filter(|t| t.is_known()).collect();
        if let Some(pair) = known.windows(2).find(|w| conflict(w[0], w[1])) {
            let (a, b) = (pair[0].clone(), pair[1].clone());
            self.clash(at, &a, &b);
            return Ty::Unknown;
        }
        if known.len() == types.len() {
            known.first().map_or(Ty::Unknown, |t| (*t).clone())
        } else {
            Ty::Unknown
        }
    }

    pub fn expr(&mut self, e: &syn::Expr) -> Ty {
        use syn::Expr as E;
        match e {
            E::Lit(l) => match &l.lit {
                syn::Lit::Int(i) => Ty::int(i.suffix()),
                _ => Ty::Unknown,
            },
            E::Paren(p) => self.expr(&p.expr),
            E::Group(g) => self.expr(&g.expr),
            E::Cast(c) => {
                self.expr(&c.expr);
                self.items.of(&c.ty, self.self_ty.as_deref())
            }
            E::Unary(u) => {
                let t = self.expr(&u.expr);
                match u.op {
                    syn::UnOp::Deref(_) => match t {
                        Ty::Ref(x) => *x,
                        _ => Ty::Unknown,
                    },
                    _ => match t.bare() {
                        Ty::Int(w) => Ty::Int(w.clone()),
                        _ => Ty::Unknown,
                    },
                }
            }
            E::Reference(r) => Ty::Ref(Box::new(self.expr(&r.expr))),
            E::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
                self.lookup(&p.path.segments[0].ident.to_string()).cloned().unwrap_or(Ty::Unknown)
            }
            E::Binary(b) => self.binary(e, b),
            E::Assign(a) => {
                let l = self.expr(&a.left);
                let r = self.expr(&a.right);
                self.require(e, &l, &r);
                Ty::Unknown
            }
            E::Index(i) => {
                let base = self.expr(&i.expr);
                let range = matches!(*i.index, E::Range(_));
                self.expr(&i.index);
                match base.bare() {
                    Ty::Seq(t) if range => Ty::Seq(t.clone()),
                    Ty::Seq(t) => (**t).clone(),
                    _ => Ty::Unknown,
                }
            }
            E::Field(f) => {
                let base = self.expr(&f.base);
                match (base.bare(), &f.member) {
                    (Ty::Named(s), syn::Member::Named(m)) => {
                        self.items.structs.get(s).and_then(|fs| fs.get(&m.to_string())).cloned().unwrap_or(Ty::Unknown)
                    }
                    _ => Ty::Unknown,
                }
            }
            E::MethodCall(m) => self.method(e, m),
            E::Call(c) => self.call(e, c),
            E::Macro(m) => self.mac(&m.mac),
            E::Block(b) => self.block(&b.block),
            E::Unsafe(u) => self.block(&u.block),
            E::If(i) => self.if_expr(e, i),
            E::Match(m) => {
                let s = self.expr(&m.expr);
                let mut arms = Vec::new();
                for arm in &m.arms {
                    self.scopes.push(HashMap::new());
                    self.pattern(&arm.pat, s.clone());
                    if let Some((_, g)) = &arm.guard {
                        self.expr(g);
                    }
                    arms.push(self.expr(&arm.body));
                    self.scopes.pop();
                }
                self.branches(e, &arms)
            }
            E::ForLoop(f) => {
                let it = self.expr(&f.expr);
                let item = match (&*f.expr, it) {
                    (E::Range(_), Ty::Int(w)) => Ty::Int(w),
                    _ => Ty::Unknown,
                };
                self.scopes.push(HashMap::new());
                self.pattern(&f.pat, item);
                self.block(&f.body);
                self.scopes.pop();
                Ty::Unknown
            }
            E::While(w) => {
                self.scopes.push(HashMap::new());
                self.condition(&w.cond);
                self.block(&w.body);
                self.scopes.pop();
                Ty::Unknown
            }
            E::Loop(l) => {
                self.block(&l.body);
                Ty::Unknown
            }
            E::Closure(c) => {
                self.scopes.push(HashMap::new());
                for p in &c.inputs {
                    self.pattern(p, Ty::Unknown);
                }
                self.expr(&c.body);
                self.scopes.pop();
                Ty::Unknown
            }
            E::Range(r) => {
                let a = r.start.as_ref().map_or(Ty::Unknown, |x| self.expr(x));
                let b = r.end.as_ref().map_or(Ty::Unknown, |x| self.expr(x));
                if conflict(a.bare(), b.bare()) {
                    self.clash(e, &a, &b);
                }
                if a.is_known() { a } else { b }
            }
            E::Array(a) => self.elements(&a.elems, e),
            E::Repeat(r) => {
                let t = self.expr(&r.expr);
                self.expr(&r.len);
                Ty::Seq(Box::new(t))
            }
            E::Tuple(t) => {
                for x in &t.elems {
                    self.expr(x);
                }
                Ty::Unknown
            }
            E::Struct(s) => self.struct_lit(s),
            E::Return(r) => {
                if let Some(x) = &r.expr {
                    self.expr(x);
                }
                Ty::Unknown
            }
            E::Break(b) => {
                if let Some(x) = &b.expr {
                    self.expr(x);
                }
                Ty::Unknown
            }
            E::Try(t) => {
                self.expr(&t.expr);
                Ty::Unknown
            }
            _ => Ty::Unknown,
        }
    }

    /// The condition of `if`/`while`: an `if let` binds in the scope the caller opened.
    fn condition(&mut self, c: &syn::Expr) {
        if let syn::Expr::Let(l) = c {
            let t = self.expr(&l.expr);
            self.pattern(&l.pat, t);
        } else {
            self.expr(c);
        }
    }

    fn if_expr(&mut self, e: &syn::Expr, i: &syn::ExprIf) -> Ty {
        self.scopes.push(HashMap::new());
        self.condition(&i.cond);
        let then = self.block(&i.then_branch);
        self.scopes.pop();
        match &i.else_branch {
            Some((_, els)) => {
                let other = self.expr(els);
                self.branches(e, &[then, other])
            }
            None => Ty::Unknown,
        }
    }

    fn binary(&mut self, e: &syn::Expr, b: &syn::ExprBinary) -> Ty {
        use syn::BinOp as B;
        let l = self.expr(&b.left);
        let r = self.expr(&b.right);
        match b.op {
            // shifts take any integer width on the right; `&&`/`||` are bool
            B::Shl(_) | B::Shr(_) | B::ShlAssign(_) | B::ShrAssign(_) => l.bare().clone(),
            B::And(_) | B::Or(_) => Ty::Unknown,
            op => {
                if let (Ty::Int(x), Ty::Int(y)) = (l.bare(), r.bare()) {
                    if x != y {
                        self.clash(e, l.bare(), r.bare());
                        return Ty::Unknown;
                    }
                }
                let compares = matches!(op, B::Eq(_) | B::Ne(_) | B::Lt(_) | B::Le(_) | B::Gt(_) | B::Ge(_));
                let assigns = matches!(
                    op,
                    B::AddAssign(_) | B::SubAssign(_) | B::MulAssign(_) | B::DivAssign(_) | B::RemAssign(_) | B::BitAndAssign(_) | B::BitOrAssign(_) | B::BitXorAssign(_)
                );
                if compares || assigns {
                    return Ty::Unknown;
                }
                // the left operand picks the impl: `iN op X` exists only for `X` of width iN (by value or by
                // reference), and its output is iN
                match l.bare() {
                    Ty::Int(x) => Ty::Int(x.clone()),
                    _ => Ty::Unknown,
                }
            }
        }
    }

    fn args(&mut self, at: &dyn ToTokens, params: Option<&[Ty]>, args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>) {
        let got: Vec<Ty> = args.iter().map(|a| self.expr(a)).collect();
        if let Some(ps) = params.filter(|p| p.len() == got.len()) {
            for (p, g) in ps.iter().zip(&got) {
                self.require(at, p, g);
            }
        }
    }

    fn call(&mut self, e: &syn::Expr, c: &syn::ExprCall) -> Ty {
        let syn::Expr::Path(p) = &*c.func else {
            self.expr(&c.func);
            self.args(e, None, &c.args);
            return Ty::Unknown;
        };
        let segs: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
        let generic = p.path.segments.iter().any(|s| !matches!(s.arguments, syn::PathArguments::None));
        if p.qself.is_some() || generic {
            self.args(e, None, &c.args);
            return Ty::Unknown;
        }
        match segs.as_slice() {
            [s] if s == "Some" && self.items.prelude("Some") && c.args.len() == 1 && self.lookup(s).is_none() => {
                Ty::Opt(Box::new(self.expr(&c.args[0])))
            }
            // a local closure of the same name shadows the function
            [f] if self.lookup(f).is_none() && self.fns.contains_key(f) => {
                let fns = self.fns;
                let sig = &fns[f];
                self.args(e, Some(&sig.params), &c.args);
                sig.ret.clone()
            }
            [t, m] => {
                let t = if t == "Self" { self.self_ty.clone().unwrap_or_default() } else { t.clone() };
                let methods = self.methods;
                match methods.get(&(t, m.clone())) {
                    Some(sig) => {
                        self.args(e, Some(&sig.params), &c.args);
                        sig.ret.clone()
                    }
                    None => {
                        self.args(e, None, &c.args);
                        Ty::Unknown
                    }
                }
            }
            _ => {
                self.args(e, None, &c.args);
                Ty::Unknown
            }
        }
    }

    fn method(&mut self, e: &syn::Expr, m: &syn::ExprMethodCall) -> Ty {
        let recv = self.expr(&m.receiver);
        let name = m.method.to_string();
        if m.turbofish.is_some() {
            self.args(e, None, &m.args);
            return Ty::Unknown;
        }
        // inherent methods of the file's own types win over trait methods
        if let Ty::Named(s) = recv.bare() {
            let methods = self.methods;
            if let Some(sig) = methods.get(&(s.clone(), name.clone())) {
                self.args(e, Some(&sig.params), &m.args);
                return sig.ret.clone();
            }
        }
        let got: Vec<Ty> = m.args.iter().map(|a| self.expr(a)).collect();
        // std types the file cannot add inherent methods to
        match (recv.bare(), name.as_str(), got.as_slice()) {
            (t @ (Ty::Int(_) | Ty::Opt(_) | Ty::Seq(_)), "clone", []) => t.clone(),
            (Ty::Opt(t), "unwrap" | "expect", _) => (**t).clone(),
            (Ty::Opt(t), "unwrap_or", [d]) => {
                let (t, d) = ((**t).clone(), d.clone());
                self.require(e, &t, &d);
                t
            }
            (Ty::Opt(t), "copied" | "cloned", []) => match &**t {
                Ty::Ref(x) => Ty::Opt(x.clone()),
                _ => Ty::Unknown,
            },
            (Ty::Seq(_), "len", []) => Ty::int("usize"),
            _ => Ty::Unknown,
        }
    }

    fn struct_lit(&mut self, s: &syn::ExprStruct) -> Ty {
        let name = (s.qself.is_none() && s.path.segments.len() == 1).then(|| s.path.segments[0].ident.to_string());
        let name = match name.as_deref() {
            Some("Self") => self.self_ty.clone(),
            n => n.map(str::to_string),
        };
        let fields = name.as_ref().and_then(|n| self.items.structs.get(n)).cloned();
        for f in &s.fields {
            let got = self.expr(&f.expr);
            if let (Some(fs), syn::Member::Named(m)) = (&fields, &f.member) {
                if let Some(need) = fs.get(&m.to_string()) {
                    self.require(&f.expr, need, &got);
                }
            }
        }
        if let Some(r) = &s.rest {
            self.expr(r);
        }
        name.filter(|n| fields.is_some() && !n.is_empty()).map_or(Ty::Unknown, Ty::Named)
    }
}

/// The first identifier an expression names that is not a method or a type (to place the diagnostic).
fn first_var(ts: &proc_macro2::TokenStream) -> Option<String> {
    let mut prev_dot = false;
    for tt in ts.clone() {
        match &tt {
            proc_macro2::TokenTree::Ident(i) => {
                let s = i.to_string();
                let lower = s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_');
                if !prev_dot && lower && !["self", "as", "match", "if", "else", "let", "mut", "ref", "move", "return"].contains(&s.as_str()) {
                    return Some(s);
                }
                prev_dot = false;
            }
            proc_macro2::TokenTree::Group(g) => {
                if let Some(v) = first_var(&g.stream()) {
                    return Some(v);
                }
                prev_dot = false;
            }
            proc_macro2::TokenTree::Punct(p) => prev_dot = p.as_char() == '.',
            proc_macro2::TokenTree::Literal(_) => prev_dot = false,
        }
    }
    None
}

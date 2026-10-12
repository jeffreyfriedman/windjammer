//! Read-only view of the Rust that the compiler emitted: function signatures, `let` widths,
//! call argument forms and variable uses. Used to report what the generated program
//! actually does so it can be compared with the IR decision. It decides nothing.

use quote::ToTokens;
use std::collections::HashMap;
use syn::visit::Visit;

/// Mode of a parameter as it appears in the emitted signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmittedMode {
    Owned,
    Ref,
    MutRef,
}

impl EmittedMode {
    pub fn label(self) -> &'static str {
        match self {
            EmittedMode::Owned => "owned",
            EmittedMode::Ref => "&",
            EmittedMode::MutRef => "&mut",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmittedParam {
    pub name: String,
    pub mode: EmittedMode,
    pub ty: String,
}

/// How an argument is written at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgForm {
    Borrow,
    BorrowMut,
    Clone,
    ToOwned,
    /// A bare variable (a move for non-Copy types, a copy for Copy types).
    Bare,
    Other,
}

impl ArgForm {
    pub fn label(self) -> &'static str {
        match self {
            ArgForm::Borrow => "borrow",
            ArgForm::BorrowMut => "borrow-mut",
            ArgForm::Clone => "clone",
            ArgForm::ToOwned => "to_string",
            ArgForm::Bare => "move",
            ArgForm::Other => "expr",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmittedArg {
    pub form: ArgForm,
    pub root: Option<String>,
    /// Position in the traversal order, to decide whether a later use exists.
    pub seq: usize,
    pub loop_depth: usize,
}

#[derive(Debug, Clone)]
pub struct EmittedCall {
    pub callee: String,
    /// True for `recv.method(..)`: the receiver is not among `args`.
    pub is_method: bool,
    pub args: Vec<EmittedArg>,
}

#[derive(Debug, Clone)]
pub struct EmittedLet {
    pub name: String,
    pub width: Option<String>,
    pub mutable: bool,
    pub owned_init: bool,
}

#[derive(Debug, Clone, Default)]
pub struct EmittedFn {
    pub name: String,
    pub params: Vec<EmittedParam>,
    pub lets: Vec<EmittedLet>,
    pub calls: Vec<EmittedCall>,
    /// `x.clone()` receivers by variable name.
    pub clones: HashMap<String, usize>,
    /// Every use of a plain variable: `(seq, name, loop_depth)`.
    pub uses: Vec<(usize, String, usize)>,
    /// Loop depth at which each variable was declared (parameters are 0).
    pub decl_depth: HashMap<String, usize>,
}

/// All functions of an emitted Rust file, keyed by `name` or `Type::name`.
#[derive(Debug, Clone, Default)]
pub struct EmittedFile {
    pub fns: HashMap<String, EmittedFn>,
}

/// Parse emitted Rust. Returns `None` if it is not valid Rust syntax.
pub fn parse_emitted(src: &str) -> Option<EmittedFile> {
    let file = syn::parse_file(src).ok()?;
    let mut out = EmittedFile::default();
    for item in &file.items {
        match item {
            syn::Item::Fn(f) => {
                let name = f.sig.ident.to_string();
                out.fns.insert(name.clone(), analyze_fn(&name, &f.sig, &f.block));
            }
            syn::Item::Impl(imp) => {
                let ty = imp.self_ty.to_token_stream().to_string().replace(' ', "");
                let ty = ty.split('<').next().unwrap_or(&ty).to_string();
                for it in &imp.items {
                    if let syn::ImplItem::Fn(m) = it {
                        let name = format!("{}::{}", ty, m.sig.ident);
                        out.fns.insert(name.clone(), analyze_fn(&name, &m.sig, &m.block));
                    }
                }
            }
            _ => {}
        }
    }
    Some(out)
}

fn mode_of_type(ty: &syn::Type) -> EmittedMode {
    match ty {
        syn::Type::Reference(r) if r.mutability.is_some() => EmittedMode::MutRef,
        syn::Type::Reference(_) => EmittedMode::Ref,
        _ => EmittedMode::Owned,
    }
}

fn analyze_fn(name: &str, sig: &syn::Signature, block: &syn::Block) -> EmittedFn {
    let mut f = EmittedFn {
        name: name.to_string(),
        ..Default::default()
    };
    for input in &sig.inputs {
        match input {
            syn::FnArg::Receiver(r) => {
                let mode = match (&r.reference, r.mutability.is_some()) {
                    (Some(_), true) => EmittedMode::MutRef,
                    (Some(_), false) => EmittedMode::Ref,
                    (None, _) => EmittedMode::Owned,
                };
                f.params.push(EmittedParam {
                    name: "self".into(),
                    mode,
                    ty: "Self".into(),
                });
                f.decl_depth.insert("self".into(), 0);
            }
            syn::FnArg::Typed(t) => {
                let pname = pat_name(&t.pat).unwrap_or_default();
                f.decl_depth.insert(pname.clone(), 0);
                f.params.push(EmittedParam {
                    name: pname,
                    mode: mode_of_type(&t.ty),
                    ty: t.ty.to_token_stream().to_string(),
                });
            }
        }
    }
    let mut v = BodyVisitor {
        f: &mut f,
        seq: 0,
        loop_depth: 0,
    };
    v.visit_block(block);
    f
}

fn pat_name(p: &syn::Pat) -> Option<String> {
    match p {
        syn::Pat::Ident(i) => Some(i.ident.to_string()),
        syn::Pat::Type(t) => pat_name(&t.pat),
        syn::Pat::Reference(r) => pat_name(&r.pat),
        _ => None,
    }
}

struct BodyVisitor<'a> {
    f: &'a mut EmittedFn,
    seq: usize,
    loop_depth: usize,
}

fn root_ident(e: &syn::Expr) -> Option<String> {
    match e {
        syn::Expr::Path(p) if p.path.segments.len() == 1 => Some(p.path.segments[0].ident.to_string()),
        syn::Expr::Field(f) => root_ident(&f.base),
        syn::Expr::Index(i) => root_ident(&i.expr),
        syn::Expr::Paren(p) => root_ident(&p.expr),
        syn::Expr::Reference(r) => root_ident(&r.expr),
        syn::Expr::MethodCall(m) => root_ident(&m.receiver),
        _ => None,
    }
}

fn classify_arg(e: &syn::Expr) -> (ArgForm, Option<String>) {
    match e {
        syn::Expr::Reference(r) => {
            let form = if r.mutability.is_some() { ArgForm::BorrowMut } else { ArgForm::Borrow };
            (form, root_ident(&r.expr))
        }
        syn::Expr::MethodCall(m) if m.method == "clone" && m.args.is_empty() => {
            (ArgForm::Clone, root_ident(&m.receiver))
        }
        syn::Expr::MethodCall(m) if (m.method == "to_string" || m.method == "to_owned") && m.args.is_empty() => {
            (ArgForm::ToOwned, root_ident(&m.receiver))
        }
        syn::Expr::Path(p) if p.path.segments.len() == 1 => {
            (ArgForm::Bare, Some(p.path.segments[0].ident.to_string()))
        }
        syn::Expr::Paren(p) => classify_arg(&p.expr),
        other => (ArgForm::Other, root_ident(other)),
    }
}

impl<'a> BodyVisitor<'a> {
    fn width_of_init(init: &syn::Expr) -> Option<String> {
        match init {
            syn::Expr::Lit(l) => match &l.lit {
                syn::Lit::Int(i) if !i.suffix().is_empty() => Some(i.suffix().to_string()),
                syn::Lit::Float(f) if !f.suffix().is_empty() => Some(f.suffix().to_string()),
                _ => None,
            },
            syn::Expr::Cast(c) => Some(c.ty.to_token_stream().to_string()),
            syn::Expr::Unary(u) => Self::width_of_init(&u.expr),
            syn::Expr::Paren(p) => Self::width_of_init(&p.expr),
            _ => None,
        }
    }

    /// Classify and visit the arguments of a call. `seq` of each argument is the traversal
    /// position *after* its own uses, so only uses with a larger `seq` are "later" uses.
    fn record_call(
        &mut self,
        callee: String,
        is_method: bool,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    ) {
        let mut eargs = Vec::new();
        for a in args {
            let (form, root) = classify_arg(a);
            self.visit_expr(a);
            eargs.push(EmittedArg {
                form,
                root,
                seq: self.seq,
                loop_depth: self.loop_depth,
            });
        }
        self.f.calls.push(EmittedCall {
            callee,
            is_method,
            args: eargs,
        });
    }
}

impl<'ast, 'a> Visit<'ast> for BodyVisitor<'a> {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        // Visit the initializer first: uses inside it precede the new binding.
        if let Some(init) = &l.init {
            self.visit_expr(&init.expr);
        }
        if let Some(name) = pat_name(&l.pat) {
            let mutable = matches!(&l.pat, syn::Pat::Ident(i) if i.mutability.is_some());
            let mut width = match &l.pat {
                syn::Pat::Type(t) => Some(t.ty.to_token_stream().to_string()),
                _ => None,
            };
            if width.is_none() {
                width = l.init.as_ref().and_then(|i| Self::width_of_init(&i.expr));
            }
            let owned_init = l
                .init
                .as_ref()
                .is_some_and(|i| !matches!(&*i.expr, syn::Expr::Reference(_)));
            self.f.decl_depth.insert(name.clone(), self.loop_depth);
            self.f.lets.push(EmittedLet {
                name,
                width,
                mutable,
                owned_init,
            });
        }
    }

    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*c.func {
            let callee = p
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            self.record_call(callee, false, &c.args);
        } else {
            self.visit_expr(&c.func);
            for a in &c.args {
                self.visit_expr(a);
            }
        }
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == "clone" && m.args.is_empty() {
            if let Some(r) = root_ident(&m.receiver) {
                *self.f.clones.entry(r).or_insert(0) += 1;
            }
        }
        self.visit_expr(&m.receiver);
        self.record_call(m.method.to_string(), true, &m.args);
    }

    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.segments.len() == 1 {
            self.seq += 1;
            self.f
                .uses
                .push((self.seq, p.path.segments[0].ident.to_string(), self.loop_depth));
        }
    }

    fn visit_expr_for_loop(&mut self, f: &'ast syn::ExprForLoop) {
        self.visit_expr(&f.expr);
        self.loop_depth += 1;
        if let Some(n) = pat_name(&f.pat) {
            self.f.decl_depth.insert(n, self.loop_depth);
        }
        self.visit_block(&f.body);
        self.loop_depth -= 1;
    }

    fn visit_expr_while(&mut self, w: &'ast syn::ExprWhile) {
        self.loop_depth += 1;
        self.visit_expr(&w.cond);
        self.visit_block(&w.body);
        self.loop_depth -= 1;
    }

    fn visit_expr_loop(&mut self, l: &'ast syn::ExprLoop) {
        self.loop_depth += 1;
        self.visit_block(&l.body);
        self.loop_depth -= 1;
    }

    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        use syn::parse::Parser;
        let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        if let Ok(args) = parser.parse2(m.tokens.clone()) {
            for a in &args {
                self.visit_expr(a);
            }
        }
    }
}

impl EmittedFn {
    /// True when `var` is used again after traversal position `seq`, or the position is inside
    /// a loop that does not contain the variable's declaration (so the next iteration reuses it).
    pub fn used_after(&self, var: &str, seq: usize, loop_depth: usize) -> bool {
        if self.uses.iter().any(|(s, n, _)| n == var && *s > seq) {
            return true;
        }
        let decl = self.decl_depth.get(var).copied().unwrap_or(0);
        loop_depth > decl
    }
}

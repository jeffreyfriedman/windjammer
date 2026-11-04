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
    /// A method call's receiver when it is a bare variable (`n.advance()`, not `n.clone().advance()`).
    pub receiver: Option<EmittedArg>,
}

#[derive(Debug, Clone)]
pub struct EmittedLet {
    pub name: String,
    pub width: Option<String>,
    pub mutable: bool,
    pub owned_init: bool,
    /// The bound value's type when the emitted code spells it: an annotation, a struct literal, `vec![..]`,
    /// `String::from`/`format!`/`.to_string()`.
    pub ty: Option<String>,
    /// `let x = y` with `y` a bare variable: `(y, seq, loop_depth)`, a move of `y` unless it is `Copy`.
    pub moved_from: Option<(String, usize, usize)>,
    /// `let x = y.f` with `y` a bare variable: `(y, f, seq, loop_depth)`, a partial move of `y`.
    pub field_from: Option<(String, String, usize, usize)>,
}

#[derive(Debug, Clone, Default)]
pub struct EmittedFn {
    pub name: String,
    pub params: Vec<EmittedParam>,
    pub lets: Vec<EmittedLet>,
    pub calls: Vec<EmittedCall>,
    /// `x.clone()` receivers by variable name.
    pub clones: HashMap<String, usize>,
    /// Whole reassignments `name = ..`: `(seq, name)`, at the position after the right side.
    pub assigns: Vec<(usize, String)>,
    /// Every use of a plain variable: `(seq, name, loop_depth)`.
    pub uses: Vec<(usize, String, usize)>,
    /// Names bound by a pattern (`let`, `for`, `match` arms, closure parameters): `(seq, name)`. A later use
    /// of the name may be the new binding.
    pub bindings: Vec<(usize, String)>,
    /// Loop depth of each binding in `bindings`, by its position.
    pub binding_depth: HashMap<usize, usize>,
    /// Positions of `return`, `break` and `continue` (a move right before leaving is not used again).
    pub exits: Vec<usize>,
    /// The branch path of every position that is a use or a move: `(branch node, arm)` pairs from the
    /// outermost `if`/`match` in. Two positions on different arms of one node never both run.
    pub paths: HashMap<usize, Vec<(usize, usize)>>,
    /// The `seq`s of those uses that are the base of a field access (`p` in `p.x`), not of the whole value.
    pub field_bases: std::collections::HashSet<usize>,
    /// Loop depth at which each variable was declared (parameters are 0).
    pub decl_depth: HashMap<String, usize>,
}

/// All functions of an emitted Rust file, keyed by `name` or `Type::name`.
#[derive(Debug, Clone, Default)]
pub struct EmittedFile {
    pub fns: HashMap<String, EmittedFn>,
    /// Types the emitted Rust derives `Copy` for (a move of one is a copy).
    pub copy_types: std::collections::HashSet<String>,
    /// Field types of the emitted structs: type -> field -> type.
    pub struct_fields: HashMap<String, HashMap<String, String>>,
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
            syn::Item::Struct(st) => {
                if st.attrs.iter().any(derives_copy) {
                    out.copy_types.insert(st.ident.to_string());
                }
                let fields = st
                    .fields
                    .iter()
                    .filter_map(|f| Some((f.ident.as_ref()?.to_string(), f.ty.to_token_stream().to_string().replace(' ', ""))))
                    .collect();
                out.struct_fields.insert(st.ident.to_string(), fields);
            }
            syn::Item::Enum(syn::ItemEnum { attrs, ident, .. }) if attrs.iter().any(derives_copy) => {
                out.copy_types.insert(ident.to_string());
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

/// `#[derive(.., Copy, ..)]`.
fn derives_copy(a: &syn::Attribute) -> bool {
    a.path().is_ident("derive") && a.meta.to_token_stream().to_string().split(|c: char| !c.is_alphanumeric()).any(|w| w == "Copy")
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
        path: Vec::new(),
        nodes: 0,
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
    /// The current branch path (`EmittedFn::paths`) and the next branch node id.
    path: Vec<(usize, usize)>,
    nodes: usize,
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

/// The type of a `let` initializer when the emitted code spells it.
fn type_of_init(e: &syn::Expr) -> Option<String> {
    match e {
        syn::Expr::Struct(s) => s.path.segments.last().map(|x| x.ident.to_string()),
        syn::Expr::Macro(m) if m.mac.path.is_ident("vec") => Some("Vec".into()),
        syn::Expr::Macro(m) if m.mac.path.is_ident("format") => Some("String".into()),
        syn::Expr::MethodCall(m) if m.method == "to_string" || m.method == "to_owned" && matches!(&*m.receiver, syn::Expr::Lit(_)) => Some("String".into()),
        syn::Expr::Call(c) => match &*c.func {
            syn::Expr::Path(p) if p.path.segments.len() == 2 && p.path.segments[0].ident == "String" => Some("String".into()),
            syn::Expr::Path(p) if p.path.segments.len() == 2 && ["Vec", "HashMap", "HashSet", "VecDeque", "BTreeMap"].iter().any(|t| p.path.segments[0].ident == t) => {
                Some(p.path.segments[0].ident.to_string())
            }
            _ => None,
        },
        syn::Expr::Paren(p) => type_of_init(&p.expr),
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
        self.record_call_on(callee, is_method, args, None);
    }

    fn record_call_on(
        &mut self,
        callee: String,
        is_method: bool,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
        receiver: Option<EmittedArg>,
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
            receiver,
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
            let ty = match &l.pat {
                syn::Pat::Type(t) => Some(t.ty.to_token_stream().to_string().replace(' ', "")),
                _ => l.init.as_ref().and_then(|i| type_of_init(&i.expr)),
            };
            let (seq, depth) = (self.seq, self.loop_depth);
            let moved_from = match l.init.as_ref().map(|i| &*i.expr) {
                Some(syn::Expr::Path(p)) if p.path.segments.len() == 1 => Some((p.path.segments[0].ident.to_string(), seq, depth)),
                _ => None,
            };
            let field_from = match l.init.as_ref().map(|i| &*i.expr) {
                Some(syn::Expr::Field(f)) => match (&*f.base, &f.member) {
                    (syn::Expr::Path(p), syn::Member::Named(m)) if p.path.segments.len() == 1 => {
                        Some((p.path.segments[0].ident.to_string(), m.to_string(), seq, depth))
                    }
                    _ => None,
                },
                _ => None,
            };
            self.f.decl_depth.insert(name.clone(), self.loop_depth);
            self.seq += 1;
            self.f.bindings.push((self.seq, name.clone()));
            self.f.binding_depth.insert(self.seq, self.loop_depth);
            self.f.lets.push(EmittedLet {
                name,
                width,
                mutable,
                owned_init,
                ty,
                moved_from,
                field_from,
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
        let receiver = match &*m.receiver {
            syn::Expr::Path(p) if p.path.segments.len() == 1 => Some(EmittedArg {
                form: ArgForm::Bare,
                root: Some(p.path.segments[0].ident.to_string()),
                seq: self.seq,
                loop_depth: self.loop_depth,
            }),
            _ => None,
        };
        self.record_call_on(m.method.to_string(), true, &m.args, receiver);
    }

    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        // the right side runs first; the target is (re)initialized, not read
        self.visit_expr(&a.right);
        match &*a.left {
            syn::Expr::Path(p) if p.path.segments.len() == 1 => {
                self.seq += 1;
                self.f.assigns.push((self.seq, p.path.segments[0].ident.to_string()));
            }
            left => self.visit_expr(left),
        }
    }

    fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
        self.seq += 1;
        self.f.bindings.push((self.seq, p.ident.to_string()));
        self.f.binding_depth.insert(self.seq, self.loop_depth);
        syn::visit::visit_pat_ident(self, p);
    }

    fn visit_expr_return(&mut self, r: &'ast syn::ExprReturn) {
        syn::visit::visit_expr_return(self, r);
        self.f.exits.push(self.seq);
    }

    fn visit_expr_break(&mut self, b: &'ast syn::ExprBreak) {
        syn::visit::visit_expr_break(self, b);
        self.f.exits.push(self.seq);
    }

    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        // a closure body runs when the closure is called, not once per iteration of the loop around it
        let depth = std::mem::replace(&mut self.loop_depth, 0);
        syn::visit::visit_expr_closure(self, c);
        self.loop_depth = depth;
    }

    fn visit_expr_continue(&mut self, c: &'ast syn::ExprContinue) {
        syn::visit::visit_expr_continue(self, c);
        self.f.exits.push(self.seq);
    }

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        self.visit_expr(&i.cond);
        let node = self.nodes;
        self.nodes += 1;
        self.path.push((node, 0));
        self.visit_block(&i.then_branch);
        if let Some((_, e)) = &i.else_branch {
            *self.path.last_mut().expect("pushed") = (node, 1);
            self.visit_expr(e);
        }
        self.path.pop();
    }

    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        self.visit_expr(&m.expr);
        let node = self.nodes;
        self.nodes += 1;
        for (k, arm) in m.arms.iter().enumerate() {
            self.path.push((node, k));
            self.visit_arm(arm);
            self.path.pop();
        }
    }

    fn visit_expr_field(&mut self, f: &'ast syn::ExprField) {
        self.visit_expr(&f.base);
        if matches!(&*f.base, syn::Expr::Path(p) if p.path.segments.len() == 1) {
            self.f.field_bases.insert(self.seq);
        }
    }

    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.segments.len() == 1 {
            self.seq += 1;
            self.f.paths.insert(self.seq, self.path.clone());
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
        self.visit_pat(&f.pat);
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

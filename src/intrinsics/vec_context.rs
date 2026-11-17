//! Declared-type context for `vec![..]` -> `[..]` (`wj fix`, ADR-0014).
//!
//! A bare list literal `[a, b]` is a `Vec` unless its place is a fixed array (ADR-0014). The textual
//! rewriter only knows the syntactic positions (`let x = `, `field: `, `for .. in`). This module reads
//! the parsed program and decides every other position from DECLARATIONS: a call argument whose
//! formal is a `Vec`, a function's declared return type (tail and `return`, through `Ok`/`Some`,
//! `if`, `match` and blocks), a struct field, a typed `let` and an assignment to a place whose declared
//! type is known. Receiver and place types come from declared parameter, field, `let` and return types
//! only; anything not declared (closures, generics, unknown receivers) is `Unknown` and the macro stays.
use std::collections::HashMap;

use crate::parser::ast::core::*;
use crate::parser::ast::operators::UnaryOp;
use crate::parser::ast::types::Type;

/// What the declared context says about a `vec![..]` occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The place is declared `Vec<T>` (or `&Vec<T>` / `&[T]`): `[..]` lowers to the same `Vec`.
    List,
    /// The place is a declared fixed array `[T; N]`: a bare literal would stay an array.
    Array,
    /// No declared type reaches this position.
    Unknown,
}

/// Verdicts keyed by the 1-based (line, column) of the token after the macro (where the parser positions the node).
pub type Verdicts = HashMap<(usize, usize), Verdict>;

#[derive(Debug, Clone)]
struct Sig {
    params: Vec<Type>, // including `self` when `has_self`
    has_self: bool,
    ret: Option<Type>,
}

/// Declarations of one project: structs, free functions, methods (inherent, trait impl and trait).
#[derive(Default)]
pub struct DeclIndex {
    fields: HashMap<String, Vec<HashMap<String, Type>>>,
    fns: HashMap<String, Vec<Sig>>,
    methods: HashMap<(String, String), Vec<Sig>>,
}

fn leaf(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

fn sig_of(f: &FunctionDecl<'_>) -> Sig {
    Sig {
        params: f.parameters.iter().map(|p| p.type_.clone()).collect(),
        has_self: f.parameters.first().is_some_and(|p| p.name == "self"),
        ret: f.return_type.clone(),
    }
}

impl DeclIndex {
    pub fn add_program(&mut self, program: &Program<'_>) {
        self.add_items(&program.items);
    }

    fn add_items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => self.fns.entry(decl.name.clone()).or_default().push(sig_of(decl)),
                Item::Struct { decl, .. } => {
                    let map = decl.fields.iter().map(|f| (f.name.clone(), f.field_type.clone())).collect();
                    self.fields.entry(decl.name.clone()).or_default().push(map);
                }
                Item::Impl { block, .. } => {
                    for f in &block.functions {
                        self.methods
                            .entry((leaf(&block.type_name).to_string(), f.name.clone()))
                            .or_default()
                            .push(sig_of(f));
                    }
                }
                Item::Trait { decl, .. } => {
                    for m in &decl.methods {
                        let sig = Sig {
                            params: m.parameters.iter().map(|p| p.type_.clone()).collect(),
                            has_self: m.parameters.first().is_some_and(|p| p.name == "self"),
                            ret: m.return_type.clone(),
                        };
                        self.methods.entry((decl.name.clone(), m.name.clone())).or_default().push(sig);
                    }
                }
                Item::Mod { items, .. } => self.add_items(items),
                _ => {}
            }
        }
    }

    /// The declared type of `field` on struct `ty`; `None` when undeclared or declared differently twice.
    fn field_type(&self, ty: &str, field: &str) -> Option<Type> {
        let decls = self.fields.get(leaf(ty))?;
        let mut found: Option<&Type> = None;
        for d in decls {
            let t = d.get(field)?;
            if found.is_some_and(|f| f != t) {
                return None;
            }
            found = Some(t);
        }
        found.cloned()
    }

    fn pick<'a>(sigs: &'a [Sig], arity: usize) -> Option<&'a Sig> {
        Self::pick_where(sigs.iter(), arity)
    }

    fn pick_where<'a>(sigs: impl Iterator<Item = &'a Sig>, arity: usize) -> Option<&'a Sig> {
        let mut it = sigs.filter(|s| s.params.len() == arity);
        let first = it.next()?;
        // every same-arity declaration must agree on parameters and result
        if it.all(|s| s.params == first.params && s.ret == first.ret) {
            Some(first)
        } else {
            None
        }
    }

    fn method_sig(&self, ty: &str, name: &str, args: usize) -> Option<&Sig> {
        let sigs = self.methods.get(&(leaf(ty).to_string(), name.to_string()))?;
        Self::pick_where(sigs.iter().filter(|s| s.has_self), args + 1)
    }
}

/// `Vec<T>` / `&Vec<T>` / `&[T]` (the declared list types a list literal can take).
fn is_list_type(t: &Type) -> bool {
    match t {
        Type::Vec(_) => true,
        Type::Parameterized(name, _) => leaf(name) == "Vec",
        Type::Reference(i) | Type::MutableReference(i) => is_list_type(i),
        _ => false,
    }
}

/// The type name a receiver expression's declared type resolves to (references looked through).
fn head_name(t: &Type) -> Option<String> {
    match t {
        Type::Custom(n) | Type::Parameterized(n, _) => Some(leaf(n).to_string()),
        Type::Reference(i) | Type::MutableReference(i) => head_name(i),
        _ => None,
    }
}

type Env = HashMap<String, Type>;

struct Walker<'i> {
    index: &'i DeclIndex,
    out: Verdicts,
    /// Set by the callers that give a value its result type directly (tail, `return`, typed `let`,
    /// `match` arm): only there does the compiler lower `Ok([..])` / `Some([..])` to a `Vec` (`Err([..])` is not lowered).
    allow_wrap: bool,
}

fn wrap_inner(expected: Option<&Type>, ctor: &str) -> Option<Type> {
    match (expected?, ctor) {
        (Type::Option(i), "Some") => Some((**i).clone()),
        (Type::Result(ok, _), "Ok") => Some((**ok).clone()),
        _ => None,
    }
}

impl<'i> Walker<'i> {
    /// Declared type of `e`, from declarations only.
    fn type_of(&self, e: &Expression<'_>, env: &Env) -> Option<Type> {
        match e {
            Expression::Identifier { name, .. } => env.get(name).cloned(),
            Expression::FieldAccess { object, field, .. } => {
                let ty = head_name(&self.type_of(object, env)?)?;
                self.index.field_type(&ty, field)
            }
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Unary { op: UnaryOp::Ref | UnaryOp::MutRef | UnaryOp::Deref, operand, .. } => {
                self.type_of(operand, env)
            }
            Expression::TryOp { expr, .. } => match self.type_of(expr, env)? {
                Type::Result(ok, _) => Some(*ok),
                Type::Option(i) => Some(*i),
                _ => None,
            },
            Expression::Call { function: Expression::Identifier { name, .. }, arguments, .. } => {
                let sig = self.call_sig(name, arguments.len())?;
                let ret = sig.0.ret.clone()?;
                Some(self_subst(ret, sig.1.as_deref()))
            }
            Expression::MethodCall { object, method, arguments, .. } => {
                let recv = self.type_of(object, env)?;
                let ty = head_name(&recv)?;
                let sig = self.index.method_sig(&ty, method, arguments.len())?;
                Some(self_subst(sig.ret.clone()?, Some(&ty)))
            }
            _ => None,
        }
    }

    /// Signature for a path call `f(..)` / `Type::f(..)` plus the `Type` for `Self`.
    fn call_sig(&self, name: &str, args: usize) -> Option<(&Sig, Option<String>)> {
        if let Some((ty, f)) = name.rsplit_once("::") {
            let ty = leaf(ty);
            if let Some(s) = self.index.methods.get(&(ty.to_string(), f.to_string())) {
                return DeclIndex::pick_call(s, args).map(|s| (s, Some(ty.to_string())));
            }
            // `module::function(..)`: the declared free function of that name
            return DeclIndex::pick_call(self.index.fns.get(f)?, args).map(|s| (s, None));
        }
        DeclIndex::pick_call(self.index.fns.get(name)?, args).map(|s| (s, None))
    }

    fn mark(&mut self, e: &Expression<'_>, expected: Option<&Type>) {
        let Some(loc) = e.location() else { return };
        let verdict = match expected {
            Some(Type::Array(..)) => Verdict::Array,
            Some(t) if is_list_type(t) => Verdict::List,
            _ => Verdict::Unknown,
        };
        self.out.insert((loc.line, loc.column), verdict);
    }

    fn args(&mut self, formals: Option<Vec<Type>>, args: &[(Option<String>, &Expression<'_>)], env: &Env) {
        for (i, (_, a)) in args.iter().enumerate() {
            let want = formals.as_ref().and_then(|f| f.get(i)).cloned();
            self.expr(a, want.as_ref(), env);
        }
    }

    /// `expr` for a value whose declared result type is `expected` (see `allow_wrap`).
    fn typed_value(&mut self, e: &Expression<'_>, expected: Option<&Type>, env: &Env) {
        self.allow_wrap = true;
        self.expr(e, expected, env);
    }

    fn expr(&mut self, e: &Expression<'_>, expected: Option<&Type>, env: &Env) {
        let wrap_ok = std::mem::take(&mut self.allow_wrap);
        match e {
            Expression::MacroInvocation { name, args, .. } => {
                if name == "vec" {
                    self.mark(e, expected);
                }
                let elem = match expected {
                    Some(Type::Vec(i)) | Some(Type::Array(i, _)) => Some((**i).clone()),
                    _ => None,
                };
                for a in args {
                    self.expr(a, elem.as_ref(), env);
                }
            }
            Expression::Array { elements, .. } => {
                let elem = match expected {
                    Some(Type::Vec(i)) | Some(Type::Array(i, _)) => Some((**i).clone()),
                    _ => None,
                };
                for a in elements {
                    self.expr(a, elem.as_ref(), env);
                }
            }
            Expression::Call { function, arguments, .. } => {
                if let Expression::Identifier { name, .. } = function {
                    if matches!(name.as_str(), "Ok" | "Some" | "Err") && arguments.len() == 1 {
                        let inner = if wrap_ok { wrap_inner(expected, name) } else { None };
                        self.expr(arguments[0].1, inner.as_ref(), env);
                        return;
                    }
                    let formals = self.call_sig(name, arguments.len()).map(|(s, _)| s.params.clone());
                    self.args(formals, arguments, env);
                } else {
                    self.expr(function, None, env);
                    self.args(None, arguments, env);
                }
            }
            Expression::MethodCall { object, method, arguments, .. } => {
                self.expr(object, None, env);
                let recv = self.type_of(object, env);
                let mut formals = None;
                if let Some(ty) = recv.as_ref().and_then(head_name) {
                    if let Some(s) = self.index.method_sig(&ty, method, arguments.len()) {
                        formals = Some(s.params.iter().skip(1).cloned().collect());
                    }
                }
                self.args(formals, arguments, env);
            }
            Expression::StructLiteral { name, fields, .. } => {
                for (f, v) in fields {
                    let want = self.index.field_type(name, f);
                    self.expr(v, want.as_ref(), env);
                }
            }
            // tuple slots are not lowered from the declared tuple type: elements see no declared type
            Expression::Tuple { elements, .. } => {
                for a in elements {
                    self.expr(a, None, env);
                }
            }
            Expression::Block { statements, .. } => self.stmts(statements, expected, None, &mut env.clone()),
            Expression::Unary { op, operand, .. } => {
                let inner = match (op, expected) {
                    (UnaryOp::Ref | UnaryOp::MutRef, Some(Type::Reference(i) | Type::MutableReference(i))) => {
                        Some((**i).clone())
                    }
                    _ => None,
                };
                self.expr(operand, inner.as_ref(), env);
            }
            // closures have no declared result type; their bodies see only their own declarations
            Expression::Closure { body, .. } => self.expr(body, None, env),
            Expression::Binary { left, right, .. } => {
                self.expr(left, None, env);
                self.expr(right, None, env);
            }
            Expression::FieldAccess { object, .. } | Expression::Cast { expr: object, .. } => {
                self.expr(object, None, env)
            }
            Expression::Index { object, index, .. } => {
                self.expr(object, None, env);
                self.expr(index, None, env);
            }
            Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. }
            | Expression::Scope { body: expr, .. } => self.expr(expr, None, env),
            Expression::Select { arms, .. } => {
                for a in arms {
                    if let Some(s) = a.source {
                        self.expr(s, None, env);
                    }
                    self.expr(a.body, None, env);
                }
            }
            Expression::MapLiteral { pairs, .. } => {
                for (k, v) in pairs {
                    self.expr(k, None, env);
                    self.expr(v, None, env);
                }
            }
            Expression::Range { start, end, .. } => {
                self.expr(start, None, env);
                self.expr(end, None, env);
            }
            _ => {}
        }
    }

    /// `tail`: the declared type of the block's value; `ret`: the enclosing function's declared result.
    fn stmts(&mut self, list: &[&Statement<'_>], tail: Option<&Type>, ret: Option<&Type>, env: &mut Env) {
        for (i, st) in list.iter().enumerate() {
            let last = i + 1 == list.len();
            let want = if last { tail } else { None };
            self.stmt(st, want, ret, env);
        }
    }

    fn stmt(&mut self, st: &Statement<'_>, want: Option<&Type>, ret: Option<&Type>, env: &mut Env) {
        match st {
            Statement::Let { pattern, type_, value, else_block, .. } => {
                self.typed_value(value, type_.as_ref(), env);
                if let Some(b) = else_block {
                    self.stmts(b, None, ret, &mut env.clone());
                }
                let bound = type_.clone().or_else(|| self.type_of(value, env));
                if let Pattern::Identifier(n) | Pattern::MutBinding(n) = pattern {
                    match bound {
                        Some(t) => env.insert(n.clone(), t),
                        None => env.remove(n),
                    };
                }
            }
            Statement::Const { type_, value, .. } | Statement::Static { type_, value, .. } => {
                self.expr(value, Some(type_), env)
            }
            Statement::Assignment { target, value, compound_op, .. } => {
                self.expr(target, None, env);
                let place = if compound_op.is_none() { self.type_of(target, env) } else { None };
                self.expr(value, place.as_ref(), env);
            }
            Statement::Return { value: Some(v), .. } => self.typed_value(v, ret, env),
            Statement::Expression { expr, .. } => self.typed_value(expr, want, env),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition, None, env);
                self.stmts(then_block, want, ret, &mut env.clone());
                if let Some(b) = else_block {
                    self.stmts(b, want, ret, &mut env.clone());
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr(value, None, env);
                for a in arms {
                    if let Some(g) = a.guard {
                        self.expr(g, None, env);
                    }
                    self.typed_value(a.body, want, env);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.expr(iterable, None, env);
                self.stmts(body, None, ret, &mut env.clone());
            }
            Statement::While { condition, body, .. } => {
                self.expr(condition, None, env);
                self.stmts(body, None, ret, &mut env.clone());
            }
            Statement::Loop { body, .. } => {
                self.stmts(body, None, ret, &mut env.clone())
            }
            Statement::Defer { statement, .. } => self.stmt(statement, None, ret, env),
            Statement::Break { value: Some(v), .. } => self.expr(v, None, env),
            _ => {}
        }
    }

    fn function(&mut self, f: &FunctionDecl<'_>, self_ty: Option<&str>) {
        let mut env = Env::new();
        for p in &f.parameters {
            if p.name == "self" {
                if let Some(t) = self_ty {
                    env.insert("self".to_string(), Type::Custom(t.to_string()));
                }
            } else {
                env.insert(p.name.clone(), p.type_.clone());
            }
        }
        let ret = f.return_type.clone().map(|r| self_subst(r, self_ty));
        self.stmts(&f.body, ret.as_ref(), ret.as_ref(), &mut env);
    }

    fn items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => self.function(decl, None),
                Item::Impl { block, .. } => {
                    for f in &block.functions {
                        self.function(f, Some(leaf(&block.type_name)));
                    }
                }
                Item::Trait { decl, .. } => {
                    for m in &decl.methods {
                        let Some(body) = &m.body else { continue };
                        let ret = m.return_type.clone();
                        let mut env = Env::new();
                        for p in m.parameters.iter().filter(|p| p.name != "self") {
                            env.insert(p.name.clone(), p.type_.clone());
                        }
                        self.stmts(body, ret.as_ref(), ret.as_ref(), &mut env);
                    }
                }
                Item::Const { type_, value, .. } | Item::Static { type_, value, .. } => {
                    self.expr(value, Some(type_), &Env::new())
                }
                Item::Mod { items, .. } => self.items(items),
                _ => {}
            }
        }
    }
}

/// `Self` in a declared type means the impl's type.
fn self_subst(t: Type, self_ty: Option<&str>) -> Type {
    match (&t, self_ty) {
        (Type::Custom(n), Some(s)) if n == "Self" => Type::Custom(s.to_string()),
        _ => t,
    }
}

impl DeclIndex {
    /// Pick a signature for a path call `f(a, b)` / `Type::f(a, b)` (arity counts every argument).
    fn pick_call(sigs: &[Sig], args: usize) -> Option<&Sig> {
        Self::pick(sigs, args)
    }
}

/// Verdicts for every `vec![..]` in `program`, decided from `index`.
pub fn verdicts(program: &Program<'_>, index: &DeclIndex) -> Verdicts {
    let mut w = Walker { index, out: Verdicts::new(), allow_wrap: false };
    w.items(&program.items);
    w.out
}

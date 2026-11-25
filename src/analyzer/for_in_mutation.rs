//! A `for` loop must not change the collection it iterates (WJ0202).
//!
//! `for p in v { v.push(..) }` has no single meaning. The interpreter walks a snapshot taken when the loop starts,
//! so the pushes would be invisible to the loop; the compiled program holds a borrow of `v` for the whole loop, and
//! rustc rejects every change to `v` inside it. The author has to say which one was meant, so `wj check` refuses the
//! loop in `.wj` terms instead of leaving the compiled build to fail late.
//!
//! Decided by structure, from declared facts only:
//!
//! * the iterated expression is a path (`v`, `self.items`, `a.b`) whose declared type is a collection that the std
//!   signature table knows (`Vec`, `HashMap`, ...), optionally behind `&` / `&mut`, or a chain of calls on it that
//!   the table declares to borrow it and return an `Iterator` (`v.iter()`, `v.iter_mut().enumerate()`);
//! * the body (closures, nested loops and blocks included) contains, on that same path, a method call whose std
//!   signature takes `&mut self`, an assignment to the path or to an element of it (`v = ..`, `v[i] = ..`,
//!   `v[i].f += ..`), or an explicit `&mut v`.
//!
//! Not reported: a by-value `for x in self.items` (the generated loop may iterate a copy), `0..v.len()` and `v.clone()` (they iterate something else), reads, changes to other collections or
//! to the loop variable, a path whose type is not declared (fail closed), and anything after a `let`, loop or arm
//! pattern that rebinds the root name (a different binding).
//!
//! Run by `wj check` (spans from the parser's span log, no fix: the intent is ambiguous), `wj build` and the
//! interpreter loader (ADR-0030: `wj run` keeps `wj check`'s refusals).

use super::param_scope::binds;
use super::SignatureRegistry;
use super::stdlib_method_traits::{lookup_method_signature, method_call_mutates_receiver};
use super::task_structure::children;
use crate::parser::ast::core::{Expression, Statement};
use crate::parser::*;
use std::collections::HashMap;

/// Diagnostic code of a loop that changes what it iterates.
pub const CODE: &str = "WJ0202";

/// How the body changes the iterated path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A std method that takes `&mut self` (`push`, `clear`, `sort`, ...).
    Method(String),
    /// `v[i] = ..`, `v[i].f += ..`.
    Element,
    /// `v = ..` (or `self.items = ..`).
    Whole,
    /// An explicit `&mut v`.
    Borrow,
}

/// One loop that changes what it iterates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopMutation {
    /// `v` or `self.items`.
    pub path: String,
    pub change: Change,
    /// The changing expression or statement (needs the parser's span log).
    pub change_span: Option<NodeSpan>,
    /// The iterated expression of the `for` header.
    pub header_span: Option<NodeSpan>,
    /// 1-based position of the change when there is no span log (0 without a location).
    pub line: usize,
    pub column: usize,
}

impl LoopMutation {
    pub fn message(&self) -> String {
        let p = &self.path;
        match &self.change {
            Change::Method(m) => format!("`{p}.{m}(..)` changes `{p}` while the loop over `{p}` is running"),
            Change::Element => format!("this assignment changes an element of `{p}` while the loop over `{p}` is running"),
            Change::Whole => format!("this assignment replaces `{p}` while the loop over `{p}` is running"),
            Change::Borrow => format!("`&mut {p}` lets the loop body change `{p}` while the loop over `{p}` is running"),
        }
    }

    pub fn label(&self) -> String {
        format!("changes `{}` inside its own loop", self.path)
    }

    pub fn header_label(&self) -> String {
        format!("`{}` is iterated here", self.path)
    }

    pub fn note(&self) -> String {
        format!(
            "`wj run` would walk the vector as it was when the loop started and not see the change; a compiled build keeps `{}` borrowed for the whole loop and rustc rejects the change",
            self.path
        )
    }

    pub fn help(&self) -> Vec<String> {
        let p = &self.path;
        vec![
            format!("collect the changes while looping (a new list, or the indexes to change) and apply them to `{p}` after the loop"),
            format!("loop over a fixed range with `for i in 0..{p}.len()` and use `{p}[i]` when the body has to change `{p}` itself, or write `for item in {p}.clone()` when the loop should see a snapshot"),
        ]
    }

    pub fn render(&self) -> String {
        format!("error[{CODE}]: {}\n  --> line {}:{}\n  = note: {}\n  = help: {}", self.message(), self.line, self.column, self.note(), self.help().join("; "))
    }
}

/// Every loop of `program` that changes the collection it iterates, in source order. `spans` is the parser's span
/// log (`SpanSnapshot::default()` when there is none).
pub fn check(program: &Program<'_>, spans: &SpanSnapshot) -> Vec<LoopMutation> {
    let mut cx = Cx { structs: HashMap::new(), free_fns: HashMap::new(), methods: HashMap::new(), spans, env: Vec::new(), out: Vec::new() };
    cx.collect(&program.items);
    cx.items(&program.items);
    // an inner loop over the same path owns the change: keep the last (innermost) report per change
    let mut keep: Vec<(usize, LoopMutation)> = Vec::new();
    for (key, m) in std::mem::take(&mut cx.out) {
        match keep.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = m,
            None => keep.push((key, m)),
        }
    }
    keep.into_iter().map(|(_, m)| m).collect()
}

/// `Err` with the first finding, rendered (for the compiler and the interpreter loader).
pub fn gate(program: &Program<'_>) -> Result<(), String> {
    match check(program, &SpanSnapshot::default()).first() {
        Some(f) => Err(f.render()),
        None => Ok(()),
    }
}

struct Cx<'a, 'ast> {
    structs: HashMap<String, &'a StructDecl<'ast>>,
    free_fns: HashMap<String, Type>,
    /// `(type, method)` -> declared return type, `Self` already replaced.
    methods: HashMap<(String, String), Type>,
    spans: &'a SpanSnapshot,
    /// Bindings in scope, innermost last; `None` = type not declared.
    env: Vec<(String, Option<Type>)>,
    out: Vec<(usize, LoopMutation)>,
}

fn pos(e: &Expression<'_>) -> (usize, usize) {
    e.location().as_ref().map(|l| (l.line, l.column)).unwrap_or((0, 0))
}

fn strip_refs(t: &Type) -> &Type {
    match t {
        Type::Reference(i) | Type::MutableReference(i) => strip_refs(i),
        _ => t,
    }
}

fn type_base(t: &Type) -> Option<&str> {
    match strip_refs(t) {
        Type::Vec(_) => Some("Vec"),
        Type::Parameterized(n, _) | Type::Custom(n) => Some(n.as_str()),
        _ => None,
    }
}

/// The std signature table declares `iter` for this type: it is a collection (Vec, HashMap, ...).
fn is_std_collection(t: &Type) -> bool {
    type_base(t).is_some_and(|b| lookup_method_signature("iter", Some(b), SignatureRegistry::stdlib()).is_some())
}

/// `v` / `self.items` as path segments.
fn path_of(e: &Expression<'_>) -> Option<Vec<String>> {
    match e {
        Expression::Identifier { name, .. } => Some(vec![name.clone()]),
        Expression::FieldAccess { object, field, .. } => {
            let mut p = path_of(object)?;
            p.push(field.clone());
            Some(p)
        }
        _ => None,
    }
}

fn pattern_names(p: &Pattern<'_>, out: &mut Vec<String>) {
    match p {
        Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) | Pattern::RefMut(n) => out.push(n.clone()),
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|p| pattern_names(p, out)),
        Pattern::Reference(i) => pattern_names(i, out),
        Pattern::EnumVariant(_, b) => match b {
            EnumPatternBinding::Single(n) => out.push(n.clone()),
            EnumPatternBinding::Tuple(ps) => ps.iter().for_each(|p| pattern_names(p, out)),
            EnumPatternBinding::Struct(fs, _) => fs.iter().for_each(|(_, p)| pattern_names(p, out)),
            EnumPatternBinding::None | EnumPatternBinding::Wildcard => {}
        },
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
    }
}

impl<'a, 'ast> Cx<'a, 'ast> {
    fn collect(&mut self, items: &'a [Item<'ast>]) {
        for item in items {
            match item {
                Item::Struct { decl, .. } => {
                    self.structs.insert(decl.name.clone(), decl);
                }
                Item::Function { decl, .. } => {
                    if let Some(r) = &decl.return_type {
                        self.free_fns.insert(decl.name.clone(), r.clone());
                    }
                }
                Item::Impl { block, .. } => {
                    for f in &block.functions {
                        if let Some(r) = &f.return_type {
                            let r = match r {
                                Type::Custom(s) if s == "Self" => Type::Custom(block.type_name.clone()),
                                other => other.clone(),
                            };
                            self.methods.insert((block.type_name.clone(), f.name.clone()), r);
                        }
                    }
                }
                Item::Mod { items, .. } => self.collect(items),
                _ => {}
            }
        }
    }

    fn items(&mut self, items: &[Item<'ast>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => self.func(decl, None),
                Item::Impl { block, .. } => block.functions.iter().for_each(|f| self.func(f, Some(&block.type_name))),
                Item::Mod { items, .. } => self.items(items),
                _ => {}
            }
        }
    }

    fn func(&mut self, f: &FunctionDecl<'ast>, self_ty: Option<&str>) {
        self.env.clear();
        for p in &f.parameters {
            self.env.push((p.name.clone(), Some(p.type_.clone())));
        }
        if let Some(t) = self_ty {
            self.env.push(("self".into(), Some(Type::Custom(t.to_string()))));
        }
        self.block(&f.body);
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        self.env.iter().rev().find(|(n, _)| n == name).and_then(|(_, t)| t.as_ref())
    }

    fn type_of_path(&self, path: &[String]) -> Option<Type> {
        let mut ty = self.lookup(&path[0])?.clone();
        for field in &path[1..] {
            let decl = self.structs.get(type_base(&ty)?)?;
            ty = decl.fields.iter().find(|f| &f.name == field)?.field_type.clone();
        }
        Some(ty)
    }

    /// The declared type of an initializer, when it states one.
    fn infer(&self, e: &Expression<'_>) -> Option<Type> {
        match e {
            // `[a, b]` is a list (a `Vec`); `vec![..]` is its deprecated spelling
            Expression::Array { .. } => Some(Type::Vec(Box::new(Type::Infer))),
            Expression::MacroInvocation { name, .. } if name == "vec" => Some(Type::Vec(Box::new(Type::Infer))),
            Expression::StructLiteral { name, .. } => Some(Type::Custom(name.clone())),
            Expression::Call { function: Expression::Identifier { name, .. }, .. } => match name.rsplit_once("::") {
                None => self.free_fns.get(name).cloned(),
                Some((ty, f)) => {
                    let ret = self
                        .methods
                        .get(&(ty.to_string(), f.to_string()))
                        .cloned()
                        .or_else(|| lookup_method_signature(f, Some(ty), SignatureRegistry::stdlib()).and_then(|s| s.return_type.clone()))?;
                    Some(match ret {
                        Type::Custom(s) if s == "Self" => Type::Custom(ty.to_string()),
                        other => other,
                    })
                }
            },
            _ => None,
        }
    }

    fn bind(&mut self, p: &Pattern<'_>, ty: Option<Type>) {
        match p {
            Pattern::Identifier(n) | Pattern::MutBinding(n) => self.env.push((n.clone(), ty)),
            other => {
                let mut names = Vec::new();
                pattern_names(other, &mut names);
                names.into_iter().for_each(|n| self.env.push((n, None)));
            }
        }
    }

    fn block(&mut self, stmts: &[&Statement<'ast>]) {
        let mark = self.env.len();
        for s in stmts {
            self.stmt(s);
        }
        self.env.truncate(mark);
    }

    fn stmt(&mut self, s: &Statement<'ast>) {
        match s {
            Statement::Let { pattern, type_, value, else_block, .. } => {
                self.expr(value);
                if let Some(b) = else_block {
                    self.block(b);
                }
                let ty = type_.clone().or_else(|| self.infer(value));
                self.bind(pattern, ty);
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => self.expr(value),
            Statement::Assignment { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            Statement::Return { value, .. } | Statement::Break { value, .. } => value.iter().for_each(|v| self.expr(v)),
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr(value);
                for a in arms {
                    let mark = self.env.len();
                    self.bind(&a.pattern, None);
                    a.guard.iter().for_each(|g| self.expr(g));
                    self.expr(a.body);
                    self.env.truncate(mark);
                }
            }
            Statement::For { pattern, iterable, body, .. } => {
                self.expr(iterable);
                self.check_loop(iterable, body);
                let mark = self.env.len();
                self.bind(pattern, None);
                self.block(body);
                self.env.truncate(mark);
            }
            Statement::While { condition, body, .. } => {
                self.expr(condition);
                self.block(body);
            }
            Statement::Loop { body, .. } => self.block(body),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Continue { .. } | Statement::Use { .. } => {}
        }
    }

    fn expr(&mut self, e: &Expression<'ast>) {
        match e {
            Expression::Block { statements, .. } => self.block(statements),
            Expression::Closure { parameters, body, .. } => {
                let mark = self.env.len();
                parameters.iter().for_each(|p| self.env.push((p.clone(), None)));
                self.expr(body);
                self.env.truncate(mark);
            }
            _ => children(e).into_iter().for_each(|c| self.expr(c)),
        }
    }

    /// The path of the collection that `iterable` walks, when it borrows or moves it.
    fn iterated_path(&self, iterable: &Expression<'_>) -> Option<Vec<String>> {
        match iterable {
            Expression::Unary { op: UnaryOp::Ref | UnaryOp::MutRef, operand, .. } => path_of(operand),
            Expression::MethodCall { object, method, .. } => {
                let stdlib = SignatureRegistry::stdlib();
                let returns_iterator = |base: &str| {
                    lookup_method_signature(method, Some(base), stdlib)
                        .is_some_and(|s| s.has_self_receiver && matches!(&s.return_type, Some(Type::Custom(n)) if n == "Iterator"))
                };
                match path_of(object) {
                    Some(p) => {
                        let ty = self.type_of_path(&p)?;
                        returns_iterator(type_base(&ty)?).then_some(p)
                    }
                    // an adapter the table declares on `Iterator`, over a borrowing call
                    None => returns_iterator("Iterator").then(|| self.iterated_path(object)).flatten(),
                }
            }
            // by value: through `self` the generated loop may iterate a copy (a `&mut self` method whose body uses
            // `self`, see codegen `needs_owned_self_field_clone`), so only a local or parameter is decided here
            other => path_of(other).filter(|p| p[0] != "self"),
        }
    }

    fn check_loop(&mut self, iterable: &Expression<'_>, body: &[&Statement<'_>]) {
        let Some(path) = self.iterated_path(iterable) else { return };
        let Some(ty) = self.type_of_path(&path).filter(is_std_collection) else { return };
        let base = type_base(&ty).unwrap_or("Vec").to_string();
        let scan = Scan { path: &path, base: &base, spans: self.spans };
        if let Some(hit) = scan.block(body) {
            self.out.push((
                hit.key,
                LoopMutation {
                    path: path.join("."),
                    change: hit.change,
                    change_span: hit.span,
                    header_span: self.spans.expr(iterable),
                    line: hit.line,
                    column: hit.column,
                },
            ));
        }
    }
}

struct Hit {
    key: usize,
    change: Change,
    span: Option<NodeSpan>,
    line: usize,
    column: usize,
}

/// Looks for the first change to `path` in a loop body.
struct Scan<'a> {
    path: &'a [String],
    base: &'a str,
    spans: &'a SpanSnapshot,
}

impl Scan<'_> {
    fn root(&self) -> &str {
        &self.path[0]
    }

    fn is_path(&self, e: &Expression<'_>) -> bool {
        path_of(e).is_some_and(|p| p == self.path)
    }

    fn block(&self, stmts: &[&Statement<'_>]) -> Option<Hit> {
        for s in stmts {
            if let Some(h) = self.stmt(s) {
                return Some(h);
            }
            // a `let` of the root name starts a different binding: nothing after it is about the loop's collection
            if let Statement::Let { pattern, .. } = s {
                if binds(pattern, self.root()) {
                    return None;
                }
            }
        }
        None
    }

    fn stmt(&self, s: &Statement<'_>) -> Option<Hit> {
        match s {
            Statement::Assignment { target, value, .. } => {
                if let Some(change) = self.assigns(target) {
                    let (line, column) = pos(target);
                    return Some(Hit { key: *target as *const Expression as usize, change, span: self.spans.stmt(s), line, column });
                }
                self.expr(target).or_else(|| self.expr(value))
            }
            Statement::Let { value, else_block, .. } => self.expr(value).or_else(|| else_block.as_ref().and_then(|b| self.block(b))),
            Statement::Const { value, .. } | Statement::Static { value, .. } => self.expr(value),
            Statement::Return { value, .. } | Statement::Break { value, .. } => value.and_then(|v| self.expr(v)),
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition).or_else(|| self.block(then_block)).or_else(|| else_block.as_ref().and_then(|b| self.block(b)))
            }
            Statement::Match { value, arms, .. } => self.expr(value).or_else(|| self.arms(arms)),
            Statement::For { pattern, iterable, body, .. } => {
                self.expr(iterable).or_else(|| if binds(pattern, self.root()) { None } else { self.block(body) })
            }
            Statement::While { condition, body, .. } => self.expr(condition).or_else(|| self.block(body)),
            Statement::Loop { body, .. } => self.block(body),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Continue { .. } | Statement::Use { .. } => None,
        }
    }

    fn arms(&self, arms: &[MatchArm<'_>]) -> Option<Hit> {
        arms.iter().filter(|a| !binds(&a.pattern, self.root())).find_map(|a| a.guard.and_then(|g| self.expr(g)).or_else(|| self.expr(a.body)))
    }

    /// `v = ..` / `v[i] = ..` / `v[i].f += ..`.
    fn assigns(&self, target: &Expression<'_>) -> Option<Change> {
        if self.is_path(target) {
            return Some(Change::Whole);
        }
        match target {
            Expression::Index { object, .. } => (self.is_path(object) || self.assigns(object).is_some()).then_some(Change::Element),
            Expression::FieldAccess { object, .. } => self.assigns(object).filter(|c| *c == Change::Element),
            _ => None,
        }
    }

    fn expr(&self, e: &Expression<'_>) -> Option<Hit> {
        match e {
            Expression::MethodCall { object, method, .. }
                if self.is_path(object) && method_call_mutates_receiver(method, Some(self.base), SignatureRegistry::stdlib()) =>
            {
                let (line, column) = pos(e);
                return Some(Hit { key: e as *const Expression as usize, change: Change::Method(method.clone()), span: self.spans.expr(e), line, column });
            }
            Expression::Unary { op: UnaryOp::MutRef, operand, .. } if self.is_path(operand) => {
                let (line, column) = pos(e);
                return Some(Hit { key: e as *const Expression as usize, change: Change::Borrow, span: self.spans.expr(e), line, column });
            }
            Expression::Block { statements, .. } => return self.block(statements),
            Expression::Closure { parameters, body, .. } => {
                return if parameters.iter().any(|p| p == self.root()) { None } else { self.expr(body) };
            }
            _ => {}
        }
        children(e).into_iter().find_map(|c| self.expr(c))
    }
}

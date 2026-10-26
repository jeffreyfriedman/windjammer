//! `durable fn` checks and step sites (ADR-0021 section 9, stage E4), computed from the resolved program and the effect facts.
//!
//! A `durable fn` is an ordinary function with the marker attribute `@durable` (the parser turns the keyword into it, so no AST
//! shape changes). Its progress survives crashes: every call from its body that performs an effect is a **step**, whose result
//! is journaled; after a crash the body re-runs from the start and completed steps return their journaled result.
//!
//! Rules reported here (all compile-time, `wj check`):
//! * WJ0045  a step result, an argument or the return value cannot be journaled (SPEC-EFF-023)
//! * WJ0046  a step mutates a binding of the durable body (SPEC-EFF-024)
//! * WJ0047  the body reads state it does not own: channel receive, lock, atomic (SPEC-EFF-025)
//! * WJ0048  iteration over a `HashMap`/`HashSet` outside a step (SPEC-EFF-026)
//!
//! Output also includes `step_sites`: the source positions of the calls the interpreter journals.

use super::analysis::{full_path, sink_uses, EffectFacts};
use super::sinks;
use crate::parser::ast::core::{Expression, FunctionDecl, Pattern, Statement};
use crate::parser::ast::operators::UnaryOp;
use crate::parser::ast::types::Type;
use crate::resolve::{CallKind, CallResolution, DefId, ExprKey, Reason, Res, ResolvedProgram};
use crate::source_map::Location;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DurableFinding {
    pub code: &'static str,
    pub function: String,
    pub message: String,
    pub site: Option<Location>,
    /// The word to underline at `site`.
    pub word: String,
    pub help: String,
}

#[derive(Debug, Default)]
pub struct DurableFacts {
    /// Paths of the durable functions.
    pub functions: Vec<String>,
    /// (file, line, column) of every call in a durable body that is a journaled step.
    pub step_sites: BTreeSet<(PathBuf, usize, usize)>,
    pub findings: Vec<DurableFinding>,
}

pub fn is_durable(d: &FunctionDecl<'_>) -> bool {
    d.decorators.iter().any(|x| x.name == "durable")
}

/// Types that are live handles or shared cells: they have no stable serialization.
const HANDLE_TYPES: &[&str] = &["File", "TcpStream", "TcpListener", "Socket", "Child", "JoinHandle", "Rc", "RefCell", "Cell", "Weak", "Stream"];
/// Types of state the body does not own; reading them is WJ0047's business, not a journaling error.
const SHARED_TYPES: &[&str] = &["Receiver", "Sender", "Mutex", "RwLock", "Condvar", "Barrier"];
const MUTATING_METHODS: &[&str] = &[
    "push", "push_str", "insert", "remove", "pop", "clear", "extend", "append", "sort", "sort_by", "reverse", "truncate", "retain", "drain", "swap",
    "dedup", "push_back", "push_front", "pop_front", "pop_back", "entry", "resize", "set",
];
const SHARED_READS: &[&str] = &["recv", "try_recv", "recv_timeout", "select", "lock", "try_lock", "load", "store", "fetch_add", "compare_exchange"];

fn show(t: &Type) -> String {
    match t {
        Type::Int => "int".into(),
        Type::Int32 => "i32".into(),
        Type::Uint => "uint".into(),
        Type::Float => "float".into(),
        Type::Bool => "bool".into(),
        Type::String => "string".into(),
        Type::Custom(n) | Type::Generic(n) | Type::TraitObject(n) | Type::ImplTrait(n) => n.clone(),
        Type::Parameterized(n, a) => format!("{n}<{}>", a.iter().map(show).collect::<Vec<_>>().join(", ")),
        Type::Option(i) => format!("Option<{}>", show(i)),
        Type::Vec(i) => format!("Vec<{}>", show(i)),
        Type::Result(a, b) => format!("Result<{}, {}>", show(a), show(b)),
        Type::Array(i, n) => format!("[{}; {n}]", show(i)),
        Type::Reference(i) => format!("&{}", show(i)),
        Type::MutableReference(i) => format!("&mut {}", show(i)),
        Type::Tuple(ts) => format!("({})", ts.iter().map(show).collect::<Vec<_>>().join(", ")),
        Type::FunctionPointer { params, return_type } => {
            let ret = return_type.as_ref().map(|r| format!(" -> {}", show(r))).unwrap_or_default();
            format!("fn({}){ret}", params.iter().map(show).collect::<Vec<_>>().join(", "))
        }
        Type::RawPointer { mutable, pointee } => format!("*{} {}", if *mutable { "mut" } else { "const" }, show(pointee)),
        Type::Associated(a, b) => format!("{a}::{b}"),
        Type::Infer => "_".into(),
    }
}

fn last_seg(n: &str) -> &str {
    n.rsplit("::").next().unwrap_or(n)
}

/// The first part of `t` that cannot be journaled, displayed.
fn unjournalable(t: &Type) -> Option<String> {
    match t {
        Type::FunctionPointer { .. } | Type::RawPointer { .. } | Type::TraitObject(_) | Type::ImplTrait(_) => Some(show(t)),
        Type::Reference(i) | Type::MutableReference(i) | Type::Option(i) | Type::Vec(i) | Type::Array(i, _) => unjournalable(i),
        Type::Result(a, b) => unjournalable(a).or_else(|| unjournalable(b)),
        Type::Tuple(ts) => ts.iter().find_map(unjournalable),
        Type::Parameterized(n, args) => {
            if SHARED_TYPES.contains(&last_seg(n)) {
                None
            } else if HANDLE_TYPES.contains(&last_seg(n)) {
                Some(show(t))
            } else {
                args.iter().find_map(unjournalable)
            }
        }
        Type::Custom(n) if HANDLE_TYPES.contains(&last_seg(n)) => Some(show(t)),
        _ => None,
    }
}

fn hash_kind(t: &Type) -> Option<&'static str> {
    match t {
        Type::Parameterized(n, _) | Type::Custom(n) => match last_seg(n) {
            "HashMap" => Some("HashMap"),
            "HashSet" => Some("HashSet"),
            _ => None,
        },
        Type::Reference(i) | Type::MutableReference(i) => hash_kind(i),
        _ => None,
    }
}

/// The variable at the root of `a.b[c].d`.
fn root_ident<'a>(e: &'a Expression<'_>) -> Option<&'a str> {
    match e {
        Expression::Identifier { name, .. } => Some(name),
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => root_ident(object),
        Expression::Unary { operand, .. } => root_ident(operand),
        _ => None,
    }
}

/// Does `stmts` (a callee body) mutate the variable `name`?
fn mutates(name: &str, stmts: &[&Statement<'_>]) -> bool {
    stmts.iter().any(|s| stmt_mutates(name, s))
}

fn stmt_mutates(name: &str, s: &Statement<'_>) -> bool {
    match s {
        Statement::Assignment { target, value, .. } => root_ident(target) == Some(name) || expr_mutates(name, value),
        Statement::Let { value, else_block, .. } => expr_mutates(name, value) || else_block.as_ref().is_some_and(|b| mutates(name, b)),
        Statement::Expression { expr, .. } => expr_mutates(name, expr),
        Statement::Return { value, .. } => value.is_some_and(|v| expr_mutates(name, v)),
        Statement::If { condition, then_block, else_block, .. } => {
            expr_mutates(name, condition) || mutates(name, then_block) || else_block.as_ref().is_some_and(|b| mutates(name, b))
        }
        Statement::For { iterable, body, .. } => expr_mutates(name, iterable) || mutates(name, body),
        Statement::While { condition, body, .. } => expr_mutates(name, condition) || mutates(name, body),
        Statement::Loop { body, .. } | Statement::Thread { body, .. } | Statement::Async { body, .. } => mutates(name, body),
        Statement::Match { value, arms, .. } => expr_mutates(name, value) || arms.iter().any(|a| expr_mutates(name, a.body)),
        Statement::Defer { statement, .. } => stmt_mutates(name, statement),
        _ => false,
    }
}

fn expr_mutates(name: &str, e: &Expression<'_>) -> bool {
    match e {
        Expression::MethodCall { object, method, arguments, .. } => {
            (MUTATING_METHODS.contains(&method.as_str()) && root_ident(object) == Some(name))
                || expr_mutates(name, object)
                || arguments.iter().any(|(_, a)| expr_mutates(name, a))
        }
        Expression::Call { arguments, .. } => arguments.iter().any(|(_, a)| match a {
            Expression::Unary { op: UnaryOp::MutRef, operand, .. } => root_ident(operand) == Some(name),
            other => expr_mutates(name, other),
        }),
        Expression::Block { statements, .. } => mutates(name, statements),
        Expression::Binary { left, right, .. } => expr_mutates(name, left) || expr_mutates(name, right),
        Expression::Unary { operand, .. } => expr_mutates(name, operand),
        Expression::TryOp { expr, .. } | Expression::Await { expr, .. } | Expression::Cast { expr, .. } => expr_mutates(name, expr),
        Expression::Closure { body, .. } => expr_mutates(name, body),
        _ => false,
    }
}

struct Pass<'p, 'a> {
    r: &'p ResolvedProgram<'a>,
    facts: &'p EffectFacts,
    calls: &'p HashMap<ExprKey, &'p CallResolution>,
    out: &'p mut DurableFacts,
    function: String,
    /// Variables of the durable body (parameters and lets).
    locals: HashSet<String>,
    /// Variables known to hold a `HashMap` / `HashSet`.
    hashes: HashMap<String, &'static str>,
}

impl Pass<'_, '_> {
    fn finding(&mut self, code: &'static str, message: String, site: Option<Location>, word: &str, help: &str) {
        self.out.findings.push(DurableFinding { code, function: self.function.clone(), message, site, word: word.to_string(), help: help.to_string() });
    }

    fn effectful(&self, c: &CallResolution) -> bool {
        let fn_effectful = |id: &DefId| self.facts.get(*id).is_some_and(|f| !f.effects.is_empty() || f.open);
        match &c.res {
            Res::Def(id) => fn_effectful(id),
            Res::MethodCandidates(ids) | Res::Ambiguous(ids) => ids.iter().any(fn_effectful),
            Res::Unresolved(Reason::External(p)) | Res::Prim(p) => !sink_uses(&full_path(p, &c.callee), c).is_empty(),
            Res::Unresolved(Reason::NotFound(_)) if c.kind == CallKind::Free => !sink_uses(&c.callee, c).is_empty(),
            Res::Unresolved(Reason::Dynamic) | Res::Local(_) => true,
            _ => false,
        }
    }

    fn call(&mut self, e: &Expression<'_>) {
        let Some(c) = self.calls.get(&ExprKey(e as *const Expression<'_> as usize)).copied() else { return };
        // state the body does not own (WJ0047): receives, locks, atomics with no user method of that name
        if c.kind == CallKind::Method && matches!(c.res, Res::Unresolved(Reason::NoMethodCandidate(_))) && SHARED_READS.contains(&c.callee.as_str()) {
            let what = match c.callee.as_str() {
                "recv" | "try_recv" | "recv_timeout" | "select" => "receives from a channel".to_string(),
                m => format!("reads shared state (`{m}`)"),
            };
            self.finding(
                "WJ0047",
                format!("durable fn `{}` {what}", self.function),
                c.location.clone(),
                &c.callee,
                "pass the value in as an argument, or do the receive inside a step that returns data",
            );
            return;
        }
        if !self.effectful(c) {
            return;
        }
        if let Some(l) = &c.location {
            self.out.step_sites.insert((l.file.clone(), l.line, l.column));
        }
        let handle = sinks::HANDLE_SINKS.contains(&sinks::canonical(&c.callee).as_str());
        let ret = match &c.res {
            Res::Def(id) => self.r.collected.decls.get(id).and_then(|d| d.return_type.clone()),
            _ => None,
        };
        let bad = if handle { Some("a live handle".to_string()) } else { ret.as_ref().and_then(unjournalable) };
        if let Some(t) = bad {
            self.finding(
                "WJ0045",
                format!("step result of type `{t}` cannot be journaled"),
                c.location.clone(),
                last_seg(&c.callee),
                "do the work inside the step and return data (numbers, strings, collections, structs of those)",
            );
        }
        if let Res::Def(id) = &c.res {
            if let Some(d) = self.r.collected.decls.get(id) {
                let params: Vec<_> = d.parameters.iter().filter(|p| p.name != "self").collect();
                for (i, arg) in c.ident_args.iter().enumerate() {
                    let (Some(arg), Some(p)) = (arg, params.get(i)) else { continue };
                    if self.locals.contains(arg) && (matches!(p.type_, Type::MutableReference(_)) || mutates(&p.name, &d.body)) {
                        self.finding(
                            "WJ0046",
                            format!("step `{}` mutates `{arg}`", last_seg(&c.callee)),
                            c.location.clone(),
                            last_seg(&c.callee),
                            "return the new value from the step and bind it in the durable body: replay skips the mutation",
                        );
                    }
                }
            }
        }
    }

    fn block(&mut self, stmts: &[&Statement<'_>]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    fn bind(&mut self, p: &Pattern<'_>) {
        match p {
            Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) | Pattern::RefMut(n) => {
                self.locals.insert(n.clone());
            }
            Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|q| self.bind(q)),
            Pattern::Reference(q) => self.bind(q),
            _ => {}
        }
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        match s {
            Statement::Let { pattern, value, else_block, type_, .. } => {
                self.expr(value);
                if let Some(b) = else_block {
                    self.block(b);
                }
                if let (Pattern::Identifier(n) | Pattern::MutBinding(n), Some(kind)) = (pattern, hash_of_value(value).or_else(|| type_.as_ref().and_then(hash_kind))) {
                    self.hashes.insert(n.clone(), kind);
                }
                self.bind(pattern);
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => self.expr(value),
            Statement::Assignment { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
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
                    self.bind(&a.pattern);
                    if let Some(g) = a.guard {
                        self.expr(g);
                    }
                    self.expr(a.body);
                }
            }
            Statement::For { pattern, iterable, body, .. } => {
                if let Some(kind) = self.hash_iteration(iterable) {
                    self.finding(
                        "WJ0048",
                        format!("iteration over `{kind}` has no stable order"),
                        iterable.location(),
                        "for",
                        "use OrderedMap/OrderedSet or BTreeMap, or sort the keys first",
                    );
                }
                self.expr(iterable);
                self.bind(pattern);
                self.block(body);
            }
            Statement::While { condition, body, .. } => {
                self.expr(condition);
                self.block(body);
            }
            Statement::Loop { body, .. } | Statement::Thread { body, .. } | Statement::Async { body, .. } => self.block(body),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Use { .. } | Statement::Break { .. } | Statement::Continue { .. } => {}
        }
    }

    /// `for x in m`, `for x in m.iter()` / `.keys()` / `.values()` ... where `m` holds a `HashMap` or `HashSet`.
    fn hash_iteration(&self, iterable: &Expression<'_>) -> Option<&'static str> {
        match iterable {
            Expression::Identifier { name, .. } => self.hashes.get(name).copied(),
            Expression::MethodCall { object, method, .. } if matches!(method.as_str(), "iter" | "iter_mut" | "keys" | "values" | "values_mut" | "into_iter" | "drain") => {
                self.hash_iteration(object)
            }
            Expression::Unary { operand, .. } => self.hash_iteration(operand),
            _ => None,
        }
    }

    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Call { function, arguments, .. } => {
                self.call(e);
                self.expr(function);
                arguments.iter().for_each(|(_, a)| self.expr(a));
            }
            Expression::MethodCall { object, arguments, .. } => {
                self.call(e);
                self.expr(object);
                arguments.iter().for_each(|(_, a)| self.expr(a));
            }
            Expression::ChannelRecv { channel, location } => {
                self.finding(
                    "WJ0047",
                    format!("durable fn `{}` receives from a channel", self.function),
                    location.clone(),
                    "<-",
                    "pass the value in as an argument, or do the receive inside a step that returns data",
                );
                self.expr(channel);
            }
            Expression::Closure { body, .. } => self.expr(body),
            Expression::Block { statements, .. } => self.block(statements),
            Expression::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expression::Unary { operand, .. } => self.expr(operand),
            Expression::FieldAccess { object, .. } => self.expr(object),
            Expression::StructLiteral { fields, .. } => fields.iter().for_each(|(_, v)| self.expr(v)),
            Expression::MapLiteral { pairs, .. } => pairs.iter().for_each(|(k, v)| {
                self.expr(k);
                self.expr(v);
            }),
            Expression::Range { start, end, .. } => {
                self.expr(start);
                self.expr(end);
            }
            Expression::Cast { expr, .. }
            | Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. } => self.expr(expr),
            Expression::Index { object, index, .. } => {
                self.expr(object);
                self.expr(index);
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements.iter().for_each(|x| self.expr(x)),
            Expression::MacroInvocation { args, .. } => args.iter().for_each(|x| self.expr(x)),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr(channel);
                self.expr(value);
            }
            Expression::Literal { .. } | Expression::Identifier { .. } => {}
        }
    }
}

/// `HashMap::new()` and friends.
fn hash_of_value(e: &Expression<'_>) -> Option<&'static str> {
    if let Expression::Call { function, .. } = e {
        if let Expression::Identifier { name, .. } = function {
            if name.starts_with("HashMap::") {
                return Some("HashMap");
            }
            if name.starts_with("HashSet::") {
                return Some("HashSet");
            }
        }
    }
    None
}

pub fn analyze(r: &ResolvedProgram<'_>, facts: &EffectFacts) -> DurableFacts {
    let mut out = DurableFacts::default();
    let calls: HashMap<ExprKey, &CallResolution> = r.calls.iter().map(|c| (c.key, c)).collect();
    let mut ids: Vec<DefId> = r.collected.decls.iter().filter(|(_, d)| is_durable(d)).map(|(id, _)| *id).collect();
    ids.sort();
    for id in ids {
        let d = r.collected.decls[&id];
        let path = r.defs().get(id).path.clone();
        out.functions.push(path.clone());
        let function = last_seg(&path).to_string();
        let mut pass = Pass { r, facts, calls: &calls, out: &mut out, function: function.clone(), locals: HashSet::new(), hashes: HashMap::new() };
        for p in &d.parameters {
            pass.locals.insert(p.name.clone());
            if let Some(kind) = hash_kind(&p.type_) {
                pass.hashes.insert(p.name.clone(), kind);
            }
            if let Some(t) = unjournalable(&p.type_) {
                pass.finding(
                    "WJ0045",
                    format!("argument `{}` of type `{t}` cannot be journaled", p.name),
                    None,
                    &p.name,
                    "pass data (numbers, strings, collections, structs of those)",
                );
            }
        }
        if let Some(t) = d.return_type.as_ref().and_then(unjournalable) {
            pass.finding("WJ0045", format!("return type `{t}` cannot be journaled"), None, &d.name, "return data");
        }
        pass.block(&d.body);
    }
    out
}

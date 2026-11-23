//! The taint flow: which values hold data from a source, per function, with summaries so a flow through a helper is seen.
//!
//! A value's taint is the set of origins it may hold: a `Source` (a call that read untrusted data, with its place) or
//! `Param(i)` (whatever the caller passes as parameter `i`; symbolic, for summaries). Each function is analyzed with every
//! parameter symbolic. Its summary records which origins reach its result and which parameters reach a sink; a call applies
//! the callee's summary to the arguments' taint. The summaries are iterated to a fixed point (recursion included).
//!
//! The walk is flow-sensitive in order and joins at branches: `let` and `=` replace the taint of a name, an `if`/`match`
//! joins its arms, a loop body is walked twice. Operators that cannot carry text (comparisons, arithmetic on numbers,
//! `len`, `parse`) produce clean values; values of scalar declared type are clean.
//!
//! Annotations (doc comments on a function): `/// wj-taint: sanitizer` makes its result clean for every sink;
//! `/// wj-taint: trusted(<reason>)` reports nothing inside the function (the reason is required and is the review record).

use super::catalog::{self, SinkRole};
use crate::parser::ast::core::{EnumPatternBinding, Expression, FunctionDecl, Pattern, Statement};
use crate::parser::ast::operators::{BinaryOp, UnaryOp};
use crate::parser::ast::types::Type;
use crate::resolve::{CallKind, CallResolution, DefId, ExprKey, Reason, ResolvedProgram, Res};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Place {
    pub file: String,
    pub line: usize,
    pub column: usize,
    /// The identifier at the place (`read_to_string`, `run`), for the span.
    pub word: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    Source { what: String, callee: String, at: Place },
    /// Parameter `i`, or the part of it reached through `path` (`page.filename` is `Param(i, ["filename"])`).
    Param(usize, Vec<String>),
    /// A value built as a struct: the origin holds in field `name` only (a struct literal's fields are tracked apart).
    Field(String, Box<Origin>),
}

pub type Taint = BTreeSet<Origin>;


/// Bounds on the walk. Every one of them OVER-approximates: a value may only get more origins, never fewer, so a flow that
/// exists is never dropped (this is a security analysis).
/// * wrapping deeper than `MAX_WRAP_DEPTH` flattens the value to its base origins: `select` of a field then keeps it, where the
///   wrapped form would have dropped it for a different field, so the flattened set is a superset of every selection;
/// * a parameter path longer than `MAX_PATH` stops growing: `Param(i, p)` stands for everything reachable from that member, so the
///   truncated origin includes every deeper one;
/// * more than `MAX_ORIGINS` origins in one value collapse to TOP, the function's `top` set: every source call textually in the
///   function, the return origins of every function it calls, and every parameter. Any origin a value of this function can hold
///   is in it, so a sink fed by the collapsed value still fires;
/// * a loop is walked until a pass adds nothing (a fixpoint, at most four passes, then TOP for what it assigns); a loop nested more
///   than three deep is walked once after every variable it assigns first received TOP (`havoc`), so a flow carried from one
///   iteration to the next, however long the chain of assignments, is present.
/// A sanitizer is a declaration (`std` table, `/// wj-taint: sanitizer`): its result is clean for every input, TOP included, exactly
/// as for any other value; the bounds never turn a declared sanitizer into a non-sanitizer nor the reverse.
const MAX_WRAP_DEPTH: usize = 3;
const MAX_PATH: usize = 3;
const MAX_ORIGINS: usize = 48;

fn wrap_depth(o: &Origin) -> usize {
    match o {
        Origin::Field(_, inner) => 1 + wrap_depth(inner),
        _ => 0,
    }
}

/// `o` held in field `name`, unless it is already wrapped `MAX_WRAP_DEPTH` deep.
pub fn wrap(name: &str, o: Origin) -> Origin {
    if wrap_depth(&o) >= MAX_WRAP_DEPTH {
        return flatten_one(&o);
    }
    Origin::Field(name.to_string(), Box::new(o))
}

fn flatten_one(o: &Origin) -> Origin {
    let mut cur = o;
    while let Origin::Field(_, inner) = cur {
        cur = inner;
    }
    cur.clone()
}

/// Keep a value's origin set bounded.
fn bound(t: Taint, top: &Taint) -> Taint {
    if t.len() <= MAX_ORIGINS {
        return t;
    }
    let mut out: Taint = t.iter().map(flatten_one).collect();
    if out.len() > MAX_ORIGINS {
        out = top.clone();
    }
    out
}

/// The taint of member `name` of a value with taint `t`: the field's own origins, whole-value origins, and a parameter's
/// member. Other fields' origins do not reach it.
pub fn select(t: &Taint, name: &str) -> Taint {
    t.iter()
        .filter_map(|o| match o {
            Origin::Field(n, inner) => (n == name).then(|| (**inner).clone()),
            Origin::Param(i, path) => {
                let mut p = path.clone();
                if p.len() < MAX_PATH {
                    p.push(name.to_string());
                }
                Some(Origin::Param(*i, p))
            }
            src => Some(src.clone()),
        })
        .collect()
}

fn select_path(t: &Taint, path: &[String]) -> Taint {
    path.iter().fold(t.clone(), |acc, n| select(&acc, n))
}

/// Every origin of a value regardless of which field holds it (for a value consumed whole).
pub fn flatten(t: &Taint) -> Taint {
    t.iter()
        .map(|o| {
            let mut cur = o;
            while let Origin::Field(_, inner) = cur {
                cur = inner;
            }
            cur.clone()
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SinkSite {
    pub role: SinkRole,
    pub callee: String,
    pub at: Place,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub ret: Taint,
    /// Parameter index -> the sinks it reaches.
    pub sinks: BTreeMap<(usize, Vec<String>), BTreeSet<SinkSite>>,
    pub sanitizer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub source_what: String,
    pub source_callee: String,
    pub source_at: Place,
    pub sink: SinkSite,
    /// Where it is reported: the sink call, or the call of the helper whose parameter reaches the sink.
    pub report_at: Place,
    /// The helpers the value passes through, outermost first.
    pub via: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pragma {
    None,
    Sanitizer,
    Trusted,
    /// `trusted` without a reason.
    TrustedNoReason,
}

pub fn pragma(decl: &FunctionDecl<'_>) -> Pragma {
    pragma_of_doc(decl.doc_comment.as_deref())
}

/// The `wj-taint:` pragma of a doc comment (functions, methods and trait methods all carry one).
pub fn pragma_of_doc(doc: Option<&str>) -> Pragma {
    let Some(doc) = doc else { return Pragma::None };
    for line in doc.lines() {
        let l = line.trim().trim_start_matches('/').trim();
        let Some(rest) = l.strip_prefix("wj-taint:") else { continue };
        let rest = rest.trim();
        if rest == "sanitizer" {
            return Pragma::Sanitizer;
        }
        if let Some(r) = rest.strip_prefix("trusted") {
            let r = r.trim();
            let reason = r.strip_prefix('(').and_then(|x| x.strip_suffix(')')).map(str::trim).unwrap_or("");
            return if reason.is_empty() { Pragma::TrustedNoReason } else { Pragma::Trusted };
        }
    }
    Pragma::None
}

/// Everything the walk reads about the program.
pub struct Program<'p, 'a> {
    pub resolved: &'p ResolvedProgram<'a>,
    pub by_key: HashMap<ExprKey, &'p CallResolution>,
}

impl<'p, 'a> Program<'p, 'a> {
    pub fn new(resolved: &'p ResolvedProgram<'a>) -> Self {
        let by_key = resolved.calls.iter().map(|c| (c.key, c)).collect();
        Program { resolved, by_key }
    }
}

fn is_scalar(t: &Type) -> bool {
    match t {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::Custom(n) => matches!(
            n.as_str(),
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "f32" | "f64" | "bool" | "char"
        ),
        Type::Option(i) | Type::Reference(i) | Type::MutableReference(i) | Type::Vec(i) | Type::Array(i, _) => is_scalar(i),
        Type::Result(ok, _) => is_scalar(ok),
        Type::Tuple(ts) => ts.iter().all(is_scalar),
        _ => false,
    }
}

fn strip_refs(t: &Type) -> &Type {
    match t {
        Type::Reference(i) | Type::MutableReference(i) => strip_refs(i),
        o => o,
    }
}

fn pattern_names(p: &Pattern<'_>, out: &mut Vec<String>) {
    match p {
        Pattern::Identifier(n) | Pattern::Ref(n) | Pattern::RefMut(n) | Pattern::MutBinding(n) => out.push(n.clone()),
        Pattern::EnumVariant(_, b) => match b {
            EnumPatternBinding::Single(n) => out.push(n.clone()),
            EnumPatternBinding::Tuple(ps) => ps.iter().for_each(|p| pattern_names(p, out)),
            EnumPatternBinding::Struct(fs, _) => fs.iter().for_each(|(_, p)| pattern_names(p, out)),
            _ => {}
        },
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|p| pattern_names(p, out)),
        Pattern::Reference(p) => pattern_names(p, out),
        _ => {}
    }
}

type Env = HashMap<String, Taint>;

fn join_into(a: &mut Env, b: &Env, top: &Taint) {
    for (k, v) in b {
        let slot = a.entry(k.clone()).or_default();
        if slot != v {
            slot.extend(v.iter().cloned());
            if slot.len() > MAX_ORIGINS {
                *slot = bound(std::mem::take(slot), top);
            }
        }
    }
}

pub struct Walker<'w, 'p, 'a> {
    prog: &'w Program<'p, 'a>,
    summaries: &'w HashMap<DefId, Summary>,
    file: String,
    env: Env,
    bound: HashSet<String>,
    conns: HashSet<String>,
    request_types: HashMap<String, String>,
    ret: Taint,
    sinks: BTreeMap<(usize, Vec<String>), BTreeSet<SinkSite>>,
    pub findings: Vec<Finding>,
    quiet: bool,
    /// Loops being walked.
    loop_depth: usize,
    /// Any origin a value of this function can hold (see `MAX_ORIGINS`).
    top: Taint,
    request_cache: HashMap<String, bool>,
}

pub struct Outcome {
    pub summary: Summary,
    pub findings: Vec<Finding>,
}

/// Analyze one function with symbolic parameters.
pub fn analyze_fn(prog: &Program<'_, '_>, summaries: &HashMap<DefId, Summary>, decl: &FunctionDecl<'_>, file: &str) -> Outcome {
    let pr = pragma(decl);
    let mut w = Walker {
        prog,
        summaries,
        file: file.to_string(),
        env: HashMap::new(),
        bound: HashSet::new(),
        conns: HashSet::new(),
        request_types: HashMap::new(),
        ret: Taint::new(),
        sinks: BTreeMap::new(),
        findings: Vec::new(),
        quiet: matches!(pr, Pragma::Trusted),
        loop_depth: 0,
        top: Taint::new(),
        request_cache: HashMap::new(),
    };
    for (i, p) in decl.parameters.iter().enumerate() {
        w.bound.insert(p.name.clone());
        let ty = strip_refs(&p.type_);
        if is_scalar(ty) {
            continue;
        }
        if let Type::Custom(n) = ty {
            let base = n.rsplit("::").next().unwrap_or(n);
            if base == "Connection" {
                w.conns.insert(p.name.clone());
            }
            w.request_types.insert(p.name.clone(), base.to_string());
        }
        let mut t = Taint::new();
        t.insert(Origin::Param(i, Vec::new()));
        w.env.insert(p.name.clone(), t);
    }
    w.compute_top(decl);
    let tail = w.block(&decl.body);
    w.ret.extend(tail);
    let mut summary = Summary { ret: std::mem::take(&mut w.ret), sinks: std::mem::take(&mut w.sinks), sanitizer: pr == Pragma::Sanitizer };
    if decl.return_type.as_ref().map_or(true, is_scalar) || summary.sanitizer {
        summary.ret.clear();
    }
    if matches!(pr, Pragma::Trusted | Pragma::TrustedNoReason) {
        summary.sinks.clear();
    }
    let findings = if w.quiet { Vec::new() } else { w.findings };
    Outcome { summary, findings }
}

impl<'w, 'p, 'a> Walker<'w, 'p, 'a> {
    fn place(&self, loc: &crate::parser::ast::types::SourceLocation, word: &str) -> Place {
        let (line, column) = loc
            .as_ref()
            .map(|l| (l.line, crate::source_map::display_column(l.column)))
            .unwrap_or((0, 0));
        Place { file: self.file.clone(), line, column, word: word.to_string() }
    }

    /// Walk a loop. The environment after the loop is the least fixpoint of "join the body's effect into it", so a flow carried
    /// from one iteration to the next, however many assignments long, is found: the body is walked until a pass adds nothing. Two
    /// bounds keep that linear, and both over-approximate: after `MAX_LOOP_PASSES` passes that still change something, and for
    /// every loop nested more than three deep, each variable the body assigns first receives TOP (`havoc`) and the body is walked
    /// once more (everything a later iteration could hold is already in TOP).
    fn run_loop(&mut self, body: &[&Statement<'_>], head: &mut dyn FnMut(&mut Self)) {
        const MAX_LOOP_PASSES: usize = 4;
        self.loop_depth += 1;
        if self.loop_depth > 3 {
            self.havoc(body);
            let before = self.env.clone();
            head(self);
            self.block(body);
            join_into(&mut self.env, &before, &self.top);
        } else {
            let mut stable = false;
            for _ in 0..MAX_LOOP_PASSES {
                let before = self.env.clone();
                head(self);
                self.block(body);
                join_into(&mut self.env, &before, &self.top);
                if self.env == before {
                    stable = true;
                    break;
                }
            }
            if !stable {
                self.havoc(body);
                let before = self.env.clone();
                head(self);
                self.block(body);
                join_into(&mut self.env, &before, &self.top);
            }
        }
        self.loop_depth -= 1;
    }

    /// Give every variable the body assigns (or stores into) TOP.
    fn havoc(&mut self, body: &[&Statement<'_>]) {
        let names: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
        scan(
            body,
            &mut |st| {
                if let Statement::Assignment { target, .. } = st {
                    names.borrow_mut().extend(root_ident(target));
                }
            },
            &mut |e| {
                if let Expression::MethodCall { object, method, .. } = e {
                    if catalog::STORING_METHODS.contains(&method.as_str()) {
                        names.borrow_mut().extend(root_ident(object));
                    }
                }
            },
        );
        for n in names.into_inner() {
            let slot = self.env.entry(n).or_default();
            slot.extend(self.top.iter().cloned());
        }
    }

    /// Every origin a value of this function can hold: the std source calls in its text, what the functions it calls return, what
    /// an HTTP request parameter yields, and its parameters.
    fn compute_top(&mut self, decl: &FunctionDecl<'_>) {
        type Site = (ExprKey, crate::parser::ast::types::SourceLocation, Option<(String, String)>);
        let mut sites: Vec<Site> = Vec::new();
        scan(&decl.body, &mut |_| {}, &mut |e| {
            let member = match e {
                Expression::MethodCall { object, method, .. } => Some((&**object, method.as_str())),
                Expression::FieldAccess { object, field, .. } => Some((&**object, field.as_str())),
                _ => None,
            };
            let request = match member {
                Some((Expression::Identifier { name, .. }, m)) => Some((name.clone(), m.to_string())),
                _ => None,
            };
            sites.push((ExprKey(e as *const Expression<'_> as usize), e.location(), request));
        });
        let mut top = Taint::new();
        let (prog, summaries) = (self.prog, self.summaries);
        for (key, loc, request) in sites {
            if let Some(c) = prog.by_key.get(&key) {
                match &c.res {
                    Res::Def(id) => top.extend(summaries.get(id).into_iter().flat_map(|s| s.ret.iter().map(flatten_one)).filter(|o| matches!(o, Origin::Source { .. }))),
                    Res::MethodCandidates(ids) => {
                        for id in ids {
                            top.extend(summaries.get(id).into_iter().flat_map(|s| s.ret.iter().map(flatten_one)).filter(|o| matches!(o, Origin::Source { .. })));
                        }
                    }
                    Res::Unresolved(Reason::External(p)) | Res::Prim(p) => {
                        let path = full(p, &c.callee);
                        if let Some(src) = catalog::source(&path) {
                            let word = path.rsplit("::").next().unwrap_or(&path).to_string();
                            top.extend(self.source(src.what, &canonical(&path), &loc, &word));
                        }
                    }
                    _ => {}
                }
            }
            if let Some((name, m)) = request {
                if let Some(t) = self.request_member_at(&name, &m, &loc) {
                    top.extend(t.iter().map(flatten_one));
                }
            }
        }
        for (i, p) in decl.parameters.iter().enumerate() {
            if !is_scalar(strip_refs(&p.type_)) {
                top.insert(Origin::Param(i, Vec::new()));
            }
        }
        self.top = top;
    }

    fn block(&mut self, stmts: &[&Statement<'_>]) -> Taint {
        let mut tail = Taint::new();
        for (i, s) in stmts.iter().enumerate() {
            let last = i + 1 == stmts.len();
            let t = self.stmt(s);
            if last {
                tail = t;
            }
        }
        tail
    }

    fn branch(&mut self, stmts: &[&Statement<'_>], base: &Env) -> (Env, Taint) {
        let saved = std::mem::replace(&mut self.env, base.clone());
        let t = self.block(stmts);
        (std::mem::replace(&mut self.env, saved), t)
    }

    fn bind(&mut self, p: &Pattern<'_>, t: &Taint) {
        // `Ok(x)` / `Err(e)`: x holds the `Ok` payload of the matched value, e its `Err` payload
        if let Pattern::EnumVariant(v, EnumPatternBinding::Single(n)) = p {
            if v == "Ok" || v == "Err" {
                let payload = select(t, v);
                self.bound.insert(n.clone());
                self.conns.remove(n);
                self.env.insert(n.clone(), payload);
                return;
            }
        }
        let mut names = Vec::new();
        pattern_names(p, &mut names);
        for n in names {
            self.bound.insert(n.clone());
            self.conns.remove(&n);
            self.env.insert(n, t.clone());
        }
    }

    fn stmt(&mut self, s: &Statement<'_>) -> Taint {
        match s {
            Statement::Let { pattern, type_, value, else_block, .. } => {
                let mut t = self.expr(value);
                if type_.as_ref().is_some_and(is_scalar) {
                    t.clear();
                }
                let is_conn = self.is_connect(value);
                self.bind(pattern, &t);
                if is_conn {
                    if let Pattern::Identifier(n) | Pattern::MutBinding(n) = pattern {
                        self.conns.insert(n.clone());
                    }
                }
                if let Some(e) = else_block {
                    let base = self.env.clone();
                    self.branch(e, &base);
                }
                Taint::new()
            }
            Statement::Assignment { target, value, compound_op, .. } => {
                let t = self.expr(value);
                match target {
                    Expression::Identifier { name, .. } => {
                        if compound_op.is_some() {
                            let slot = self.env.entry(name.clone()).or_default();
                            slot.extend(t);
                            if slot.len() > MAX_ORIGINS {
                                *slot = bound(std::mem::take(slot), &self.top);
                            }
                        } else {
                            self.env.insert(name.clone(), t);
                        }
                    }
                    other => {
                        if let Some(root) = root_ident(other) {
                            let slot = self.env.entry(root).or_default();
                            slot.extend(t);
                            if slot.len() > MAX_ORIGINS {
                                *slot = bound(std::mem::take(slot), &self.top);
                            }
                        }
                        self.sub_exprs(other);
                    }
                }
                Taint::new()
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    let t = self.expr(v);
                    self.ret.extend(t);
                }
                Taint::new()
            }
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition);
                let base = self.env.clone();
                let (mut env, mut tail) = self.branch(then_block, &base);
                match else_block {
                    Some(e) => {
                        let (env2, t2) = self.branch(e, &base);
                        join_into(&mut env, &env2, &self.top);
                        tail.extend(t2);
                    }
                    None => join_into(&mut env, &base, &self.top),
                }
                self.env = env;
                tail
            }
            Statement::Match { value, arms, .. } => {
                let scrut = self.expr(value);
                let base = self.env.clone();
                let mut joined: Option<Env> = None;
                let mut tail = Taint::new();
                for arm in arms {
                    let saved = std::mem::replace(&mut self.env, base.clone());
                    self.bind(&arm.pattern, &scrut);
                    if let Some(g) = arm.guard {
                        self.expr(g);
                    }
                    tail.extend(self.expr(arm.body));
                    let e = std::mem::replace(&mut self.env, saved);
                    match joined.as_mut() {
                        Some(j) => join_into(j, &e, &self.top),
                        None => joined = Some(e),
                    }
                }
                if let Some(j) = joined {
                    self.env = j;
                }
                tail
            }
            Statement::For { pattern, iterable, body, .. } => {
                let it = self.expr(iterable);
                self.run_loop(body, &mut |w| w.bind(pattern, &it));
                Taint::new()
            }
            Statement::While { condition, body, .. } => {
                self.run_loop(body, &mut |w| {
                    w.expr(condition);
                });
                Taint::new()
            }
            Statement::Loop { body, .. } => {
                self.run_loop(body, &mut |_| {});
                Taint::new()
            }
            Statement::Defer { statement, .. } => {
                self.stmt(statement);
                Taint::new()
            }
            Statement::Break { value: Some(v), .. } => {
                self.expr(v);
                Taint::new()
            }
            Statement::Const { .. } | Statement::Static { .. } | Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => Taint::new(),
        }
    }

    /// Walk the sub-expressions of a place expression (index operands) for the calls in them.
    fn sub_exprs(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Index { object, index, .. } => {
                self.sub_exprs(object);
                self.expr(index);
            }
            Expression::FieldAccess { object, .. } => self.sub_exprs(object),
            Expression::MethodCall { .. } | Expression::Call { .. } => {
                self.expr(e);
            }
            _ => {}
        }
    }

    /// `db::connect(..)`, possibly through `?`, `.unwrap()` or `.expect(..)`.
    fn is_connect(&self, e: &Expression<'_>) -> bool {
        match e {
            Expression::TryOp { expr, .. } => self.is_connect(expr),
            Expression::MethodCall { object, method, .. } if matches!(method.as_str(), "unwrap" | "expect") => self.is_connect(object),
            Expression::MethodCall { method, .. } if method == "connect" => {
                let key = ExprKey(e as *const Expression<'_> as usize);
                matches!(self.prog.by_key.get(&key).map(|c| (&c.res, c.kind)), Some((Res::Unresolved(Reason::External(p)), CallKind::Path)) if p.ends_with("db"))
            }
            Expression::Call { function: Expression::Identifier { name, .. }, .. } => {
                matches!(self.std_path_of(e, name).as_deref(), Some(p) if p.ends_with("db::connect"))
            }
            _ => false,
        }
    }

    fn std_path_of(&self, call: &Expression<'_>, name: &str) -> Option<String> {
        let key = ExprKey(call as *const Expression<'_> as usize);
        let c = self.prog.by_key.get(&key)?;
        match &c.res {
            Res::Unresolved(Reason::External(p)) | Res::Prim(p) => Some(full(p, name)),
            Res::Unresolved(Reason::NotFound(_)) if c.kind == CallKind::Free => Some(name.to_string()),
            _ => None,
        }
    }

    fn cap(&self, t: Taint) -> Taint {
        if t.len() <= MAX_ORIGINS {
            t
        } else {
            bound(t, &self.top)
        }
    }

    fn expr(&mut self, e: &Expression<'_>) -> Taint {
        let t = self.expr_inner(e);
        self.cap(t)
    }

    fn expr_inner(&mut self, e: &Expression<'_>) -> Taint {
        match e {
            Expression::Literal { .. } => Taint::new(),
            Expression::Identifier { name, .. } => self.env.get(name).cloned().unwrap_or_default(),
            Expression::Binary { left, op, right, .. } => {
                let l = self.expr(left);
                let r = self.expr(right);
                match op {
                    BinaryOp::Add => union(l, r),
                    _ => Taint::new(),
                }
            }
            Expression::Unary { op, operand, .. } => {
                let t = self.expr(operand);
                match op {
                    UnaryOp::Not | UnaryOp::Neg => Taint::new(),
                    _ => t,
                }
            }
            Expression::Call { function, arguments, location } => self.call(e, function, arguments, location),
            Expression::MethodCall { object, method, arguments, location, .. } => self.method_call(e, object, method, arguments, location),
            Expression::FieldAccess { object, field, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    if let Some(t) = self.request_member(name, field, e) {
                        return t;
                    }
                }
                let t = self.expr(object);
                select(&t, field)
            }
            Expression::StructLiteral { fields, .. } => {
                let mut out = Taint::new();
                for (n, v) in fields {
                    let t = self.expr(v);
                    out.extend(t.into_iter().map(|o| wrap(n, o)));
                }
                out
            }
            Expression::MapLiteral { pairs, .. } => pairs.iter().fold(Taint::new(), |acc, (k, v)| {
                let a = union(acc, self.expr(k));
                union(a, self.expr(v))
            }),
            Expression::Range { start, end, .. } => union(self.expr(start), self.expr(end)),
            Expression::Closure { parameters, body, .. } => {
                let saved: Vec<_> = parameters.iter().map(|p| (p.clone(), self.env.remove(p))).collect();
                let t = self.expr(body);
                for (p, old) in saved {
                    match old {
                        Some(o) => {
                            self.env.insert(p, o);
                        }
                        None => {
                            self.env.remove(&p);
                        }
                    }
                }
                t
            }
            Expression::Cast { expr, type_, .. } => {
                let t = self.expr(expr);
                if is_scalar(type_) {
                    Taint::new()
                } else {
                    t
                }
            }
            Expression::Index { object, index, .. } => {
                self.expr(index);
                self.expr(object)
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements.iter().fold(Taint::new(), |acc, x| union(acc, self.expr(x))),
            Expression::MacroInvocation { args, .. } => args.iter().fold(Taint::new(), |acc, x| union(acc, self.expr(x))),
            Expression::TryOp { expr, .. } => {
                let t = self.expr(expr);
                select(&t, "Ok")
            }
            Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. } => self.expr(expr),
            Expression::Scope { body, .. } => self.expr(body),
            Expression::Select { arms, .. } => {
                for arm in arms {
                    if let Some(source) = arm.source {
                        self.expr(source);
                    }
                    self.expr(arm.body);
                }
                Taint::new()
            }
            Expression::Block { statements, .. } => {
                self.block(statements)
            }
        }
    }

    /// `req.body`, `req.query`: a member of a value of an HTTP request type (decided by the std struct's fields).
    fn request_member(&mut self, var: &str, member: &str, at: &Expression<'_>) -> Option<Taint> {
        self.request_member_at(var, member, &at.location())
    }

    fn request_member_at(&mut self, var: &str, member: &str, at: &crate::parser::ast::types::SourceLocation) -> Option<Taint> {
        let ty = self.request_types.get(var)?.clone();
        let key = format!("{ty}::{member}");
        let hit = *self.request_cache.entry(key.clone()).or_insert_with(|| crate::ir::taint::taint_source_for_callee(&key).is_some());
        hit.then(|| {
            let kind = crate::ir::taint::taint_source_for_callee(&key);
            let what = format!("an HTTP request ({})", member_what(&kind));
            self.source(&what, &key, at, member)
        })
    }

    fn source(&self, what: &str, callee: &str, loc: &crate::parser::ast::types::SourceLocation, word: &str) -> Taint {
        let mut t = Taint::new();
        t.insert(Origin::Source { what: what.to_string(), callee: callee.to_string(), at: self.place(loc, word) });
        t
    }

    fn args(&mut self, arguments: &[(Option<String>, &Expression<'_>)]) -> Vec<Taint> {
        arguments.iter().map(|(_, a)| self.expr(a)).collect()
    }

    fn call(&mut self, e: &Expression<'_>, function: &Expression<'_>, arguments: &[(Option<String>, &Expression<'_>)], loc: &crate::parser::ast::types::SourceLocation) -> Taint {
        let name = match function {
            Expression::Identifier { name, .. } => name.clone(),
            other => {
                self.expr(other);
                String::new()
            }
        };
        let args = self.args(arguments);
        let key = ExprKey(e as *const Expression<'_> as usize);
        let prog = self.prog;
        let Some(res) = prog.by_key.get(&key).map(|c| (&c.res, c.kind)) else { return args.into_iter().fold(Taint::new(), union) };
        let word = name.rsplit("::").next().unwrap_or(&name).to_string();
        match res.0 {
            Res::Def(id) => self.apply_user(&[*id], None, &args, loc, &word, res.1),
            Res::Unresolved(Reason::External(p)) | Res::Prim(p) => {
                let path = full(p, &name);
                self.std_call(&path, &word, &args, loc, e)
            }
            Res::Unresolved(Reason::NotFound(_)) if res.1 == CallKind::Free => self.std_call(&name, &word, &args, loc, e),
            _ => args.into_iter().fold(Taint::new(), union),
        }
    }

    fn method_call(&mut self, e: &Expression<'_>, object: &Expression<'_>, method: &str, arguments: &[(Option<String>, &Expression<'_>)], loc: &crate::parser::ast::types::SourceLocation) -> Taint {
        // `fs.read_to_string(..)` with `use std::fs` resolves like `fs::read_to_string(..)` (`resolve::walk::module_dot_call`):
        // one classification for both spellings
        let key = ExprKey(e as *const Expression<'_> as usize);
        let prog = self.prog;
        if let Some(c) = prog.by_key.get(&key).filter(|c| c.kind == CallKind::Path) {
            let args = self.args(arguments);
            let word = method.to_string();
            return match &c.res {
                Res::Def(id) => self.apply_user(&[*id], None, &args, loc, &word, CallKind::Path),
                Res::Unresolved(Reason::External(p)) | Res::Prim(p) => self.std_call(&full(p, &c.callee), &word, &args, loc, e),
                _ => args.into_iter().fold(Taint::new(), union),
            };
        }
        let recv = self.expr(object);
        // a closure argument sees the elements of the receiver (`lines.map(|l| ..)`)
        let mut args = Vec::new();
        for (_, a) in arguments {
            if let Expression::Closure { parameters, body, .. } = a {
                let saved: Vec<_> = parameters.iter().map(|p| (p.clone(), self.env.insert(p.clone(), recv.clone()))).collect();
                let t = self.expr(body);
                for (p, old) in saved {
                    match old {
                        Some(o) => {
                            self.env.insert(p, o);
                        }
                        None => {
                            self.env.remove(&p);
                        }
                    }
                }
                args.push(t);
            } else {
                args.push(self.expr(a));
            }
        }
        // SQL text of a database connection
        if let (Some(role), Expression::Identifier { name, .. }) = (catalog::db_method_role(method), object) {
            if self.conns.contains(name) {
                self.check_sink(&role, &format!("Connection::{method}"), method, &args, loc);
                return Taint::new();
            }
        }
        // a member of an HTTP request value
        if let Expression::Identifier { name, .. } = object {
            if let Some(t) = self.request_member(name, method, e) {
                return t;
            }
        }
        let key = ExprKey(e as *const Expression<'_> as usize);
        let prog = self.prog;
        if let Some(c) = prog.by_key.get(&key) {
            if let Res::MethodCandidates(ids) = &c.res {
                let ids = ids.clone();
                return self.apply_user(&ids, Some(recv), &args, loc, method, CallKind::Method);
            }
        }
        if catalog::NARROWING_METHODS.contains(&method) {
            return Taint::new();
        }
        let all = args.into_iter().fold(recv.clone(), union);
        if catalog::UNWRAP_METHODS.contains(&method) {
            return select(&all, "Ok");
        }
        if catalog::STORING_METHODS.contains(&method) {
            if let Some(root) = root_ident(object) {
                let slot = self.env.entry(root).or_default();
                slot.extend(all.clone());
                if slot.len() > MAX_ORIGINS {
                    *slot = bound(std::mem::take(slot), &self.top);
                }
            }
        }
        all
    }

    fn std_call(&mut self, path: &str, word: &str, args: &[Taint], loc: &crate::parser::ast::types::SourceLocation, _e: &Expression<'_>) -> Taint {
        for role in catalog::sink_roles(path) {
            self.check_sink(&role, path, word, args, loc);
        }
        let variant = path.rsplit("::").next().unwrap_or(path);
        if matches!(variant, "Ok" | "Err") {
            let all = args.iter().cloned().fold(Taint::new(), union);
            return all.into_iter().map(|o| wrap(variant, o)).collect();
        }
        if catalog::std_sanitizer(path).is_some() {
            return Taint::new();
        }
        if let Some(src) = catalog::source(path) {
            let t = self.source(src.what, &canonical(path), loc, word);
            if src.result {
                return t.into_iter().map(|o| wrap("Ok", o)).collect();
            }
            return t;
        }
        if crate::ir::taint::is_sanitizer_callee(path) {
            return Taint::new();
        }
        args.iter().cloned().fold(Taint::new(), union)
    }

    fn check_sink(&mut self, role: &SinkRole, callee: &str, word: &str, args: &[Taint], loc: &crate::parser::ast::types::SourceLocation) {
        let Some(t) = args.get(role.arg).map(flatten) else { return };
        let at = self.place(loc, word);
        let site = SinkSite { role: role.clone(), callee: canonical(callee), at: at.clone() };
        for o in &t {
            match o {
                Origin::Source { what, callee: sc, at: sat } => self.findings.push(Finding {
                    source_what: what.clone(),
                    source_callee: sc.clone(),
                    source_at: sat.clone(),
                    sink: site.clone(),
                    report_at: at.clone(),
                    via: Vec::new(),
                }),
                Origin::Param(i, path) => {
                    self.sinks.entry((*i, path.clone())).or_default().insert(site.clone());
                }
                Origin::Field(..) => {}
            }
        }
    }

    fn apply_user(&mut self, ids: &[DefId], recv: Option<Taint>, args: &[Taint], loc: &crate::parser::ast::types::SourceLocation, word: &str, kind: CallKind) -> Taint {
        let (prog, summaries) = (self.prog, self.summaries);
        let defs = prog.resolved.defs();
        let mut result = Taint::new();
        for id in ids {
            let Some(sum) = summaries.get(id) else { continue };
            let d = defs.get(*id);
            let off = usize::from(d.has_self && kind == CallKind::Method);
            let param = |p: usize| -> Taint {
                if kind == CallKind::Method && d.has_self && p == 0 {
                    return recv.clone().unwrap_or_default();
                }
                args.get(p - off).cloned().unwrap_or_default()
            };
            for o in &sum.ret {
                match o {
                    Origin::Param(p, path) => result.extend(select_path(&param(*p), path)),
                    Origin::Field(n, inner) => {
                        // a struct built in the callee: its field holds what the inner origin resolves to
                        let mut one = Taint::new();
                        one.insert((**inner).clone());
                        let resolved = self.resolve_in_callee(&one, &param);
                        result.extend(resolved.into_iter().map(|r| wrap(n, r)));
                    }
                    src => {
                        result.insert(src.clone());
                    }
                }
            }
            if ids.len() == 1 {
                for ((p, path), sites) in &sum.sinks {
                    let t = flatten(&select_path(&param(*p), path));
                    for o in t {
                        match o {
                            Origin::Source { what, callee, at } => {
                                for s in sites {
                                    self.findings.push(Finding {
                                        source_what: what.clone(),
                                        source_callee: callee.clone(),
                                        source_at: at.clone(),
                                        sink: s.clone(),
                                        report_at: self.place(loc, word),
                                        via: vec![d.name.clone()],
                                    });
                                }
                            }
                            Origin::Param(q, qpath) => {
                                self.sinks.entry((q, qpath)).or_default().extend(sites.iter().cloned());
                            }
                            Origin::Field(..) => {}
                        }
                    }
                }
            }
        }
        result
    }
}

impl<'w, 'p, 'a> Walker<'w, 'p, 'a> {
    /// A callee origin (possibly a parameter's member) as it reads in the caller.
    fn resolve_in_callee(&self, one: &Taint, param: &dyn Fn(usize) -> Taint) -> Taint {
        one.iter()
            .flat_map(|o| match o {
                Origin::Param(p, path) => select_path(&param(*p), path),
                other => [other.clone()].into_iter().collect(),
            })
            .collect()
    }
}

fn union(mut a: Taint, b: Taint) -> Taint {
    a.extend(b);
    a
}

fn root_ident(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => root_ident(object),
        _ => None,
    }
}

fn full(external: &str, callee: &str) -> String {
    let last = callee.rsplit("::").next().unwrap_or(callee);
    if external == last || external.ends_with(&format!("::{last}")) {
        external.to_string()
    } else {
        format!("{external}::{last}")
    }
}

/// `std::fs::read_to_string` -> `fs::read_to_string`.
fn canonical(path: &str) -> String {
    let segs: Vec<&str> = path.split("::").collect();
    segs[segs.len().saturating_sub(2)..].join("::")
}

fn member_what(kind: &Option<crate::ir::taint::TaintSourceKind>) -> &'static str {
    use crate::ir::taint::TaintSourceKind as K;
    match kind {
        Some(K::HttpRequestBody) => "body",
        Some(K::HttpRequestQuery) => "query",
        Some(K::HttpRequestHeader) => "headers",
        _ => "request",
    }
}


/// Visit every statement and every expression of `stmts` (nested blocks, branches, loops, closures, arguments, interpolation parts).
pub(super) fn scan(stmts: &[&Statement<'_>], on_stmt: &mut dyn FnMut(&Statement<'_>), on_expr: &mut dyn FnMut(&Expression<'_>)) {
    for st in stmts {
        scan_stmt(st, on_stmt, on_expr);
    }
}

fn scan_stmt(st: &Statement<'_>, on_stmt: &mut dyn FnMut(&Statement<'_>), on_expr: &mut dyn FnMut(&Expression<'_>)) {
    on_stmt(st);
    match st {
        Statement::Let { value, else_block, .. } => {
            scan_expr(value, on_stmt, on_expr);
            if let Some(b) = else_block {
                scan(b, on_stmt, on_expr);
            }
        }
        Statement::Const { value, .. } | Statement::Static { value, .. } => scan_expr(value, on_stmt, on_expr),
        Statement::Assignment { target, value, .. } => {
            scan_expr(target, on_stmt, on_expr);
            scan_expr(value, on_stmt, on_expr);
        }
        Statement::Return { value, .. } => {
            if let Some(v) = value {
                scan_expr(v, on_stmt, on_expr);
            }
        }
        Statement::Expression { expr, .. } => scan_expr(expr, on_stmt, on_expr),
        Statement::If { condition, then_block, else_block, .. } => {
            scan_expr(condition, on_stmt, on_expr);
            scan(then_block, on_stmt, on_expr);
            if let Some(b) = else_block {
                scan(b, on_stmt, on_expr);
            }
        }
        Statement::Match { value, arms, .. } => {
            scan_expr(value, on_stmt, on_expr);
            for a in arms {
                if let Some(g) = a.guard {
                    scan_expr(g, on_stmt, on_expr);
                }
                scan_expr(a.body, on_stmt, on_expr);
            }
        }
        Statement::For { iterable, body, .. } => {
            scan_expr(iterable, on_stmt, on_expr);
            scan(body, on_stmt, on_expr);
        }
        Statement::While { condition, body, .. } => {
            scan_expr(condition, on_stmt, on_expr);
            scan(body, on_stmt, on_expr);
        }
        Statement::Loop { body, .. } => scan(body, on_stmt, on_expr),
        Statement::Defer { statement, .. } => scan_stmt(statement, on_stmt, on_expr),
        Statement::Break { value: Some(v), .. } => scan_expr(v, on_stmt, on_expr),
        Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => {}
    }
}

fn scan_expr(e: &Expression<'_>, on_stmt: &mut dyn FnMut(&Statement<'_>), on_expr: &mut dyn FnMut(&Expression<'_>)) {
    on_expr(e);
    match e {
        Expression::Literal { .. } | Expression::Identifier { .. } => {}
        Expression::Binary { left, right, .. } => {
            scan_expr(left, on_stmt, on_expr);
            scan_expr(right, on_stmt, on_expr);
        }
        Expression::Unary { operand, .. } => scan_expr(operand, on_stmt, on_expr),
        Expression::Call { function, arguments, .. } => {
            scan_expr(function, on_stmt, on_expr);
            arguments.iter().for_each(|(_, a)| scan_expr(a, on_stmt, on_expr));
        }
        Expression::MethodCall { object, arguments, .. } => {
            scan_expr(object, on_stmt, on_expr);
            arguments.iter().for_each(|(_, a)| scan_expr(a, on_stmt, on_expr));
        }
        Expression::FieldAccess { object, .. } => scan_expr(object, on_stmt, on_expr),
        Expression::StructLiteral { fields, .. } => fields.iter().for_each(|(_, v)| scan_expr(v, on_stmt, on_expr)),
        Expression::MapLiteral { pairs, .. } => pairs.iter().for_each(|(k, v)| {
            scan_expr(k, on_stmt, on_expr);
            scan_expr(v, on_stmt, on_expr);
        }),
        Expression::Range { start, end, .. } => {
            scan_expr(start, on_stmt, on_expr);
            scan_expr(end, on_stmt, on_expr);
        }
        Expression::Closure { body, .. } => scan_expr(body, on_stmt, on_expr),
        Expression::Cast { expr, .. }
        | Expression::TryOp { expr, .. }
        | Expression::Await { expr, .. }
        | Expression::AsyncCall { expr, .. }
        | Expression::SpawnCall { expr, .. } => scan_expr(expr, on_stmt, on_expr),
        Expression::Index { object, index, .. } => {
            scan_expr(object, on_stmt, on_expr);
            scan_expr(index, on_stmt, on_expr);
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements.iter().for_each(|x| scan_expr(x, on_stmt, on_expr)),
        Expression::MacroInvocation { args, .. } => args.iter().for_each(|x| scan_expr(x, on_stmt, on_expr)),
        Expression::Scope { body, .. } => scan_expr(body, on_stmt, on_expr),
        Expression::Select { arms, .. } => {
            for arm in arms {
                if let Some(source) = arm.source {
                    scan_expr(source, on_stmt, on_expr);
                }
                scan_expr(arm.body, on_stmt, on_expr);
            }
        }
        Expression::Block { statements, .. } => scan(statements, on_stmt, on_expr),
    }
}

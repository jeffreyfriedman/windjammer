//! Last-use fact (convergence plan C6, ADR-0017 `last_use(binding, site)`).
//!
//! A read of a local is its *last use* when no later mention of that name exists on any path:
//! there is no textually later mention in the function, the read is not inside a loop that the
//! binding outlives (the next iteration would read it again), and it is not inside a closure.
//! Textual order is a sound over-approximation of "later on some path" for structured control
//! flow (branches only make the answer more conservative).
//!
//! A read is also not last while a binding derived from it is still live: `let r = f(&v)`
//! (or a match/for pattern over an expression mentioning `v`) may hold a borrow of `v`, so a
//! read of `v` followed by a later mention of `r` must not move `v` (E0505).
//!
//! Data only. Keyed by AST node identity ([`node_id`]). Consumers: backends move instead of
//! copying/cloning at a last use (MOVE-LAST-USE): the Rust backend drops its reuse clone there,
//! the portable backends elide the value copy.
//!
//! Mutation hook (gate self-test only): `always_move` marks every read a last use
//! (`CompilerConfig::liveness_mutation_always_move`); `tests/inference_differential_test.rs`
//! proves the oracles reject the result.

use super::numeric_defaults::node_id;
use crate::parser::ast::core::{Expression, Pattern, Statement};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
struct Mention {
    node: Option<usize>,
    /// Index into `Collector::branch_paths`: the `if`/`match` arms enclosing the mention.
    branch: usize,
    /// Textual order of the mention in the function.
    seq: usize,
    /// Loop nesting path (ids of enclosing loops) at the mention.
    loop_depth_id: usize,
    in_closure: bool,
}

#[derive(Default)]
struct Collector {
    /// Per name, mentions in textual order.
    mentions: BTreeMap<String, Vec<Mention>>,
    /// Per name, the loop id the (latest) binding was declared in (0 = function body).
    declared_in: BTreeMap<String, usize>,
    /// Stack of loop ids; each loop gets a fresh id. `loop_parents[id]` is its enclosing loop.
    loop_stack: Vec<usize>,
    loop_parents: Vec<usize>,
    closure_depth: usize,
    seq: usize,
    /// Names mentioned while evaluating the current `let`/`match`/`for` source expression.
    sources: Option<BTreeSet<String>>,
    /// Per binding, the names its initializer mentioned (it may borrow from them).
    derived_from: BTreeMap<String, BTreeSet<String>>,
    /// Per visited statement, in visiting order.
    spans: Vec<Span>,
    /// Interned `(branch group, arm)` paths; `branch_paths[0]` is the empty path.
    branch_paths: Vec<Vec<(usize, usize)>>,
    branch_now: usize,
    branch_groups: usize,
    /// Names bound by a plain `let x` / `let mut x` (owned values in every backend).
    let_bound: BTreeSet<String>,
    /// Per loop id, the (first, last) mention seq of its body.
    loop_ranges: BTreeMap<usize, (usize, usize)>,
}

/// A statement as `read_after` sees it.
struct Span {
    /// Statement node identity (the statement's address in the analyzed AST).
    node: usize,
    /// Mention seq range of the statement.
    start: usize,
    end: usize,
    loops: Vec<usize>,
    branch: usize,
    /// A `return`: nothing after it runs.
    diverges: bool,
    /// Names a `let` statement binds: later reads see the new binding.
    binds: Vec<String>,
}

fn stmt_id(s: &Statement<'_>) -> usize {
    s as *const Statement<'_> as *const () as usize
}

impl Collector {
    fn current_loop(&self) -> usize {
        self.loop_stack.last().copied().unwrap_or(0)
    }

    fn mention(&mut self, name: &str, node: Option<usize>) {
        self.seq += 1;
        if node.is_some() {
            if let Some(src) = self.sources.as_mut() {
                src.insert(name.to_string());
            }
        }
        let m = Mention { node, branch: self.branch_now, seq: self.seq, loop_depth_id: self.current_loop(), in_closure: self.closure_depth > 0 };
        self.mentions.entry(name.to_string()).or_default().push(m);
    }

    fn declare_pattern(&mut self, p: &Pattern<'_>) {
        let lp = self.current_loop();
        let mut names = Vec::new();
        pattern_names(p, &mut names);
        for n in names {
            self.declared_in.insert(n.clone(), lp);
            // A (re)binding is a write: earlier reads are not "last" relative to it, but it
            // also starts a new value, so record it as a mention without a node.
            self.mention(&n, None);
        }
    }

    /// Evaluate `e` and return the names it mentions (the sources a binding may borrow from).
    fn source_expr(&mut self, e: &Expression<'_>) -> BTreeSet<String> {
        let outer = self.sources.replace(BTreeSet::new());
        self.expr(e);
        let mine = std::mem::replace(&mut self.sources, outer).unwrap_or_default();
        if let Some(o) = self.sources.as_mut() {
            o.extend(mine.iter().cloned());
        }
        mine
    }

    /// Visit with derivation tracking off: the value built here owns what it reads.
    fn without_sources(&mut self, f: impl FnOnce(&mut Self)) {
        let sources = self.sources.take();
        f(self);
        self.sources = sources;
    }

    /// Visit one arm of an `if`/`match` (arms of one group never run in the same pass).
    fn in_arm(&mut self, group: usize, arm: usize, f: impl FnOnce(&mut Self)) {
        let outer = self.branch_now;
        if self.branch_paths.is_empty() {
            self.branch_paths.push(Vec::new());
        }
        let mut path = self.branch_paths[outer].clone();
        path.push((group, arm));
        self.branch_paths.push(path);
        self.branch_now = self.branch_paths.len() - 1;
        f(self);
        self.branch_now = outer;
    }

    fn new_group(&mut self) -> usize {
        self.branch_groups += 1;
        self.branch_groups
    }

    /// Mentions on paths `a` and `b` are in different arms of one `if`/`match`.
    fn exclusive(&self, a: usize, b: usize) -> bool {
        let (Some(pa), Some(pb)) = (self.branch_paths.get(a), self.branch_paths.get(b)) else { return false };
        pa.iter().any(|(g, x)| pb.iter().any(|(h, y)| g == h && x != y))
    }

    fn derive(&mut self, p: &Pattern<'_>, sources: &BTreeSet<String>) {
        let mut names = Vec::new();
        pattern_names(p, &mut names);
        for n in names {
            self.derived_from.entry(n).or_default().extend(sources.iter().cloned());
        }
    }

    fn enter_loop(&mut self) {
        let id = self.loop_parents.len() + 1;
        self.loop_parents.push(self.current_loop());
        self.loop_stack.push(id);
        self.loop_ranges.insert(id, (self.seq + 1, usize::MAX));
    }

    fn exit_loop(&mut self) {
        if let Some(id) = self.loop_stack.pop() {
            if let Some(r) = self.loop_ranges.get_mut(&id) {
                r.1 = self.seq;
            }
        }
    }

    /// Is loop `inner` (or a loop nested in it) inside loop `outer`'s body, i.e. does a binding
    /// declared in `outer` outlive an iteration of `inner`?
    fn loop_inside(&self, mut inner: usize, outer: usize) -> bool {
        while inner != 0 {
            if inner == outer {
                return false;
            }
            let parent = self.loop_parents[inner - 1];
            if parent == outer {
                return true;
            }
            inner = parent;
        }
        false
    }

    fn stmts(&mut self, body: &[&Statement<'_>]) {
        for s in body {
            let start = self.seq;
            self.stmt(s);
            let mut binds = Vec::new();
            if let Statement::Let { pattern, .. } = s {
                pattern_names(pattern, &mut binds);
            }
            self.spans.push(Span {
                node: stmt_id(s),
                start,
                end: self.seq,
                loops: self.loop_stack.clone(),
                branch: self.branch_now,
                diverges: matches!(s, Statement::Return { .. }),
                binds,
            });
        }
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        match s {
            Statement::Let { pattern, value, else_block, .. } => {
                let src = self.source_expr(value);
                if let Some(b) = else_block {
                    self.stmts(b);
                }
                self.declare_pattern(pattern);
                self.derive(pattern, &src);
                if let Pattern::Identifier(n) | Pattern::MutBinding(n) = pattern {
                    self.let_bound.insert(n.clone());
                }
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => self.expr(value),
            Statement::Assignment { target, value, .. } => {
                self.expr(value);
                self.expr(target);
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition);
                let g = self.new_group();
                self.in_arm(g, 0, |c| c.stmts(then_block));
                if let Some(b) = else_block {
                    self.in_arm(g, 1, |c| c.stmts(b));
                }
            }
            Statement::Match { value, arms, .. } => {
                let src = self.source_expr(value);
                let g = self.new_group();
                for (i, arm) in arms.iter().enumerate() {
                    self.in_arm(g, i, |c| {
                        c.declare_pattern(&arm.pattern);
                        c.derive(&arm.pattern, &src);
                        if let Some(guard) = arm.guard {
                            c.expr(guard);
                        }
                        c.expr(arm.body);
                    });
                }
            }
            Statement::For { pattern, iterable, body, .. } => {
                let src = self.source_expr(iterable);
                self.enter_loop();
                self.declare_pattern(pattern);
                self.derive(pattern, &src);
                self.stmts(body);
                self.exit_loop();
            }
            Statement::While { condition, body, .. } => {
                self.enter_loop();
                self.expr(condition);
                self.stmts(body);
                self.exit_loop();
            }
            Statement::Loop { body, .. } => {
                self.enter_loop();
                self.stmts(body);
                self.exit_loop();
            }
            // Threads/async bodies may run later or concurrently: treat like closures.
            Statement::Thread { body, .. } | Statement::Async { body, .. } => {
                self.closure_depth += 1;
                self.stmts(body);
                self.closure_depth -= 1;
            }
            Statement::Defer { statement, .. } => {
                self.closure_depth += 1;
                self.stmt(statement);
                self.closure_depth -= 1;
            }
            Statement::Break { value, .. } => {
                if let Some(v) = value {
                    self.expr(v);
                }
            }
            Statement::Continue { .. } | Statement::Use { .. } => {}
        }
    }

    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Identifier { name, .. } => self.mention(name, Some(node_id(e))),
            Expression::Closure { body, parameters, .. } => {
                self.closure_depth += 1;
                for name in parameters.iter().flat_map(|p| closure_param_names(p)) {
                    self.declared_in.insert(name, self.current_loop());
                }
                self.expr(body);
                self.closure_depth -= 1;
            }
            Expression::Block { statements, .. } => self.stmts(statements),
            Expression::Literal { .. } => {}
            Expression::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expression::Unary { operand, .. } => self.expr(operand),
            Expression::Call { function, arguments, .. } => {
                // A callee name is not a value read, unless it names a local (a closure binding).
                match function {
                    Expression::Identifier { name, .. } if !self.declared_in.contains_key(name) => {}
                    _ => self.expr(function),
                }
                // RET-NO-BORROW: a function result never borrows from its arguments, so a
                // binding initialised by a call does not keep the arguments alive.
                self.without_sources(|c| arguments.iter().for_each(|(_, a)| c.expr(a)));
            }
            Expression::MethodCall { object, arguments, .. } => {
                self.expr(object);
                // A method result may borrow from its receiver, never from its arguments.
                self.without_sources(|c| arguments.iter().for_each(|(_, a)| c.expr(a)));
            }
            Expression::FieldAccess { object, .. } => self.expr(object),
            // No reference fields (RET-NO-BORROW): a struct value never borrows its field sources.
            Expression::StructLiteral { fields, .. } => self.without_sources(|c| fields.iter().for_each(|(_, f)| c.expr(f))),
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
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                elements.iter().for_each(|x| self.expr(x))
            }
            // `vec![..]` / `format!(..)` build owned values from their arguments.
            Expression::MacroInvocation { args, .. } => self.without_sources(|c| args.iter().for_each(|x| c.expr(x))),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr(channel);
                self.expr(value);
            }
            Expression::ChannelRecv { channel, .. } => self.expr(channel),
        }
    }
}

fn pattern_names(p: &Pattern<'_>, out: &mut Vec<String>) {
    use crate::parser::ast::core::EnumPatternBinding as B;
    match p {
        Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) | Pattern::RefMut(n) => out.push(n.clone()),
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|p| pattern_names(p, out)),
        Pattern::Reference(p) => pattern_names(p, out),
        Pattern::EnumVariant(_, b) => match b {
            B::Single(n) => out.push(n.clone()),
            B::Tuple(ps) => ps.iter().for_each(|p| pattern_names(p, out)),
            B::Struct(fs, _) => fs.iter().for_each(|(_, p)| pattern_names(p, out)),
            B::None | B::Wildcard => {}
        },
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
    }
}

/// Identifier reads in `body` that are the last use of their binding, by [`node_id`].
/// `entry` are the bindings declared at function entry (outside every loop). Value-semantics
/// backends use this directly (a copy at a last use is unobservable).
pub fn last_uses(entry: &[&str], body: &[&Statement<'_>]) -> BTreeSet<usize> {
    let c = collect(entry, body);
    last_reads(&c, false).iter().filter_map(|m| m.node).collect()
}

/// MOVE-LAST-USE for an ownership backend: [`last_uses`] restricted to parameters and plain `let`
/// bindings (owned values; a match/for pattern binding may be a reference), and not while a
/// binding derived from the value may still borrow it (E0505). `always_move` is the gate
/// self-test mutation: every read is last.
pub fn last_moves(entry: &[&str], body: &[&Statement<'_>], always_move: bool) -> BTreeSet<usize> {
    let c = collect(entry, body);
    if always_move {
        return c.mentions.values().flatten().filter_map(|m| m.node).collect();
    }
    last_reads(&c, true)
        .iter()
        .filter_map(|m| m.node)
        .collect()
}

fn collect(entry: &[&str], body: &[&Statement<'_>]) -> Collector {
    let mut c = Collector::default();
    for p in entry {
        c.declared_in.insert(p.to_string(), 0);
        c.let_bound.insert(p.to_string());
    }
    c.stmts(body);
    c
}

fn last_reads(c: &Collector, owned_only: bool) -> Vec<Mention> {
    let mut out = Vec::new();
    for (name, ms) in &c.mentions {
        let Some(last) = ms.last() else { continue };
        if last.node.is_none() {
            continue;
        }
        // A closure may capture the binding and run after the outer "last" read.
        if ms.iter().any(|m| m.in_closure) {
            continue;
        }
        // Unknown declaration (a global or a name bound in a construct we do not model): skip.
        let Some(&decl_loop) = c.declared_in.get(name) else { continue };
        if last.loop_depth_id != decl_loop && c.loop_inside(last.loop_depth_id, decl_loop) {
            continue;
        }
        if last.loop_depth_id != decl_loop && !c.loop_inside(last.loop_depth_id, decl_loop) && decl_loop != 0 {
            // Declared in a loop that does not enclose the use (shadowing across scopes): skip.
            continue;
        }
        if owned_only && (!c.let_bound.contains(name) || borrowed_by_later_live_binding(c, name, last.seq)) {
            continue;
        }
        out.push(*last);
    }
    out
}

/// Bindings read exactly once inside `stmt`, where that read is their last use (`last` is the
/// result of [`last_moves`] for the enclosing function). A consumer emitting `stmt` may move them.
pub fn dying_in(stmt: &Statement<'_>, last: &BTreeSet<usize>) -> BTreeSet<String> {
    let mut c = Collector::default();
    c.stmt(stmt);
    c.mentions
        .iter()
        .filter_map(|(name, ms)| {
            let mut reads = ms.iter().filter_map(|m| m.node);
            let only = reads.next()?;
            (reads.next().is_none() && last.contains(&only)).then(|| name.clone())
        })
        .collect()
}

/// Per statement (by node identity), the bindings read again after it: later on a path that can
/// follow it (not in a sibling `if`/`match` arm, not after a `return`, not after the statement
/// rebinds the name), in the next iteration of an enclosing loop the binding outlives, or through
/// a later read of a binding derived from it (a closure that captured it, a borrow). A consumer
/// that moves a binding in the statement must clone it when it is listed. `always_move` is the
/// gate self-test mutation: nothing is ever read again.
pub fn read_after(entry: &[&str], body: &[&Statement<'_>], always_move: bool) -> BTreeMap<usize, BTreeSet<String>> {
    let c = collect(entry, body);
    let mut out: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for sp in &c.spans {
        let names = out.entry(sp.node).or_default();
        if always_move || sp.diverges {
            continue;
        }
        for (name, ms) in &c.mentions {
            let rebound_here = sp.binds.contains(name);
            let later = !rebound_here
                && ms
                    .iter()
                    .filter(|m| m.seq > sp.end && !c.exclusive(m.branch, sp.branch))
                    .find(|_| true)
                    .is_some_and(|m| m.node.is_some());
            let decl = c.declared_in.get(name).copied().unwrap_or(0);
            let carried = sp.loops.iter().any(|l| {
                let outlives = decl != *l && !c.loop_inside(decl, *l);
                let (lo, hi) = c.loop_ranges.get(l).copied().unwrap_or((0, 0));
                outlives && ms.iter().any(|m| m.node.is_some() && m.seq >= lo && m.seq <= hi)
            });
            // A binding derived from `name` (a closure capturing it, a borrow) read later keeps it live.
            if later || carried || (!rebound_here && borrowed_by_later_live_binding(&c, name, sp.end)) {
                names.insert(name.clone());
            }
        }
        let _ = sp.start;
    }
    out
}

/// Names bound by a closure parameter as written: `x`, `x: int`, `(k, v)`, `mut acc`.
fn closure_param_names(p: &str) -> Vec<String> {
    let pattern = p.split(':').next().unwrap_or(p);
    pattern
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .filter(|n| !n.is_empty() && *n != "mut" && *n != "_")
        .map(String::from)
        .collect()
}

/// Bindings a closure body reads but does not bind itself: its captures (`self` excluded).
/// `params` are the closure's parameters as written (`x` or `x: int`).
pub fn captures(params: &[String], body: &Expression<'_>) -> BTreeSet<String> {
    let mut c = Collector::default();
    for name in params.iter().flat_map(|p| closure_param_names(p)) {
        c.declared_in.insert(name, 0);
    }
    c.expr(body);
    c.mentions
        .iter()
        .filter(|(n, ms)| *n != "self" && !c.declared_in.contains_key(*n) && ms.iter().any(|m| m.node.is_some()))
        .map(|(n, _)| n.clone())
        .collect()
}

/// A binding derived (transitively) from `name` is mentioned after `seq`: it may hold a
/// borrow of `name`, so `name` must stay alive.
fn borrowed_by_later_live_binding(c: &Collector, name: &str, seq: usize) -> bool {
    let mut seen = BTreeSet::new();
    let mut work = vec![name.to_string()];
    while let Some(src) = work.pop() {
        for (binding, froms) in &c.derived_from {
            if binding != name && froms.contains(&src) && seen.insert(binding.clone()) {
                if c.mentions.get(binding).is_some_and(|ms| ms.iter().any(|m| m.seq > seq)) {
                    return true;
                }
                work.push(binding.clone());
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::literals::Literal;

    fn marks(src: &str) -> Vec<String> {
        marks_with(src, false)
    }

    /// `owned`: the ownership-backend fact ([`last_moves`]) instead of [`last_uses`].
    fn marks_with(src: &str, owned: bool) -> Vec<String> {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == "main" => Some(decl),
                _ => None,
            })
            .unwrap();
        let params: Vec<&str> = decl.parameters.iter().map(|p| p.name.as_str()).collect();
        let set = if owned { last_moves(&params, &decl.body, false) } else { last_uses(&params, &decl.body) };
        let mut out = Vec::new();
        fn e(x: &Expression<'_>, set: &BTreeSet<usize>, out: &mut Vec<String>) {
            match x {
                Expression::Identifier { name, .. } => {
                    out.push(format!("{}{}", name, if set.contains(&node_id(x)) { "!" } else { "" }))
                }
                Expression::Call { arguments, .. } => arguments.iter().for_each(|(_, a)| e(a, set, out)),
                Expression::MethodCall { object, arguments, .. } => {
                    e(object, set, out);
                    arguments.iter().for_each(|(_, a)| e(a, set, out));
                }
                Expression::MacroInvocation { args, .. } => args.iter().for_each(|a| e(a, set, out)),
                Expression::Binary { left, right, .. } => {
                    e(left, set, out);
                    e(right, set, out);
                }
                Expression::Literal { value: Literal::Int(_), .. } | Expression::Literal { .. } => {}
                _ => {}
            }
        }
        fn s(x: &Statement<'_>, set: &BTreeSet<usize>, out: &mut Vec<String>) {
            match x {
                Statement::Let { value, .. } => e(value, set, out),
                Statement::Expression { expr, .. } => e(expr, set, out),
                Statement::For { iterable, body, .. } => {
                    e(iterable, set, out);
                    body.iter().for_each(|b| s(b, set, out));
                }
                Statement::While { condition, body, .. } => {
                    e(condition, set, out);
                    body.iter().for_each(|b| s(b, set, out));
                }
                Statement::If { condition, then_block, else_block, .. } => {
                    e(condition, set, out);
                    then_block.iter().for_each(|b| s(b, set, out));
                    if let Some(b) = else_block {
                        b.iter().for_each(|b| s(b, set, out));
                    }
                }
                _ => {}
            }
        }
        decl.body.iter().for_each(|x| s(x, &set, &mut out));
        out
    }

    #[test]
    fn last_textual_read_is_last_use() {
        let m = marks("fn keep(v: Vec<int>) -> Vec<int> { v }\nfn main() {\n let v = vec![1]\n println(\"${v.len()}\")\n let w = keep(v)\n println(\"${w.len()}\")\n}\n");
        assert_eq!(m, vec!["v", "v!", "w!"]);
    }

    #[test]
    fn a_read_inside_a_loop_the_binding_outlives_is_never_last() {
        let m = marks("fn take(v: Vec<int>) {}\nfn main() {\n let v = vec![1]\n for i in 0..3 {\n take(v)\n }\n}\n");
        assert_eq!(m, vec!["v"]);
        let m = marks("fn take(v: Vec<int>) {}\nfn main() {\n for i in 0..3 {\n let v = vec![i]\n take(v)\n }\n}\n");
        assert_eq!(m, vec!["i!", "v!"]);
    }

    #[test]
    fn branches_are_conservative() {
        let m = marks("fn take(v: Vec<int>) {}\nfn main() {\n let v = vec![1]\n let c = true\n if c {\n take(v)\n } else {\n take(v)\n }\n}\n");
        // The then-branch read is followed textually by the else read: not last (conservative).
        assert_eq!(m, vec!["c!", "v", "v!"]);
    }

    #[test]
    fn a_read_borrowed_by_a_later_live_binding_is_not_last() {
        // `first` may borrow `v`: moving `v` at `take(v)` would be E0505.
        let m = marks_with("fn take(v: Vec<int>) {}\nfn main() {\n let v = vec![1]\n let first = v.first()\n take(v)\n println(\"${first}\")\n}\n", true);
        assert_eq!(m, vec!["v", "v", "first!"]);
        // The derived binding is dead before the move: the move is fine.
        let m = marks_with("fn take(v: Vec<int>) {}\nfn main() {\n let v = vec![1]\n let n = v.len()\n println(\"${n}\")\n take(v)\n}\n", true);
        assert_eq!(m, vec!["v", "n!", "v!"]);
    }

    #[test]
    fn a_call_result_does_not_borrow_its_arguments() {
        let m = marks_with("fn wrap(v: Vec<int>) -> int { 1 }\nfn main() {\n let v = vec![1]\n let h = wrap(v)\n println(\"${h}\")\n}\n", true);
        assert_eq!(m, vec!["v!", "h!"]);
    }

    #[test]
    fn dying_in_names_the_single_last_read_of_a_statement() {
        let src = "fn take(v: Vec<int>) {}\nfn main() {\n let v = vec![1]\n take(v)\n take(v)\n}\n";
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == "main" => Some(decl),
                _ => None,
            })
            .unwrap();
        let last = last_moves(&[], &decl.body, false);
        assert!(dying_in(decl.body[1], &last).is_empty());
        assert_eq!(dying_in(decl.body[2], &last).into_iter().collect::<Vec<_>>(), vec!["v".to_string()]);
    }

    #[test]
    fn read_after_sees_later_reads_and_the_next_loop_iteration() {
        let src = "fn take(v: Vec<int>) {}\nfn main() {\n let v = vec![1]\n let w = vec![2]\n take(v)\n for i in 0..2 {\n  take(w)\n }\n println(\"${v.len()}\")\n}\n";
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == "main" => Some(decl),
                _ => None,
            })
            .unwrap();
        let ra = read_after(&[], &decl.body, false);
        let at = |s: &Statement<'_>| ra.get(&stmt_id(s)).cloned().unwrap_or_default();
        assert!(at(decl.body[2]).contains("v"), "v is printed later");
        let Statement::For { body, .. } = decl.body[3] else { panic!() };
        assert!(at(body[0]).contains("w"), "w is moved again in the next iteration");
        assert!(!at(decl.body[4]).contains("v"), "nothing reads v after the print");
    }

    #[test]
    fn read_after_ignores_sibling_arms_returns_and_rebinding() {
        let src = "fn take(v: Vec<int>) -> Vec<int> { v }\nfn main() {\n let v = vec![1]\n let c = true\n if c {\n  take(v)\n } else {\n  take(v)\n }\n if c {\n  return\n }\n let v = take(v)\n println(\"${v.len()}\")\n}\n";
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == "main" => Some(decl),
                _ => None,
            })
            .unwrap();
        let ra = read_after(&[], &decl.body, false);
        let at = |s: &Statement<'_>| ra.get(&stmt_id(s)).cloned().unwrap_or_default();
        let Statement::If { then_block, .. } = decl.body[2] else { panic!() };
        // The else-arm read is exclusive with the then-arm; the read after the `if` is not.
        assert!(at(then_block[0]).contains("v"), "v is read after the if");
        let Statement::If { then_block: ret, .. } = decl.body[3] else { panic!() };
        assert!(at(ret[0]).is_empty(), "nothing runs after a return");
        // `let v = take(v)` rebinds v: the later read is of the new binding.
        assert!(!at(decl.body[4]).contains("v"));
        let ra2 = read_after(&[], &decl.body, true);
        assert!(ra2.values().all(|s| s.is_empty()), "the mutation empties the fact");
    }

    #[test]
    fn read_after_follows_captures_not_closure_parameters() {
        let src = "fn take(v: Vec<int>) {}\nfn main() {\n let v = vec![1]\n let f = |i: int| v.len() + i\n take(v)\n println(\"${f(1)}\")\n let w = [vec![1]].map(|t| t.len())\n}\n";
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == "main" => Some(decl),
                _ => None,
            })
            .unwrap();
        let ra = read_after(&[], &decl.body, false);
        let at = |s: &Statement<'_>| ra.get(&stmt_id(s)).cloned().unwrap_or_default();
        assert!(at(decl.body[2]).contains("v"), "the closure `f` captured v and runs later");
        assert!(!at(decl.body[4]).contains("t"), "a closure parameter is not live after the statement");
    }

    #[test]
    fn captures_are_the_free_reads_of_a_closure_body() {
        let src = "fn main() {\n let k = 1\n let f = |x: int, (y, w)| { let z = x + y + w\n z + k + format(\"${k}\").len() }\n}\n";
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == "main" => Some(decl),
                _ => None,
            })
            .unwrap();
        let Statement::Let { value: Expression::Closure { parameters, body, .. }, .. } = decl.body[1] else { panic!() };
        assert_eq!(captures(parameters, body).into_iter().collect::<Vec<_>>(), vec!["k".to_string()]);
    }

    #[test]
    fn closure_reads_are_never_last() {
        let m = marks("fn main() {\n let v = vec![1]\n let f = |i: int| v.len()\n println(\"${f(0)}\")\n}\n");
        assert!(!m.contains(&"v!".to_string()));
    }
}

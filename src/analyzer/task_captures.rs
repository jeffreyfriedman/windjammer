//! What a task may take from the code that starts it (ADR-0004; docs/concurrency/PLAN.md stage 2b). A task works on
//! its own copy of every variable it reads (the compiled backend clones captures into the task, which then runs in
//! parallel), so two things are refused, the same in `wj check`, `wj build` and the interpreter:
//!
//! * WJ0192: a task assigns to a variable of the enclosing code (`spawn { total += 1 }`): the change could never be
//!   seen outside the task, and two tasks would race on it. Return the value (`t.wait()`) or send it on a channel.
//! * WJ0193: a `spawn` task captures a value that cannot move to another thread (built with `Rc::new`, `RefCell::new`,
//!   `Cell::new` or `Weak::new`, or declared with such a type). `async` runs on the same thread and may.
//!
//! The user never writes `Send`: this is the Windjammer face of it. Known gap: a method that changes a captured value
//! (`items.push(x)` inside a task) is not reported yet; it needs the receiver modes of the IR solver.

use super::task_structure::children;
use crate::parser::ast::core::{Expression, Statement};
use crate::parser::*;
use std::collections::{HashMap, HashSet};

pub const SHARED_WRITE: &str = "WJ0192";
pub const NOT_SENDABLE: &str = "WJ0193";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureFinding {
    pub code: &'static str,
    /// Position of the task (`spawn` / `async` keyword), 1-based.
    pub line: usize,
    pub column: usize,
    pub name: String,
    /// WJ0193: what makes the value thread-bound (`Rc`, `RefCell`, ...).
    pub reason: String,
}

impl CaptureFinding {
    pub fn message(&self) -> String {
        if self.code == SHARED_WRITE {
            format!("this task changes `{}`, a variable of the code that started it", self.name)
        } else {
            format!("`{}` cannot move to another thread: it holds a `{}`", self.name, self.reason)
        }
    }

    pub fn help(&self) -> String {
        if self.code == SHARED_WRITE {
            "a task works on its own copy: return the value and read it with `t.wait()`, or send it on a channel".into()
        } else {
            "start the task with `async` (same thread), or build the value inside the task".into()
        }
    }

    pub fn render(&self) -> String {
        format!("error[{}]: {}\n  --> line {}:{}\n  = help: {}", self.code, self.message(), self.line, self.column, self.help())
    }
}

/// Every finding of `program`, in source order.
pub fn check(program: &Program<'_>) -> Vec<CaptureFinding> {
    let mut cx = Cx { thread_bound: HashMap::new(), out: Vec::new() };
    cx.items(&program.items);
    cx.out
}

const THREAD_BOUND: &[&str] = &["Rc", "RefCell", "Cell", "Weak"];

fn thread_bound_type(t: &Type) -> Option<String> {
    match t {
        Type::Custom(n) | Type::Parameterized(n, _) if THREAD_BOUND.contains(&n.as_str()) => Some(n.clone()),
        Type::Parameterized(_, args) | Type::Tuple(args) => args.iter().find_map(thread_bound_type),
        Type::Option(t) | Type::Vec(t) | Type::Array(t, _) | Type::Reference(t) | Type::MutableReference(t) => thread_bound_type(t),
        Type::Result(a, b) => thread_bound_type(a).or_else(|| thread_bound_type(b)),
        _ => None,
    }
}

/// `Rc::new(..)`, `RefCell::new(..)`, `Rc::clone(&x)`, ... (also nested: `Rc::new(RefCell::new(x))`).
fn thread_bound_value(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::Call { function: Expression::Identifier { name, .. }, .. } => {
            let head = name.split("::").next().unwrap_or("");
            THREAD_BOUND.contains(&head).then(|| head.to_string())
        }
        _ => None,
    }
}

fn pattern_names(p: &Pattern<'_>, out: &mut HashSet<String>) {
    match p {
        Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) => {
            out.insert(n.clone());
        }
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|q| pattern_names(q, out)),
        Pattern::Reference(q) => pattern_names(q, out),
        Pattern::EnumVariant(_, b) => match b {
            EnumPatternBinding::Tuple(ps) => ps.iter().for_each(|q| pattern_names(q, out)),
            EnumPatternBinding::Struct(fs, _) => fs.iter().for_each(|(_, q)| pattern_names(q, out)),
            _ => {}
        },
        _ => {}
    }
}

/// Names a task operand reads, names it declares, and names it assigns to.
#[derive(Default)]
struct Uses {
    read: HashSet<String>,
    declared: HashSet<String>,
    assigned: Vec<String>,
    /// Names called as functions (`f(x)`): functions and intrinsics, unless a local closure has the name.
    called: HashSet<String>,
}

/// The names a task operand reads from the code around it (minus what it declares itself), and the names it calls as
/// functions. The Rust backend clones the first set into the task (a name that is only called is a function).
pub fn task_free_names(e: &Expression<'_>) -> (Vec<String>, HashSet<String>) {
    let mut u = Uses::default();
    uses_expr(e, &mut u);
    let mut read: Vec<String> = u.read.into_iter().filter(|n| !u.declared.contains(n)).collect();
    read.sort();
    (read, u.called)
}

fn root_name(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => root_name(object),
        Expression::Unary { operand, .. } => root_name(operand),
        _ => None,
    }
}

fn uses_expr(e: &Expression<'_>, u: &mut Uses) {
    if let Expression::Call { function: Expression::Identifier { name, .. }, .. } = e {
        u.called.insert(name.clone());
    }
    match e {
        Expression::Identifier { name, .. } => {
            u.read.insert(name.clone());
        }
        Expression::Block { statements, .. } => statements.iter().for_each(|s| uses_stmt(s, u)),
        Expression::Closure { parameters, body, .. } => {
            for p in parameters {
                u.declared.insert(p.split(':').next().unwrap_or(p).trim().trim_start_matches("mut ").to_string());
            }
            uses_expr(body, u);
        }
        Expression::Select { arms, .. } => {
            for a in arms {
                pattern_names(&a.pattern, &mut u.declared);
                if let Some(s) = a.source {
                    uses_expr(s, u);
                }
                uses_expr(a.body, u);
            }
        }
        _ => children(e).into_iter().for_each(|c| uses_expr(c, u)),
    }
}

fn uses_block(b: &[&Statement<'_>], u: &mut Uses) {
    b.iter().for_each(|s| uses_stmt(s, u));
}

fn uses_stmt(s: &Statement<'_>, u: &mut Uses) {
    match s {
        Statement::Let { pattern, value, else_block, .. } => {
            uses_expr(value, u);
            pattern_names(pattern, &mut u.declared);
            if let Some(b) = else_block {
                uses_block(b, u);
            }
        }
        Statement::Const { name, value, .. } | Statement::Static { name, value, .. } => {
            u.declared.insert(name.clone());
            uses_expr(value, u);
        }
        Statement::Assignment { target, value, .. } => {
            if let Some(n) = root_name(target) {
                u.assigned.push(n);
            }
            uses_expr(target, u);
            uses_expr(value, u);
        }
        Statement::Return { value, .. } | Statement::Break { value, .. } => {
            if let Some(v) = value {
                uses_expr(v, u);
            }
        }
        Statement::Expression { expr, .. } => uses_expr(expr, u),
        Statement::If { condition, then_block, else_block, .. } => {
            uses_expr(condition, u);
            uses_block(then_block, u);
            if let Some(b) = else_block {
                uses_block(b, u);
            }
        }
        Statement::Match { value, arms, .. } => {
            uses_expr(value, u);
            for a in arms {
                pattern_names(&a.pattern, &mut u.declared);
                if let Some(g) = a.guard {
                    uses_expr(g, u);
                }
                uses_expr(a.body, u);
            }
        }
        Statement::For { pattern, iterable, body, .. } => {
            uses_expr(iterable, u);
            pattern_names(pattern, &mut u.declared);
            uses_block(body, u);
        }
        Statement::While { condition, body, .. } => {
            uses_expr(condition, u);
            uses_block(body, u);
        }
        Statement::Loop { body, .. } => uses_block(body, u),
        Statement::Defer { statement, .. } => uses_stmt(statement, u),
        Statement::Continue { .. } | Statement::Use { .. } => {}
    }
}

fn pos(loc: &SourceLocation) -> (usize, usize) {
    loc.as_ref().map(|l| (l.line, l.column)).unwrap_or((0, 0))
}

struct Cx {
    /// Locals of the current function known to be thread-bound, with the reason.
    thread_bound: HashMap<String, String>,
    out: Vec<CaptureFinding>,
}

impl Cx {
    fn items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => self.func(decl),
                Item::Impl { block, .. } => block.functions.iter().for_each(|f| self.func(f)),
                Item::Mod { items, .. } => self.items(items),
                _ => {}
            }
        }
    }

    fn func(&mut self, f: &FunctionDecl<'_>) {
        self.thread_bound.clear();
        for p in &f.parameters {
            if let Some(r) = thread_bound_type(&p.type_) {
                self.thread_bound.insert(p.name.clone(), r);
            }
        }
        f.body.iter().for_each(|s| self.stmt(s));
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        if let Statement::Let { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), type_, value, .. } = s {
            let reason = type_.as_ref().and_then(thread_bound_type).or_else(|| thread_bound_value(value)).or_else(|| match value {
                Expression::MethodCall { object: Expression::Identifier { name, .. }, method, .. } if method == "clone" => {
                    self.thread_bound.get(name).cloned()
                }
                _ => None,
            });
            match reason {
                Some(r) => {
                    self.thread_bound.insert(n.clone(), r);
                }
                None => {
                    self.thread_bound.remove(n);
                }
            }
        }
        self.tasks_in_stmt(s);
    }

    fn tasks_in_stmt(&mut self, s: &Statement<'_>) {
        let mut exprs: Vec<&Expression<'_>> = Vec::new();
        collect_exprs(s, &mut exprs);
        for e in exprs {
            self.expr(e);
        }
    }

    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::SpawnCall { expr, location } | Expression::AsyncCall { expr, location } => {
                let spawn = matches!(e, Expression::SpawnCall { .. });
                let mut u = Uses::default();
                uses_expr(expr, &mut u);
                let (line, column) = pos(location);
                let mut seen = HashSet::new();
                for n in &u.assigned {
                    if !u.declared.contains(n) && seen.insert(n.clone()) {
                        self.out.push(CaptureFinding { code: SHARED_WRITE, line, column, name: n.clone(), reason: String::new() });
                    }
                }
                if spawn {
                    let mut names: Vec<&String> = u.read.iter().filter(|n| !u.declared.contains(*n)).collect();
                    names.sort();
                    for n in names {
                        if let Some(r) = self.thread_bound.get(n) {
                            self.out.push(CaptureFinding { code: NOT_SENDABLE, line, column, name: n.clone(), reason: r.clone() });
                        }
                    }
                }
                self.expr(expr);
            }
            Expression::Block { statements, .. } => statements.iter().for_each(|s| self.stmt(s)),
            _ => children(e).into_iter().for_each(|c| self.expr(c)),
        }
    }
}

/// The direct expressions and nested statements of a statement, so nested blocks are walked with `stmt`.
fn collect_exprs<'a, 'ast>(s: &'a Statement<'ast>, out: &mut Vec<&'a Expression<'ast>>) {
    match s {
        Statement::Let { value, .. } | Statement::Const { value, .. } | Statement::Static { value, .. } => out.push(*value),
        Statement::Assignment { target, value, .. } => {
            out.push(*target);
            out.push(*value);
        }
        Statement::Return { value, .. } | Statement::Break { value, .. } => out.extend(value.iter().copied()),
        Statement::Expression { expr, .. } => out.push(*expr),
        Statement::If { condition, then_block, else_block, .. } => {
            out.push(*condition);
            then_block.iter().for_each(|t| collect_exprs(t, out));
            if let Some(b) = else_block {
                b.iter().for_each(|t| collect_exprs(t, out));
            }
        }
        Statement::Match { value, arms, .. } => {
            out.push(*value);
            for a in arms {
                out.extend(a.guard.iter().copied());
                out.push(a.body);
            }
        }
        Statement::For { iterable, body, .. } => {
            out.push(*iterable);
            body.iter().for_each(|t| collect_exprs(t, out));
        }
        Statement::While { condition, body, .. } => {
            out.push(*condition);
            body.iter().for_each(|t| collect_exprs(t, out));
        }
        Statement::Loop { body, .. } => body.iter().for_each(|t| collect_exprs(t, out)),
        Statement::Defer { statement, .. } => collect_exprs(statement, out),
        Statement::Continue { .. } | Statement::Use { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src: &str) -> Vec<(&'static str, String)> {
        let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
        let mut p = crate::parser::Parser::new(tokens);
        let program = p.parse().expect("parses");
        check(&program).into_iter().map(|f| (f.code, f.name)).collect()
    }

    #[test]
    fn a_task_that_changes_a_captured_variable_is_refused() {
        let f = findings("fn main() {\n    let mut total = 0\n    scope {\n        spawn { total += 1 }\n    }\n    println(\"${total}\")\n}\n");
        assert_eq!(f, vec![(SHARED_WRITE, "total".to_string())]);
    }

    #[test]
    fn a_spawned_task_cannot_take_an_rc_but_an_async_one_can() {
        let src = "use std::rc::Rc\nfn main() {\n    let r = Rc::new(1)\n    let a = async { r }\n    let b = spawn { r }\n    println(\"${a.wait()} ${b.wait()}\")\n}\n";
        assert_eq!(findings(src), vec![(NOT_SENDABLE, "r".to_string())]);
    }

    #[test]
    fn locals_of_the_task_and_reads_are_fine() {
        let src = "fn main() {\n    let n = 3\n    let t = spawn {\n        let mut s = 0\n        for i in 0..n {\n            s += i\n        }\n        s\n    }\n    println(\"${t.wait()}\")\n}\n";
        assert_eq!(findings(src), vec![]);
    }
}

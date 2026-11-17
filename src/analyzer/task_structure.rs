//! Structured concurrency (ADR-0004 decision 3, SPEC-CONC-003, WJ0190): every task is waited on.
//!
//! A task is the value of `spawn ...` / `async ...`, or of a call to a function of this file declared to return
//! `Task<T>`. Outside a `scope { }` (which waits for every task started inside it), a task must be kept and used:
//!
//! * a task used as a statement (`spawn work()` on its own line) or bound to `_` is dropped: error;
//! * a task bound to a name that nothing after the binding mentions is never waited on: error.
//!
//! Inside a `scope { }` (lexically, closures included) every task is fine: the scope waits. A task that is the last
//! statement of a block is that block's value (returned, or used by the enclosing expression), except the last
//! statement of a function declared without a return type, whose value is discarded.
//!
//! The check is conservative: a name mentioned anywhere after its binding counts as waited on (it may be passed to
//! a function that waits on it); a program it accepts may still drop a task at run time through such a call. Run by
//! `wj check` (with the fix: wrap the rest of the block in `scope { }`), `wj build` and the interpreter loader.

use crate::parser::ast::core::{Expression, Statement};
use crate::parser::*;
use std::collections::HashSet;

/// Diagnostic code of an un-waited task.
pub const CODE: &str = "WJ0190";

/// How the task is lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lost {
    /// Used as a statement or bound to `_`.
    Dropped,
    /// Bound to this name, which nothing mentions afterwards.
    NeverUsed(String),
}

/// One un-waited task. `line`/`column` are the task expression's (1-based; 0 without a location).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnwaitedTask {
    pub line: usize,
    pub column: usize,
    pub lost: Lost,
}

impl UnwaitedTask {
    pub fn message(&self) -> String {
        match &self.lost {
            Lost::Dropped => {
                "this task is never waited on: its result and its panic are lost".into()
            }
            Lost::NeverUsed(n) => {
                format!("task `{n}` is never waited on: its result and its panic are lost")
            }
        }
    }

    pub fn help(&self) -> String {
        match &self.lost {
            Lost::Dropped => "start it inside a `scope { }`, which waits for it, or keep it (`let t = spawn ...`) and call `t.wait()`".into(),
            Lost::NeverUsed(n) => format!("call `{n}.wait()` where the result is needed, or start the task inside a `scope {{ }}`"),
        }
    }

    pub fn render(&self) -> String {
        format!(
            "error[{CODE}]: {}\n  --> line {}:{}\n  = help: {}",
            self.message(),
            self.line,
            self.column,
            self.help()
        )
    }
}

/// Every un-waited task of `program`, in source order.
pub fn check(program: &Program<'_>) -> Vec<UnwaitedTask> {
    let mut cx = Cx {
        task_fns: HashSet::new(),
        scope_depth: 0,
        out: Vec::new(),
    };
    // a program with its own `Task` type means that type, not `std` tasks
    if !defines_task_type(&program.items) {
        collect_task_fns(&program.items, &mut cx.task_fns);
    }
    cx.items(&program.items);
    cx.out
}

/// `Err` with the first finding, rendered (for the compiler and the interpreter loader).
/// Also WJ0192/WJ0193 (`task_captures`).
pub fn gate(program: &Program<'_>) -> Result<(), String> {
    if let Some(f) = check(program).first() {
        return Err(f.render());
    }
    match super::task_captures::check(program).first() {
        Some(f) => Err(f.render()),
        None => Ok(()),
    }
}

fn is_task_type(t: &Type) -> bool {
    matches!(t, Type::Parameterized(n, _) | Type::Custom(n) if n == crate::parser::task_prefix::TASK_TYPE)
}

fn defines_task_type(items: &[Item<'_>]) -> bool {
    items.iter().any(|item| match item {
        Item::Struct { decl, .. } => decl.name == crate::parser::task_prefix::TASK_TYPE,
        Item::Enum { decl, .. } => decl.name == crate::parser::task_prefix::TASK_TYPE,
        Item::Mod { items, .. } => defines_task_type(items),
        _ => false,
    })
}

fn collect_task_fns(items: &[Item<'_>], out: &mut HashSet<String>) {
    for item in items {
        match item {
            Item::Function { decl, .. } if decl.return_type.as_ref().is_some_and(is_task_type) => {
                out.insert(decl.name.clone());
            }
            Item::Mod { items, .. } => collect_task_fns(items, out),
            _ => {}
        }
    }
}

fn pos(loc: &SourceLocation) -> (usize, usize) {
    loc.as_ref().map(|l| (l.line, l.column)).unwrap_or((0, 0))
}

struct Cx {
    task_fns: HashSet<String>,
    scope_depth: usize,
    out: Vec<UnwaitedTask>,
}

impl Cx {
    fn items(&mut self, items: &[Item<'_>]) {
        for item in items {
            match item {
                Item::Function { decl, .. } => self.func(decl),
                Item::Impl { block, .. } => block.functions.iter().for_each(|f| self.func(f)),
                Item::Trait { decl, .. } => {
                    for m in &decl.methods {
                        if let Some(body) = &m.body {
                            self.block(body, m.return_type.is_some());
                        }
                    }
                }
                Item::Mod { items, .. } => self.items(items),
                _ => {}
            }
        }
    }

    fn func(&mut self, f: &FunctionDecl<'_>) {
        self.block(&f.body, f.return_type.is_some());
    }

    fn is_task(&self, e: &Expression<'_>) -> bool {
        match e {
            Expression::SpawnCall { .. } | Expression::AsyncCall { .. } => true,
            Expression::Call {
                function: Expression::Identifier { name, .. },
                ..
            } => self.task_fns.contains(name),
            _ => false,
        }
    }

    /// `tail_used`: the value of the block's last statement is used (a function with a return type, or any block
    /// that is part of an expression).
    fn block(&mut self, stmts: &[&Statement<'_>], tail_used: bool) {
        let last = stmts.len().saturating_sub(1);
        for (i, s) in stmts.iter().enumerate() {
            if self.scope_depth == 0 {
                self.lost_task(stmts, i, i == last && tail_used);
            }
            self.stmt(s);
        }
    }

    fn lost_task(&mut self, stmts: &[&Statement<'_>], i: usize, is_value: bool) {
        let found = match stmts[i] {
            Statement::Expression { expr, .. } if !is_value && self.is_task(expr) => {
                Some((*expr, Lost::Dropped))
            }
            Statement::Let {
                pattern: Pattern::Wildcard,
                value,
                ..
            } if self.is_task(value) => Some((*value, Lost::Dropped)),
            Statement::Let {
                pattern: Pattern::Identifier(n) | Pattern::MutBinding(n),
                value,
                ..
            } if self.is_task(value) => {
                let used = stmts[i + 1..].iter().any(|s| stmt_mentions(s, n));
                (!used).then(|| (*value, Lost::NeverUsed(n.clone())))
            }
            _ => None,
        };
        if let Some((e, lost)) = found {
            // a call records the position after its `)` and a name the position after the name: point at the
            // start of the callee's name
            let (line, column) = match e {
                Expression::Call { function: Expression::Identifier { name, location }, .. } => {
                    let (l, c) = pos(location);
                    (l, c.saturating_sub(name.chars().count()).max(1))
                }
                _ => pos(&e.location()),
            };
            self.out.push(UnwaitedTask { line, column, lost });
        }
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        match s {
            Statement::Let {
                value, else_block, ..
            } => {
                self.expr(value);
                if let Some(b) = else_block {
                    self.block(b, false);
                }
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => self.expr(value),
            Statement::Assignment { target, value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            Statement::Return { value, .. } => value.iter().for_each(|v| self.expr(v)),
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.expr(condition);
                self.block(then_block, true);
                if let Some(b) = else_block {
                    self.block(b, true);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr(value);
                for a in arms {
                    if let Some(g) = a.guard {
                        self.expr(g);
                    }
                    self.expr(a.body);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.expr(iterable);
                self.block(body, false);
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr(condition);
                self.block(body, false);
            }
            Statement::Loop { body, .. } => self.block(body, false),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Break { value, .. } => value.iter().for_each(|v| self.expr(v)),
            Statement::Continue { .. } | Statement::Use { .. } => {}
        }
    }

    fn expr(&mut self, e: &Expression<'_>) {
        match e {
            Expression::Scope { body, .. } => {
                self.scope_depth += 1;
                self.expr(body);
                self.scope_depth -= 1;
            }
            Expression::Block { statements, .. } => self.block(statements, true),
            _ => children(e).into_iter().for_each(|c| self.expr(c)),
        }
    }
}

/// The direct sub-expressions of `e` (blocks are walked by the caller through `Block`).
pub(super) fn children<'a, 'ast>(e: &'a Expression<'ast>) -> Vec<&'a Expression<'ast>> {
    match e {
        Expression::Literal { .. } | Expression::Identifier { .. } => vec![],
        Expression::Binary { left, right, .. } => vec![*left, *right],
        Expression::Range { start, end, .. } => vec![*start, *end],
        Expression::Index { object, index, .. } => vec![*object, *index],
        Expression::Unary { operand: x, .. }
        | Expression::FieldAccess { object: x, .. }
        | Expression::Closure { body: x, .. }
        | Expression::Cast { expr: x, .. }
        | Expression::TryOp { expr: x, .. }
        | Expression::Await { expr: x, .. }
        | Expression::AsyncCall { expr: x, .. }
        | Expression::SpawnCall { expr: x, .. }
        | Expression::Scope { body: x, .. } => vec![*x],
        Expression::Call {
            function,
            arguments,
            ..
        } => std::iter::once(*function)
            .chain(arguments.iter().map(|(_, a)| *a))
            .collect(),
        Expression::MethodCall {
            object, arguments, ..
        } => std::iter::once(*object)
            .chain(arguments.iter().map(|(_, a)| *a))
            .collect(),
        Expression::StructLiteral { fields, .. } => fields.iter().map(|(_, f)| *f).collect(),
        Expression::MapLiteral { pairs, .. } => pairs.iter().flat_map(|(k, v)| [*k, *v]).collect(),
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            elements.to_vec()
        }
        Expression::MacroInvocation { args, .. } => args.to_vec(),
        Expression::Select { arms, .. } => arms.iter().flat_map(|a| a.source.into_iter().chain(std::iter::once(a.body))).collect(),
        Expression::Block { .. } => vec![],
    }
}

fn expr_mentions(e: &Expression<'_>, name: &str) -> bool {
    match e {
        Expression::Identifier { name: n, .. } => n == name,
        Expression::Block { statements, .. } => statements.iter().any(|s| stmt_mentions(s, name)),
        // `"${t.wait()}"` is a macro whose arguments are the interpolated expressions
        _ => children(e).into_iter().any(|c| expr_mentions(c, name)),
    }
}

fn block_mentions(b: &[&Statement<'_>], name: &str) -> bool {
    b.iter().any(|s| stmt_mentions(s, name))
}

fn stmt_mentions(s: &Statement<'_>, name: &str) -> bool {
    match s {
        Statement::Let {
            value, else_block, ..
        } => {
            expr_mentions(value, name)
                || else_block
                    .as_deref()
                    .is_some_and(|b| block_mentions(b, name))
        }
        Statement::Const { value, .. } | Statement::Static { value, .. } => {
            expr_mentions(value, name)
        }
        Statement::Assignment { target, value, .. } => {
            expr_mentions(target, name) || expr_mentions(value, name)
        }
        Statement::Return { value, .. } | Statement::Break { value, .. } => {
            value.is_some_and(|v| expr_mentions(v, name))
        }
        Statement::Expression { expr, .. } => expr_mentions(expr, name),
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expr_mentions(condition, name)
                || block_mentions(then_block, name)
                || else_block
                    .as_deref()
                    .is_some_and(|b| block_mentions(b, name))
        }
        Statement::Match { value, arms, .. } => {
            expr_mentions(value, name)
                || arms.iter().any(|a| {
                    a.guard.is_some_and(|g| expr_mentions(g, name)) || expr_mentions(a.body, name)
                })
        }
        Statement::For { iterable, body, .. } => {
            expr_mentions(iterable, name) || block_mentions(body, name)
        }
        Statement::While {
            condition, body, ..
        } => expr_mentions(condition, name) || block_mentions(body, name),
        Statement::Loop { body, .. } => block_mentions(body, name),
        Statement::Defer { statement, .. } => stmt_mentions(statement, name),
        Statement::Continue { .. } | Statement::Use { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src: &str) -> Vec<UnwaitedTask> {
        let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
        let mut p = crate::parser::Parser::new(tokens);
        let program = p.parse().expect("parses");
        check(&program)
    }

    #[test]
    fn dropped_and_unused_tasks_outside_a_scope_are_found() {
        let f = findings("fn w() -> int { 1 }\nfn main() {\n    spawn w()\n    let _ = async w()\n    let t = spawn { 2 }\n    println(\"x\")\n}\n");
        let lost: Vec<_> = f.iter().map(|x| (x.line, x.lost.clone())).collect();
        assert_eq!(
            lost,
            vec![
                (3, Lost::Dropped),
                (4, Lost::Dropped),
                (5, Lost::NeverUsed("t".into()))
            ]
        );
    }

    #[test]
    fn waited_scoped_and_returned_tasks_are_fine() {
        let src = "fn w() -> int { 1 }\nfn start() -> Task<int> { spawn w() }\nfn main() {\n    let t = spawn w()\n    println(\"${t.wait()}\")\n    scope {\n        spawn w()\n        spawn { println(\"a\") }\n    }\n    let s = start()\n    s.wait()\n}\n";
        assert_eq!(findings(src), vec![]);
    }

    #[test]
    fn a_dropped_call_of_a_task_function_is_found() {
        let f = findings("fn w() -> int { 1 }\nfn start() -> Task<int> { spawn w() }\nfn main() {\n    start()\n}\n");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].lost, Lost::Dropped);
    }
}

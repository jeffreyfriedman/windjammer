//! NUM-DEFAULT-INT as a fact (convergence plan C4, ADR-0015 section 1).
//!
//! An integer literal whose class (the literals and locals connected to it by `let`, assignment,
//! arithmetic, comparison and `for` ranges) is never used where a context fixes a width is `int`,
//! i.e. i64. Without this fact the Rust backend printed such literals bare and rustc's own
//! fallback made them `i32` (`let big = 3000000000` overflowed; `3.min(9)` was ambiguous;
//! `1 << 40` wrapped), which disagrees with the language.
//!
//! The analysis is deliberately conservative: any use the analysis does not model (a call
//! argument, a method receiver or argument other than the integer methods below, indexing, a
//! field, a return, a closure, a collection literal, a declared type) *opens* the class and no
//! literal in it is reported. Widening the modelled vocabulary is how this fact grows into the
//! numeric solver's authority (plan C4.2); every extension needs a corpus program.
//!
//! Facts are keyed by AST node identity (the address of the arena-allocated literal), never by
//! source position (plan C2/C4: positions collide between an expression and its closure).

use crate::parser::ast::core::{Expression, Pattern, Statement};
use crate::parser::ast::literals::Literal;
use crate::parser::ast::operators::{BinaryOp, UnaryOp};
use std::collections::{BTreeMap, BTreeSet};

/// Stable identity of an AST node for the duration of one compilation (arena address).
pub fn node_id(expr: &Expression<'_>) -> usize {
    expr as *const Expression<'_> as usize
}

/// Key of a literal that survives AST re-allocation between the analyzer and codegen: the
/// literal's own source position (unique among literals, unlike the position *after* an
/// expression that numeric inference uses).
pub fn literal_key(expr: &Expression<'_>) -> Option<(usize, usize)> {
    expr.location().as_ref().map(|l| (l.line, l.column))
}

#[derive(Default)]
struct Classes {
    parent: Vec<usize>,
    open: Vec<bool>,
    locals: BTreeMap<String, usize>,
    literals: Vec<((usize, usize), usize)>, // (literal key, class)
}

impl Classes {
    fn fresh(&mut self) -> usize {
        self.parent.push(self.parent.len());
        self.open.push(false);
        self.parent.len() - 1
    }
    fn opened(&mut self) -> usize {
        let c = self.fresh();
        self.open[c] = true;
        c
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        let mut y = x;
        while self.parent[y] != r {
            let next = self.parent[y];
            self.parent[y] = r;
            y = next;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
            self.open[rb] = self.open[rb] || self.open[ra];
        }
    }
    fn open_class(&mut self, c: usize) {
        let r = self.find(c);
        self.open[r] = true;
    }
    fn local(&mut self, name: &str) -> usize {
        if let Some(&c) = self.locals.get(name) {
            return c;
        }
        let c = self.fresh();
        self.locals.insert(name.to_string(), c);
        c
    }
}

/// Integer methods whose receiver, arguments and result share one width.
fn same_width_int_method(method: &str) -> bool {
    matches!(method, "min" | "max" | "clamp" | "abs" | "signum")
}

struct Walker {
    c: Classes,
    /// Names bound by `let`/`for` in this function; anything else (parameters, constants,
    /// globals) has a declared type and opens the class.
    bound: BTreeSet<String>,
}

impl Walker {
    fn bind_pattern_open(&mut self, p: &Pattern<'_>) {
        match p {
            Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) | Pattern::RefMut(n) => {
                self.bound.insert(n.clone());
                let c = self.c.local(n);
                self.c.open_class(c);
            }
            Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().for_each(|p| self.bind_pattern_open(p)),
            Pattern::Reference(p) => self.bind_pattern_open(p),
            Pattern::EnumVariant(_, b) => {
                use crate::parser::ast::core::EnumPatternBinding as B;
                match b {
                    B::Single(n) => {
                        self.bound.insert(n.clone());
                        let c = self.c.local(n);
                        self.c.open_class(c);
                    }
                    B::Tuple(ps) => ps.iter().for_each(|p| self.bind_pattern_open(p)),
                    B::Struct(fs, _) => fs.iter().for_each(|(_, p)| self.bind_pattern_open(p)),
                    B::None | B::Wildcard => {}
                }
            }
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
        }
    }

    fn stmts(&mut self, body: &[&Statement<'_>]) {
        for (i, s) in body.iter().enumerate() {
            // A trailing expression may be the block's value (a function result, an `if` arm):
            // its context is unknown here, so it opens its class.
            if i + 1 == body.len() {
                if let Statement::Expression { expr, .. } = s {
                    let o = self.c.opened();
                    self.expr(expr, o);
                    continue;
                }
            }
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Statement<'_>) {
        match s {
            Statement::Let { pattern, type_, value, else_block, .. } => {
                match (pattern, type_) {
                    (Pattern::Identifier(n) | Pattern::MutBinding(n), None) => {
                        self.bound.insert(n.clone());
                        let c = self.c.local(n);
                        self.expr(value, c);
                    }
                    _ => {
                        let o = self.c.opened();
                        self.expr(value, o);
                        self.bind_pattern_open(pattern);
                    }
                }
                if let Some(b) = else_block {
                    self.stmts(b);
                }
            }
            Statement::Assignment { target, value, .. } => match target {
                Expression::Identifier { name, .. } if self.bound.contains(name) => {
                    let c = self.c.local(name);
                    self.expr(value, c);
                }
                _ => {
                    let o = self.c.opened();
                    self.expr(target, o);
                    let o = self.c.opened();
                    self.expr(value, o);
                }
            },
            Statement::Expression { expr, .. } => {
                let c = self.c.fresh();
                self.expr(expr, c);
            }
            Statement::If { condition, then_block, else_block, .. } => {
                let c = self.c.fresh();
                self.expr(condition, c);
                self.stmts(then_block);
                if let Some(b) = else_block {
                    self.stmts(b);
                }
            }
            Statement::While { condition, body, .. } => {
                let c = self.c.fresh();
                self.expr(condition, c);
                self.stmts(body);
            }
            Statement::Loop { body, .. } => self.stmts(body),
            Statement::For { pattern, iterable, body, .. } => {
                match (pattern, iterable) {
                    (
                        Pattern::Identifier(n) | Pattern::MutBinding(n),
                        Expression::Range { start, end, .. },
                    ) => {
                        self.bound.insert(n.clone());
                        let c = self.c.local(n);
                        self.expr(start, c);
                        self.expr(end, c);
                    }
                    _ => {
                        let o = self.c.opened();
                        self.expr(iterable, o);
                        self.bind_pattern_open(pattern);
                    }
                }
                self.stmts(body);
            }
            Statement::Match { value, arms, .. } => {
                let o = self.c.opened();
                self.expr(value, o);
                for arm in arms {
                    self.bind_pattern_open(&arm.pattern);
                    if let Some(g) = arm.guard {
                        let c = self.c.fresh();
                        self.expr(g, c);
                    }
                    let o = self.c.opened();
                    self.expr(arm.body, o);
                }
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    let o = self.c.opened();
                    self.expr(v, o);
                }
            }
            Statement::Const { value, .. } | Statement::Static { value, .. } => {
                let o = self.c.opened();
                self.expr(value, o);
            }
            Statement::Thread { body, .. } | Statement::Async { body, .. } => self.stmts(body),
            Statement::Defer { statement, .. } => self.stmt(statement),
            Statement::Break { value, .. } => {
                // `break 'l k * 2`: the value flows to an unknown context (the loop's result).
                if let Some(v) = value {
                    let o = self.c.opened();
                    self.expr(v, o);
                }
            }
            Statement::Continue { .. } | Statement::Use { .. } => {}
        }
    }

    /// Visit `e`, whose value joins class `sink`.
    fn expr(&mut self, e: &Expression<'_>, sink: usize) {
        match e {
            Expression::Literal { value, .. } => match value {
                Literal::Int(_) => {
                    let c = self.c.fresh();
                    if let Some(k) = literal_key(e) {
                        self.c.literals.push((k, c));
                    }
                    self.c.union(c, sink);
                }
                // A suffixed or non-integer literal fixes (or contradicts) the width.
                _ => self.c.open_class(sink),
            },
            Expression::Identifier { name, .. } => {
                if self.bound.contains(name) {
                    let c = self.c.local(name);
                    self.c.union(c, sink);
                } else {
                    self.c.open_class(sink);
                }
            }
            Expression::Binary { left, op, right, .. } => match op {
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor => {
                    self.expr(left, sink);
                    self.expr(right, sink);
                }
                BinaryOp::Shl | BinaryOp::Shr => {
                    self.expr(left, sink);
                    let o = self.c.opened();
                    self.expr(right, o);
                }
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge => {
                    let c = self.c.fresh();
                    self.expr(left, c);
                    self.expr(right, c);
                }
                BinaryOp::And | BinaryOp::Or => {
                    let c = self.c.fresh();
                    self.expr(left, c);
                    let c = self.c.fresh();
                    self.expr(right, c);
                }
                #[allow(unreachable_patterns)]
                _ => {
                    self.c.open_class(sink);
                    let o = self.c.opened();
                    self.expr(left, o);
                    let o = self.c.opened();
                    self.expr(right, o);
                }
            },
            Expression::Unary { op, operand, .. } => match op {
                UnaryOp::Neg => self.expr(operand, sink),
                UnaryOp::Not => {
                    let c = self.c.fresh();
                    self.expr(operand, c);
                }
                _ => {
                    self.c.open_class(sink);
                    let o = self.c.opened();
                    self.expr(operand, o);
                }
            },
            Expression::Call { function, arguments, .. } => {
                // A callee that is not a plain name (a closure literal, a field) may use locals.
                if !matches!(function, Expression::Identifier { .. }) {
                    let o = self.c.opened();
                    self.expr(function, o);
                }
                // Interpolation (`format` macro arguments, handled below) never constrains; any other
                // argument is used in a position this analysis does not model.
                let printing = false;
                for (_, a) in arguments {
                    let is_format = matches!(a, Expression::MacroInvocation { name, .. } if name == "format");
                    let c = if printing || is_format { self.c.fresh() } else { self.c.opened() };
                    self.expr(a, c);
                }
                self.c.open_class(sink);
            }
            Expression::MacroInvocation { name, args, .. } if name == "format" => {
                // Interpolation prints any width: its arguments do not constrain.
                for a in args {
                    let c = self.c.fresh();
                    self.expr(a, c);
                }
                self.c.open_class(sink);
            }
            Expression::MethodCall { object, method, arguments, .. } if same_width_int_method(method) => {
                self.expr(object, sink);
                for (_, a) in arguments {
                    self.expr(a, sink);
                }
            }
            Expression::Cast { expr, .. } => {
                // NUM-CAST: `as` fixes the result's width and does not constrain the operand.
                let c = self.c.fresh();
                self.expr(expr, c);
                self.c.open_class(sink);
            }
            // Closure parameters take their widths from the caller; locals the body mentions are
            // used in a context this analysis does not model, so their classes open.
            Expression::Closure { body, .. } => {
                self.c.open_class(sink);
                let o = self.c.opened();
                self.expr(body, o);
            }
            other => {
                self.c.open_class(sink);
                for child in children(other) {
                    let o = self.c.opened();
                    self.expr(child, o);
                }
                if let Expression::Block { statements, .. } = other {
                    self.stmts(statements);
                }
            }
        }
    }
}

fn children<'a, 'ast>(e: &'a Expression<'ast>) -> Vec<&'a Expression<'ast>> {
    match e {
        Expression::Call { function, arguments, .. } => {
            let mut v = vec![*function];
            v.extend(arguments.iter().map(|(_, a)| *a));
            v
        }
        Expression::MethodCall { object, arguments, .. } => {
            let mut v = vec![*object];
            v.extend(arguments.iter().map(|(_, a)| *a));
            v
        }
        Expression::FieldAccess { object, .. } => vec![*object],
        Expression::StructLiteral { fields, .. } => fields.iter().map(|(_, f)| *f).collect(),
        Expression::MapLiteral { pairs, .. } => pairs.iter().flat_map(|(k, v)| [*k, *v]).collect(),
        Expression::Range { start, end, .. } => vec![*start, *end],
        Expression::Index { object, index, .. } => vec![*object, *index],
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements.clone(),
        Expression::MacroInvocation { args, .. } => args.clone(),
        Expression::TryOp { expr, .. }
        | Expression::Await { expr, .. }
        | Expression::AsyncCall { expr, .. }
        | Expression::SpawnCall { expr, .. } => vec![*expr],
        Expression::ChannelSend { channel, value, .. } => vec![*channel, *value],
        Expression::ChannelRecv { channel, .. } => vec![*channel],
        Expression::Binary { left, right, .. } => vec![*left, *right],
        Expression::Unary { operand, .. } => vec![*operand],
        Expression::Cast { expr, .. } => vec![*expr],
        Expression::Literal { .. }
        | Expression::Identifier { .. }
        | Expression::Closure { .. }
        | Expression::Block { .. } => Vec::new(),
    }
}

/// Integer literals of `body` that NUM-DEFAULT-INT makes `int` (i64), by [`literal_key`].
pub fn default_int_literals(body: &[&Statement<'_>]) -> BTreeSet<(usize, usize)> {
    let mut w = Walker { c: Classes::default(), bound: BTreeSet::new() };
    // Pre-pass: names bound anywhere in the body, so a use before the binding (in a loop) is
    // still recognised as a local.
    collect_bound(body, &mut w.bound);
    w.stmts(body);
    let literals = std::mem::take(&mut w.c.literals);
    literals
        .into_iter()
        .filter(|&(_, c)| {
            let r = w.c.find(c);
            !w.c.open[r]
        })
        .map(|(id, _)| id)
        .collect()
}

fn collect_bound(body: &[&Statement<'_>], out: &mut BTreeSet<String>) {
    for s in body {
        match s {
            Statement::Let { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), type_: None, else_block, .. } => {
                out.insert(n.clone());
                if let Some(b) = else_block {
                    collect_bound(b, out);
                }
            }
            Statement::For { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), iterable: Expression::Range { .. }, body, .. } => {
                out.insert(n.clone());
                collect_bound(body, out);
            }
            Statement::For { body, .. } | Statement::While { body, .. } | Statement::Loop { body, .. } => collect_bound(body, out),
            Statement::If { then_block, else_block, .. } => {
                collect_bound(then_block, out);
                if let Some(b) = else_block {
                    collect_bound(b, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literals_in(src: &str, f: &str) -> (Vec<String>, BTreeSet<(usize, usize)>) {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == f => Some(decl),
                _ => None,
            })
            .expect("function");
        let set = default_int_literals(&decl.body);
        // Render every int literal with a mark when it is defaulted, in source order.
        let mut all = Vec::new();
        fn walk_s(s: &Statement<'_>, set: &BTreeSet<(usize, usize)>, out: &mut Vec<String>) {
            match s {
                Statement::Let { value, .. } | Statement::Assignment { value, .. } => walk_e(value, set, out),
                Statement::Expression { expr, .. } => walk_e(expr, set, out),
                Statement::Return { value: Some(v), .. } => walk_e(v, set, out),
                Statement::While { condition, body, .. } => {
                    walk_e(condition, set, out);
                    body.iter().for_each(|s| walk_s(s, set, out));
                }
                Statement::For { iterable, body, .. } => {
                    walk_e(iterable, set, out);
                    body.iter().for_each(|s| walk_s(s, set, out));
                }
                _ => {}
            }
        }
        fn walk_e(e: &Expression<'_>, set: &BTreeSet<(usize, usize)>, out: &mut Vec<String>) {
            if let Expression::Literal { value: Literal::Int(i), .. } = e {
                out.push(format!("{}{}", i, if literal_key(e).is_some_and(|k| set.contains(&k)) { ":int" } else { ":?" }));
                return;
            }
            for c in children(e) {
                walk_e(c, set, out);
            }
        }
        decl.body.iter().for_each(|s| walk_s(s, &set, &mut all));
        (all, set)
    }

    #[test]
    fn unconstrained_literal_class_defaults_to_int() {
        let (lits, _) = literals_in("fn main() {\n let big = 3000000000\n let y = big + 1\n println(\"${y}\")\n}\n", "main");
        assert_eq!(lits, vec!["3000000000:int", "1:int"]);
    }

    #[test]
    fn same_width_int_methods_join_the_class() {
        let (lits, _) = literals_in("fn main() {\n println(\"${3.min(9)}\")\n println(\"${(-8).clamp(5, 10)}\")\n}\n", "main");
        assert_eq!(lits, vec!["3:int", "9:int", "8:int", "5:int", "10:int"]);
    }

    #[test]
    fn a_constraining_use_anywhere_opens_the_class() {
        let src = "fn take(x: u8) {}\nfn main() {\n let a = 5\n let b = a + 1\n take(b)\n let v = vec![1, 2]\n let i = 0\n println(\"${v[i]}\")\n}\n";
        let (lits, _) = literals_in(src, "main");
        assert_eq!(lits, vec!["5:?", "1:?", "1:?", "2:?", "0:?"]);
    }

    #[test]
    fn while_counter_and_for_range_default_when_only_compared_and_printed() {
        let src = "fn main() {\n let mut i = 0\n while i < 10 {\n i = i + 1\n }\n for k in 0..3 {\n println(\"${k}\")\n }\n println(\"${i}\")\n}\n";
        let (lits, _) = literals_in(src, "main");
        assert_eq!(lits, vec!["0:int", "10:int", "1:int", "0:int", "3:int"]);
    }

    #[test]
    fn break_values_and_closure_bodies_open_the_classes_they_use() {
        let src = "fn main() {\n let r = loop {\n let mut k = 0\n k += 1\n break k * 2\n }\n println(\"${r}\")\n}\n";
        let (lits, _) = literals_in(src, "main");
        assert!(lits.iter().all(|l| l.ends_with(":?")), "{lits:?}");
        let src = "fn main() {\n let a = 5\n let f = |x: int| x + a\n println(\"${f(1)}\")\n}\n";
        let (lits, _) = literals_in(src, "main");
        assert_eq!(lits, vec!["5:?"]);
    }

    #[test]
    fn returned_or_declared_values_are_left_to_their_declared_type() {
        let src = "fn f() -> i32 {\n let x = 4\n x\n}\n";
        let (lits, _) = literals_in(src, "f");
        assert_eq!(lits, vec!["4:?"], "a tail expression is the function's value");
        let src2 = "fn g() -> i32 {\n let x = 4\n return x\n}\n";
        let (lits2, _) = literals_in(src2, "g");
        assert_eq!(lits2, vec!["4:?"]);
    }
}

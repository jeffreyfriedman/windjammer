//! W0013 (ADR-0015 #1/#2, spec Q-017): arithmetic or comparison between two *explicitly typed* integer
//! values of different widths (`a: i32` + `b: i64`). The Rust backend used to widen silently; the spec says
//! mixed widths need an explicit conversion. Conservative: only identifiers whose width is declared
//! (parameters, annotated `let`) are considered, so untyped literals and inferred values never trigger it.

use crate::parser::ast::core::{Expression, FunctionDecl, Item, Program, Statement};
use crate::parser::ast::operators::BinaryOp;
use crate::parser::Type;
use crate::source_map::Location;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedWidth {
    pub location: Option<Location>,
    pub left: String,
    pub right: String,
}

fn width(t: &Type) -> Option<&'static str> {
    Some(match t {
        Type::Int => "int",
        Type::Int32 => "i32",
        Type::Uint => "u64",
        Type::Custom(n) => match n.as_str() {
            "int" | "i64" => "int",
            "i8" => "i8",
            "i16" => "i16",
            "i32" => "i32",
            "i128" => "i128",
            "isize" => "isize",
            "u8" => "u8",
            "u16" => "u16",
            "u32" => "u32",
            "u64" => "u64",
            "u128" => "u128",
            "usize" => "usize",
            _ => return None,
        },
        _ => return None,
    })
}

struct Walker {
    scopes: Vec<HashMap<String, &'static str>>,
    out: Vec<MixedWidth>,
}

impl Walker {
    fn lookup(&self, e: &Expression) -> Option<&'static str> {
        let Expression::Identifier { name, .. } = e else { return None };
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    fn check_pair(&mut self, l: &Expression, r: &Expression, loc: Option<Location>) {
        if let (Some(a), Some(b)) = (self.lookup(l), self.lookup(r)) {
            if a != b {
                // The left operand's span is its start; binary/assignment spans point past the expression.
                let location = l.location().or(loc);
                self.out.push(MixedWidth { location, left: a.to_string(), right: b.to_string() });
            }
        }
    }

    fn block(&mut self, stmts: &[&Statement]) {
        self.scopes.push(HashMap::new());
        for s in stmts {
            self.stmt(s);
        }
        self.scopes.pop();
    }

    fn stmt(&mut self, s: &Statement) {
        match s {
            Statement::Let { pattern, type_, value, .. } => {
                self.expr(value);
                if let (crate::parser::Pattern::Identifier(n), Some(w)) = (pattern, type_.as_ref().and_then(width)) {
                    if let Some(scope) = self.scopes.last_mut() {
                        scope.insert(n.clone(), w);
                    }
                }
            }
            Statement::Assignment { target, value, compound_op, location } => {
                self.expr(value);
                if compound_op.is_some() {
                    self.check_pair(target, value, location.clone());
                }
            }
            Statement::Expression { expr, .. } => self.expr(expr),
            Statement::Return { value: Some(v), .. } => self.expr(v),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition);
                self.block(then_block);
                if let Some(e) = else_block {
                    self.block(e);
                }
            }
            Statement::While { condition, body, .. } => {
                self.expr(condition);
                self.block(body);
            }
            Statement::For { iterable, body, .. } => {
                self.expr(iterable);
                self.block(body);
            }
            Statement::Loop { body, .. } => self.block(body),
            Statement::Match { value, arms, .. } => {
                self.expr(value);
                for a in arms {
                    self.expr(a.body);
                }
            }
            _ => {}
        }
    }

    fn expr(&mut self, e: &Expression) {
        match e {
            Expression::Binary { left, op, right, location } => {
                self.expr(left);
                self.expr(right);
                if !matches!(op, BinaryOp::And | BinaryOp::Or | BinaryOp::Shl | BinaryOp::Shr) {
                    self.check_pair(left, right, location.clone());
                }
            }
            Expression::Unary { operand, .. } => self.expr(operand),
            Expression::Call { function, arguments, .. } => {
                self.expr(function);
                arguments.iter().for_each(|(_, a)| self.expr(a));
            }
            Expression::MethodCall { object, arguments, .. } => {
                self.expr(object);
                arguments.iter().for_each(|(_, a)| self.expr(a));
            }
            Expression::FieldAccess { object, .. } => self.expr(object),
            Expression::Index { object, index, .. } => {
                self.expr(object);
                self.expr(index);
            }
            Expression::Block { statements, .. } => self.block(statements),
            _ => {}
        }
    }

    fn function(&mut self, f: &FunctionDecl) {
        let mut params = HashMap::new();
        for p in &f.parameters {
            if let Some(w) = width(&p.type_) {
                params.insert(p.name.clone(), w);
            }
        }
        self.scopes.push(params);
        self.block(&f.body);
        self.scopes.pop();
    }

    fn item(&mut self, i: &Item) {
        match i {
            Item::Function { decl, .. } => self.function(decl),
            Item::Impl { block, .. } => block.functions.iter().for_each(|f| self.function(f)),
            Item::Mod { items, .. } => items.iter().for_each(|i| self.item(i)),
            _ => {}
        }
    }
}

/// Every binary operation (and compound assignment) mixing two declared integer widths.
pub fn mixed_widths(program: &Program) -> Vec<MixedWidth> {
    let mut w = Walker { scopes: Vec::new(), out: Vec::new() };
    program.items.iter().for_each(|i| w.item(i));
    w.out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> Vec<MixedWidth> {
        let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
        mixed_widths(&crate::parser::Parser::new_with_source(tokens, "t.wj".into(), src.into()).parse().unwrap())
    }

    #[test]
    fn declared_widths_that_differ_are_reported() {
        let r = run("fn f(a: i32, b: int) -> bool {\n    let c: u8 = 1\n    a + 1 > 0 && a < b\n}\n");
        assert_eq!(r.len(), 1, "{r:?}");
        assert_eq!((r[0].left.as_str(), r[0].right.as_str()), ("i32", "int"));
        assert_eq!(run("fn g(a: i32) {\n    let mut t: i64 = 0\n    t += a\n}\n").len(), 1);
    }

    #[test]
    fn same_width_and_untyped_values_are_fine() {
        assert!(run("fn f(a: int, b: i64) -> int {\n    let c = a * 2\n    a + b + c\n}\n").is_empty());
        assert!(run("fn f(a: i32, b: i32) -> i32 {\n    a + b\n}\n").is_empty());
        assert!(run("fn f(a: i32, b: int) -> int {\n    int(a) + b\n}\n").is_empty());
    }
}

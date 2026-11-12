//! Benchmark experiment (convergence C5, off by default, removed before merge): SELF-CONSUME-FIELDS, the
//! candidate rule "`self` is owned when it is dead after its last field move and the method never writes
//! `self`". Decided from IR facts: the solved receiver mode (not `&mut`, not a mutated parameter) and the
//! body's `self` uses in evaluation order. Enabled by `IrPipelineConfig::self_owned_after_last_field_move`.
//!
//! A field move is a direct `self.f` of a non-Copy declared type in an owned position: a struct-literal
//! field, a tuple or array element, a `let` or assignment value, a `return` or the function's value. The
//! rule holds when there is at least one, none is inside a loop or closure, no field is moved twice or read
//! after its move, and `self` is not used whole after the first move: after the last move `self` is dead
//! except for reads of fields it did not move (a partly moved value, which Rust allows). A field that is
//! itself the result (`fn name(self) -> string { self.title }`, or `return self.title`) keeps SELF-GETTER:
//! callers keep using the object, so the field clone is cheaper than a clone of the whole object at their
//! call sites (measured: the getter shape costs clones under this rule, the struct-building shapes save them).

use std::collections::{HashMap, HashSet};

use crate::parser::ast::core::{Expression, Statement};
use crate::parser::Type;

#[derive(Debug, Clone, PartialEq)]
enum Use {
    Move(String),
    Read(String),
    Whole,
}

/// Where an expression's value goes: nowhere owned, into a binding or literal, or out as the method's result.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Pos {
    Read,
    Stored,
    Result,
}

struct Walk<'f> {
    uses: Vec<Use>,
    depth: usize,
    unsupported: bool,
    getter: bool,
    non_copy: &'f dyn Fn(&str) -> bool,
    /// `(binding, method)` of every call whose receiver is a plain binding (`store.add(..)`).
    calls: Vec<(String, String)>,
}

/// The `(binding, method)` pairs of method calls made directly on a binding, anywhere in `body`.
/// Companion of the receiver rule: a by-value parameter that is the receiver of an owned-`self` method is an
/// owned use, as an argument passed to an owned formal is (OWN-CALL-FORMAL).
pub fn direct_method_calls(body: &[&Statement<'_>]) -> Vec<(String, String)> {
    let none = |_: &str| false;
    let mut w = Walk { uses: Vec::new(), depth: 0, unsupported: false, getter: false, non_copy: &none, calls: Vec::new() };
    w.block(body, false);
    w.calls
}

/// True when the rule makes `self` owned. `field_is_non_copy` answers from the declared field type.
pub fn self_owned_after_last_field_move(body: &[&Statement<'_>], returns_value: bool, field_is_non_copy: &dyn Fn(&str) -> bool) -> bool {
    let mut w = Walk { uses: Vec::new(), depth: 0, unsupported: false, getter: false, non_copy: field_is_non_copy, calls: Vec::new() };
    w.block(body, returns_value);
    if w.getter {
        return false; // SELF-GETTER: a field returned as the result keeps `&self` and clones the field
    }
    if w.unsupported {
        return false;
    }
    let Some(first) = w.uses.iter().position(|u| matches!(u, Use::Move(_))) else { return false };
    let mut moved: HashSet<&str> = HashSet::new();
    for u in &w.uses[first..] {
        match u {
            Use::Whole => return false,
            Use::Read(f) if moved.contains(f.as_str()) => return false,
            Use::Move(f) if !moved.insert(f.as_str()) => return false,
            _ => {}
        }
    }
    true
}

/// A declared field type that is not Copy: text, collections, and user types not known to be Copy
/// (a user Copy struct counts as non-Copy here; owned `self` then only costs nothing).
pub fn type_is_non_copy(t: &Type) -> bool {
    match t {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => false,
        Type::Reference(_) | Type::FunctionPointer { .. } | Type::RawPointer { .. } => false,
        Type::Option(inner) | Type::Array(inner, _) => type_is_non_copy(inner),
        Type::Tuple(ts) => ts.iter().any(type_is_non_copy),
        Type::Custom(name) => {
            !(crate::type_classification::is_copy_primitive(name) || crate::type_classification::is_known_copy_aggregate(name))
        }
        _ => true,
    }
}

/// Field types of the method's own type, for [`self_owned_after_last_field_move`].
pub fn field_non_copy<'a>(fields: Option<&'a HashMap<String, Type>>) -> impl Fn(&str) -> bool + 'a {
    move |f: &str| fields.and_then(|m| m.get(f)).is_some_and(type_is_non_copy)
}

impl Walk<'_> {
    fn block(&mut self, stmts: &[&Statement<'_>], value_tail: bool) {
        for (i, s) in stmts.iter().enumerate() {
            self.stmt(s, value_tail && i + 1 == stmts.len());
        }
    }

    /// A block expression: its tail goes where the block's value goes.
    fn block_at(&mut self, stmts: &[&Statement<'_>], pos: Pos) {
        for (i, st) in stmts.iter().enumerate() {
            let last = i + 1 == stmts.len();
            match st {
                Statement::Expression { expr, .. } if last => self.expr(expr, pos),
                _ => self.stmt(st, false),
            }
        }
    }

    fn looped(&mut self, f: impl FnOnce(&mut Self)) {
        self.depth += 1;
        f(self);
        self.depth -= 1;
    }

    fn stmt(&mut self, s: &Statement<'_>, tail: bool) {
        match s {
            Statement::Let { value, else_block, .. } => {
                self.expr(value, Pos::Stored);
                if let Some(b) = else_block {
                    self.block(b, false);
                }
            }
            Statement::Assignment { target, value, .. } => {
                self.expr(value, Pos::Stored);
                self.expr(target, Pos::Read);
            }
            Statement::Return { value, .. } => value.iter().for_each(|v| self.expr(v, Pos::Result)),
            Statement::Expression { expr, .. } => self.expr(expr, if tail { Pos::Result } else { Pos::Read }),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition, Pos::Read);
                self.block(then_block, tail);
                if let Some(b) = else_block {
                    self.block(b, tail);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr(value, Pos::Read);
                for a in arms {
                    a.guard.iter().for_each(|g| self.expr(g, Pos::Read));
                    self.expr(a.body, if tail { Pos::Result } else { Pos::Read });
                }
            }
            Statement::For { iterable, body, .. } => {
                self.expr(iterable, Pos::Read);
                self.looped(|w| w.block(body, false));
            }
            Statement::While { condition, body, .. } => self.looped(|w| {
                w.expr(condition, Pos::Read);
                w.block(body, false);
            }),
            Statement::Loop { body, .. } => self.looped(|w| w.block(body, false)),
            Statement::Break { value, .. } => value.iter().for_each(|v| self.expr(v, Pos::Stored)),
            Statement::Continue { .. } | Statement::Use { .. } => {}
            Statement::Const { value, .. } | Statement::Static { value, .. } => self.expr(value, Pos::Read),
            Statement::Thread { .. } | Statement::Async { .. } | Statement::Defer { .. } => self.unsupported = true,
        }
    }

    fn expr(&mut self, e: &Expression<'_>, pos: Pos) {
        match e {
            Expression::Identifier { name, .. } if name == "self" => self.uses.push(Use::Whole),
            Expression::Identifier { .. } | Expression::Literal { .. } => {}
            Expression::FieldAccess { object, field, .. } => match &**object {
                Expression::Identifier { name, .. } if name == "self" => {
                    if pos != Pos::Read && (self.non_copy)(field) {
                        if self.depth > 0 {
                            self.unsupported = true;
                        }
                        self.getter |= pos == Pos::Result;
                        self.uses.push(Use::Move(field.clone()));
                    } else {
                        self.uses.push(Use::Read(field.clone()));
                    }
                }
                _ => {
                    // A deeper path `self.a.b` in an owned position would partly move `self.a`.
                    if pos != Pos::Read && self_field_root(object).is_some() {
                        self.unsupported = true;
                    }
                    self.expr(object, Pos::Read)
                }
            },
            Expression::MethodCall { object, method, arguments, .. } => {
                if let Expression::Identifier { name, .. } = &**object {
                    self.calls.push((name.clone(), method.clone()));
                }
                self.expr(object, Pos::Read);
                arguments.iter().for_each(|(_, a)| self.expr(a, Pos::Read));
            }
            Expression::Call { function, arguments, .. } => {
                self.expr(function, Pos::Read);
                arguments.iter().for_each(|(_, a)| self.expr(a, Pos::Read));
            }
            Expression::StructLiteral { fields, .. } => fields.iter().for_each(|(_, f)| self.expr(f, Pos::Stored)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                elements.iter().for_each(|x| self.expr(x, Pos::Stored))
            }
            Expression::Block { statements, .. } => self.block_at(statements, pos),
            Expression::Closure { body, .. } => self.looped(|w| w.expr(body, Pos::Read)),
            Expression::Binary { left, right, .. } => {
                self.expr(left, Pos::Read);
                self.expr(right, Pos::Read);
            }
            Expression::Unary { operand, .. } => self.expr(operand, Pos::Read),
            Expression::Range { start, end, .. } => {
                self.expr(start, Pos::Read);
                self.expr(end, Pos::Read);
            }
            Expression::Index { object, index, .. } => {
                self.expr(object, Pos::Read);
                self.expr(index, Pos::Read);
            }
            Expression::MapLiteral { pairs, .. } => pairs.iter().for_each(|(k, v)| {
                self.expr(k, Pos::Stored);
                self.expr(v, Pos::Stored);
            }),
            Expression::MacroInvocation { args, .. } => args.iter().for_each(|x| self.expr(x, Pos::Read)),
            Expression::Cast { expr, .. }
            | Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. } => self.expr(expr, Pos::Read),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr(channel, Pos::Read);
                self.expr(value, Pos::Stored);
            }
            Expression::ChannelRecv { channel, .. } => self.expr(channel, Pos::Read),
        }
    }
}

fn self_field_root(e: &Expression<'_>) -> Option<()> {
    match e {
        Expression::Identifier { name, .. } if name == "self" => Some(()),
        Expression::FieldAccess { object, .. } => self_field_root(object),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decide(src: &str, method: &str, non_copy: &[&str]) -> bool {
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let decl = program
            .items
            .iter()
            .find_map(|i| match i {
                crate::parser::Item::Impl { block, .. } => block.functions.iter().find(|f| f.name == method),
                _ => None,
            })
            .expect("method");
        let nc = |f: &str| non_copy.contains(&f);
        self_owned_after_last_field_move(&decl.body, decl.return_type.is_some(), &nc)
    }

    const SRC: &str = "struct B { items: Vec<int>, n: int }\nstruct O { items: Vec<int>, n: int }\nimpl B {\n fn wither(self) -> O { O { items: self.items, n: self.n } }\n fn read_after(self) -> O { let o = O { items: self.items, n: 1 }\n  let k = self\n  o }\n fn reread(self) -> int { let v = self.items\n  self.items.len() }\n fn looped(self) -> int { let mut t = 0\n  while t < 1 { let v = self.items\n   t = t + 1 }\n  t }\n fn only_copy(self) -> int { self.n }\n fn getter(self) -> Vec<int> { self.items }\n}\n";

    #[test]
    fn a_wither_moving_its_fields_last_is_owned() {
        assert!(decide(SRC, "wither", &["items"]));
    }

    #[test]
    fn a_read_after_the_last_move_keeps_the_borrow() {
        assert!(!decide(SRC, "read_after", &["items"]));
        assert!(!decide(SRC, "reread", &["items"]));
    }

    #[test]
    fn a_move_in_a_loop_or_of_copy_fields_only_keeps_the_borrow() {
        assert!(!decide(SRC, "looped", &["items"]));
        assert!(!decide(SRC, "only_copy", &["items"]));
    }

    #[test]
    fn a_getter_returning_a_field_keeps_the_borrow() {
        assert!(!decide(SRC, "getter", &["items"]));
    }
}

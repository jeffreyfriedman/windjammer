//! Benchmark experiment (convergence C5, off by default, removed before merge): SELF-CONSUME-FIELDS, the
//! candidate rule "`self` is owned when it is dead after its last field move and the method never writes
//! `self`". Decided from IR facts: the solved receiver mode (not `&mut`, not a mutated parameter) and the
//! body's `self` uses in evaluation order. Enabled by `IrPipelineConfig::self_owned_after_last_field_move`.
//!
//! A field move is a direct `self.f` of a non-Copy declared type in an owned position: a struct-literal
//! field, a tuple or array element, a `let` or assignment value, a `return` or the function's value. The
//! rule holds when there is at least one, none is inside a loop or closure, no field is moved twice or read
//! after its move, and `self` is not used whole after the first move: after the last move `self` is dead
//! except for reads of fields it did not move (a partly moved value, which Rust allows).

use std::collections::{HashMap, HashSet};

use crate::parser::ast::core::{Expression, Statement};
use crate::parser::Type;

#[derive(Debug, Clone, PartialEq)]
enum Use {
    Move(String),
    Read(String),
    Whole,
}

struct Walk<'f> {
    uses: Vec<Use>,
    depth: usize,
    unsupported: bool,
    non_copy: &'f dyn Fn(&str) -> bool,
}

/// True when the rule makes `self` owned. `field_is_non_copy` answers from the declared field type.
pub fn self_owned_after_last_field_move(body: &[&Statement<'_>], returns_value: bool, field_is_non_copy: &dyn Fn(&str) -> bool) -> bool {
    let mut w = Walk { uses: Vec::new(), depth: 0, unsupported: false, non_copy: field_is_non_copy };
    w.block(body, returns_value);
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

    fn looped(&mut self, f: impl FnOnce(&mut Self)) {
        self.depth += 1;
        f(self);
        self.depth -= 1;
    }

    fn stmt(&mut self, s: &Statement<'_>, tail: bool) {
        match s {
            Statement::Let { value, else_block, .. } => {
                self.expr(value, true);
                if let Some(b) = else_block {
                    self.block(b, false);
                }
            }
            Statement::Assignment { target, value, .. } => {
                self.expr(value, true);
                self.expr(target, false);
            }
            Statement::Return { value, .. } => value.iter().for_each(|v| self.expr(v, true)),
            Statement::Expression { expr, .. } => self.expr(expr, tail),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expr(condition, false);
                self.block(then_block, tail);
                if let Some(b) = else_block {
                    self.block(b, tail);
                }
            }
            Statement::Match { value, arms, .. } => {
                self.expr(value, false);
                for a in arms {
                    a.guard.iter().for_each(|g| self.expr(g, false));
                    self.expr(a.body, tail);
                }
            }
            Statement::For { iterable, body, .. } => {
                self.expr(iterable, false);
                self.looped(|w| w.block(body, false));
            }
            Statement::While { condition, body, .. } => self.looped(|w| {
                w.expr(condition, false);
                w.block(body, false);
            }),
            Statement::Loop { body, .. } => self.looped(|w| w.block(body, false)),
            Statement::Break { value, .. } => value.iter().for_each(|v| self.expr(v, true)),
            Statement::Continue { .. } | Statement::Use { .. } => {}
            Statement::Const { value, .. } | Statement::Static { value, .. } => self.expr(value, false),
            Statement::Thread { .. } | Statement::Async { .. } | Statement::Defer { .. } => self.unsupported = true,
        }
    }

    fn expr(&mut self, e: &Expression<'_>, owned: bool) {
        match e {
            Expression::Identifier { name, .. } if name == "self" => self.uses.push(Use::Whole),
            Expression::Identifier { .. } | Expression::Literal { .. } => {}
            Expression::FieldAccess { object, field, .. } => match &**object {
                Expression::Identifier { name, .. } if name == "self" => {
                    if owned && (self.non_copy)(field) {
                        if self.depth > 0 {
                            self.unsupported = true;
                        }
                        self.uses.push(Use::Move(field.clone()));
                    } else {
                        self.uses.push(Use::Read(field.clone()));
                    }
                }
                _ => {
                    // A deeper path `self.a.b` in an owned position would partly move `self.a`.
                    if owned && self_field_root(object).is_some() {
                        self.unsupported = true;
                    }
                    self.expr(object, false)
                }
            },
            Expression::MethodCall { object, arguments, .. } => {
                self.expr(object, false);
                arguments.iter().for_each(|(_, a)| self.expr(a, false));
            }
            Expression::Call { function, arguments, .. } => {
                self.expr(function, false);
                arguments.iter().for_each(|(_, a)| self.expr(a, false));
            }
            Expression::StructLiteral { fields, .. } => fields.iter().for_each(|(_, f)| self.expr(f, true)),
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                elements.iter().for_each(|x| self.expr(x, true))
            }
            Expression::Block { statements, .. } => self.block(statements, owned),
            Expression::Closure { body, .. } => self.looped(|w| w.expr(body, false)),
            Expression::Binary { left, right, .. } => {
                self.expr(left, false);
                self.expr(right, false);
            }
            Expression::Unary { operand, .. } => self.expr(operand, false),
            Expression::Range { start, end, .. } => {
                self.expr(start, false);
                self.expr(end, false);
            }
            Expression::Index { object, index, .. } => {
                self.expr(object, false);
                self.expr(index, false);
            }
            Expression::MapLiteral { pairs, .. } => pairs.iter().for_each(|(k, v)| {
                self.expr(k, true);
                self.expr(v, true);
            }),
            Expression::MacroInvocation { args, .. } => args.iter().for_each(|x| self.expr(x, false)),
            Expression::Cast { expr, .. }
            | Expression::TryOp { expr, .. }
            | Expression::Await { expr, .. }
            | Expression::AsyncCall { expr, .. }
            | Expression::SpawnCall { expr, .. } => self.expr(expr, false),
            Expression::ChannelSend { channel, value, .. } => {
                self.expr(channel, false);
                self.expr(value, true);
            }
            Expression::ChannelRecv { channel, .. } => self.expr(channel, false),
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

    const SRC: &str = "struct B { items: Vec<int>, n: int }\nstruct O { items: Vec<int>, n: int }\nimpl B {\n fn wither(self) -> O { O { items: self.items, n: self.n } }\n fn read_after(self) -> O { let o = O { items: self.items, n: 1 }\n  let k = self\n  o }\n fn reread(self) -> int { let v = self.items\n  self.items.len() }\n fn looped(self) -> int { let mut t = 0\n  while t < 1 { let v = self.items\n   t = t + 1 }\n  t }\n fn only_copy(self) -> int { self.n }\n}\n";

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
}

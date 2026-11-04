//! W0016: a trait method without a body must declare its receiver mode.
//!
//! `self` alone means "inferred from the body", and a bodyless trait method has no body: the receiver of
//! the trait (and of every impl, which must match it) would otherwise be guessed before the impls are
//! seen, which breaks an impl in another module that mutates through `self`. The declaration is the
//! single source: `&self` or `&mut self` (`self` stays by-value when the return type mentions `Self`).

use crate::parser::ast::core::{Item, Program};
use crate::parser::ast::ownership::OwnershipHint;
use crate::source_map::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndeclaredReceiver {
    /// `Trait::method`
    pub method: String,
    pub location: Option<Location>,
}

/// Bodyless trait methods whose `self` receiver carries no declared mode, in source order.
pub fn undeclared_receivers(program: &Program) -> Vec<UndeclaredReceiver> {
    let mut out = Vec::new();
    collect(&program.items, &mut out);
    out
}

fn collect(items: &[Item], out: &mut Vec<UndeclaredReceiver>) {
    for item in items {
        match item {
            Item::Trait { decl, location } => {
                for m in decl.methods.iter().filter(|m| m.body.is_none()) {
                    let undeclared = m
                        .parameters
                        .first()
                        .is_some_and(|p| p.name == "self" && p.ownership == OwnershipHint::Inferred);
                    if undeclared {
                        out.push(UndeclaredReceiver { method: format!("{}::{}", decl.name, m.name), location: location.clone() });
                    }
                }
            }
            Item::Mod { items, .. } => collect(items, out),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn methods(src: &str) -> Vec<String> {
        let tokens = crate::lexer::Lexer::new(src).tokenize_with_locations();
        let mut parser = crate::parser::Parser::new_with_source(tokens, "t.wj".to_string(), src.to_string());
        let program = parser.parse().expect("parses");
        undeclared_receivers(&program).into_iter().map(|u| u.method).collect()
    }

    #[test]
    fn bodyless_plain_self_is_reported_declared_modes_are_not() {
        let src = "pub trait Bin {\n    fn get(self, k: u64) -> u64\n    fn look(&self) -> u64\n    fn put(&mut self, k: u64)\n    fn into_inner(self) -> Self\n    fn make() -> u64\n    fn dflt(self) -> u64 {\n        1\n    }\n}\n";
        assert_eq!(methods(src), vec!["Bin::get".to_string()]);
    }
}

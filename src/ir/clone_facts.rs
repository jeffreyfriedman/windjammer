//! CLONE-BORROWED-TO-OWNED sites (convergence plan C6), per site, from the solver's result.
//!
//! A function's value is owned (RET-OWNED, RET-NO-BORROW). When that value is a field of a
//! parameter (or `self`) that the solver resolved as a borrow, the field must be cloned there:
//! it cannot move out from behind the reference. This module finds the candidate sites (field
//! projections of a parameter that are the function's value: the tail, the tails of a trailing
//! `if`/`match`, and every `return`) and keeps those whose root the solver borrowed.
//!
//! Data only, keyed by AST node identity. The Rust backend prints a clone exactly at these sites
//! (when the field type is not Copy) and nowhere else for the same candidates.

use super::numeric_defaults::node_id;
use super::safety_type::{OwnedType, SafetyType};
use crate::parser::ast::core::{Expression, Statement};
use std::collections::{BTreeMap, HashMap};

/// Candidate sites: returned field projections of a parameter, by node, with the root name.
pub fn returned_param_fields(params: &[&str], body: &[&Statement<'_>], returns_value: bool) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    if returns_value {
        tail(params, body, &mut out);
    }
    returns(params, body, &mut out);
    out
}

/// The sites among the candidates that need a clone: the solver borrowed their root.
pub fn borrowed_returned_fields(
    candidates: &BTreeMap<usize, String>,
    param_types: &HashMap<String, SafetyType>,
) -> BTreeMap<usize, String> {
    candidates
        .iter()
        .filter(|(_, root)| {
            param_types
                .get(root.as_str())
                .is_some_and(|t| matches!(t.ownership, OwnedType::Ref(_) | OwnedType::MutRef(_)))
        })
        .map(|(n, r)| (*n, r.clone()))
        .collect()
}

fn param_field_root(params: &[&str], e: &Expression<'_>) -> Option<String> {
    let Expression::FieldAccess { object, .. } = e else { return None };
    let mut cur: &Expression<'_> = object;
    loop {
        match cur {
            Expression::FieldAccess { object, .. } => cur = object,
            Expression::Identifier { name, .. } if params.contains(&name.as_str()) => return Some(name.clone()),
            _ => return None,
        }
    }
}

fn value(params: &[&str], e: &Expression<'_>, out: &mut BTreeMap<usize, String>) {
    match e {
        Expression::Block { statements, .. } => tail(params, statements, out),
        _ => {
            if let Some(root) = param_field_root(params, e) {
                out.insert(node_id(e), root);
            }
        }
    }
}

fn tail(params: &[&str], body: &[&Statement<'_>], out: &mut BTreeMap<usize, String>) {
    match body.last() {
        Some(Statement::Expression { expr, .. }) => value(params, expr, out),
        Some(Statement::If { then_block, else_block: Some(e), .. }) => {
            tail(params, then_block, out);
            tail(params, e, out);
        }
        Some(Statement::Match { arms, .. }) => arms.iter().for_each(|a| value(params, a.body, out)),
        _ => {}
    }
}

fn returns(params: &[&str], body: &[&Statement<'_>], out: &mut BTreeMap<usize, String>) {
    for s in body {
        match s {
            Statement::Return { value: Some(v), .. } => value(params, v, out),
            Statement::If { then_block, else_block, .. } => {
                returns(params, then_block, out);
                if let Some(b) = else_block {
                    returns(params, b, out);
                }
            }
            Statement::For { body, .. } | Statement::While { body, .. } | Statement::Loop { body, .. } => {
                returns(params, body, out)
            }
            Statement::Match { arms, .. } => arms.iter().for_each(|a| {
                if let Expression::Block { statements, .. } = a.body {
                    returns(params, statements, out);
                }
            }),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl_of(src: &str, name: &str) -> &'static crate::parser::FunctionDecl<'static> {
        let mut lexer = crate::lexer::Lexer::new(src);
        let parser = Box::leak(Box::new(crate::parser::Parser::new(lexer.tokenize_with_locations())));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        fn find<'a>(items: &'a [crate::parser::Item<'a>], name: &str) -> Option<&'a crate::parser::FunctionDecl<'a>> {
            items.iter().find_map(|i| match i {
                crate::parser::Item::Function { decl, .. } if decl.name == name => Some(decl),
                crate::parser::Item::Impl { block, .. } => block.functions.iter().find(|f| f.name == name),
                _ => None,
            })
        }
        find(&program.items, name).expect("fn")
    }

    #[test]
    fn tails_and_returns_of_parameter_fields_are_candidates() {
        let src = "struct Q { title: string, n: int }\nfn pick(q: Q, b: bool) -> string {\n if b { return q.title }\n let t = q.title\n t\n}\nfn tail(q: Q) -> string { q.title }\nfn other(q: Q) -> string { \"x\" }\n";
        let d = decl_of(src, "pick");
        let ps: Vec<&str> = d.parameters.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(returned_param_fields(&ps, &d.body, true).len(), 1, "the `return q.title`; `let t = q.title` is not a return");
        let d = decl_of(src, "tail");
        let ps: Vec<&str> = d.parameters.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(returned_param_fields(&ps, &d.body, true).values().cloned().collect::<Vec<_>>(), vec!["q".to_string()]);
        let d = decl_of(src, "other");
        assert!(returned_param_fields(&["q"], &d.body, true).is_empty());
    }

    #[test]
    fn only_borrowed_roots_need_a_clone() {
        let mut cands = BTreeMap::new();
        cands.insert(1, "q".to_string());
        cands.insert(2, "o".to_string());
        let mut tys = HashMap::new();
        tys.insert("q".to_string(), SafetyType { ownership: OwnedType::Ref(super::super::safety_type::Region(0)), ..SafetyType::owned(super::super::safety_type::BaseType::Unit) });
        tys.insert("o".to_string(), SafetyType::owned(super::super::safety_type::BaseType::Unit));
        assert_eq!(borrowed_returned_fields(&cands, &tys).keys().copied().collect::<Vec<_>>(), vec![1]);
    }
}

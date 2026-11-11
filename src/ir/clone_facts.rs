//! CLONE-BORROWED-TO-OWNED sites (convergence plan C6), per site, from the solver's result.
//!
//! A function's value is owned (RET-OWNED, RET-NO-BORROW). When that value is a field of a
//! parameter (or `self`) that the solver resolved as a borrow, the field must be cloned there:
//! it cannot move out from behind the reference. This module finds the candidate sites (field
//! projections of a parameter consumed as owned values: the function's value, every `return`, an
//! assignment value, a struct-literal field, an array/tuple element) and keeps those whose root the
//! solver borrowed. Element reads (`v[i]`) in the same positions are cloned unless Copy.
//!
//! Data only, keyed by AST node identity. The Rust backend prints a clone exactly at these sites
//! (when the field type is not Copy) and nowhere else for the same candidates.

use super::numeric_defaults::node_id;
use super::safety_type::{OwnedType, SafetyType};
use crate::parser::ast::core::{Expression, Statement};
use std::collections::{BTreeMap, HashMap};

/// Candidate sites: field projections of a parameter consumed as owned values (the function's
/// value, a `return`, an assignment value, a struct-literal field, an array/tuple element), by
/// node, with the root name. `returns_value`: the function's tail is its value.
pub fn returned_param_fields(params: &[&str], body: &[&Statement<'_>], returns_value: bool) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    element_stmts(body, returns_value, &|e, _| param_field_root(params, e), &mut out);
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

/// Element reads (`v[i]`, `self.items[i]`) consumed as an owned value: an assignment value, the
/// function's value or a `return`, a struct-literal field, an array/tuple element.
/// An element cannot move out of its collection, so a non-Copy element is cloned there
/// (CLONE-BORROWED-TO-OWNED); a Copy element is copied. Call arguments are not included: the
/// callee's formal decides (OWN-CALL-FORMAL). By node, with the root binding.
pub fn owned_element_reads(body: &[&Statement<'_>], returns_value: bool) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    element_stmts(body, returns_value, &|e, _| element_root(e), &mut out);
    out
}

/// CLONE-USE-AFTER-MOVE for partial moves: a field path of a binding (not `self`) consumed as an
/// owned value (a struct-literal field, a returned or assigned value, a literal element) inside a
/// statement after which liveness says the binding is read again (`read_after`, by statement node).
/// Moving the field would leave the binding partly moved, so the field is copied there.
pub fn field_copy_sites(
    body: &[&Statement<'_>],
    read_after: &BTreeMap<usize, std::collections::BTreeSet<String>>,
) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    let pick = |e: &Expression<'_>, stmt: usize| {
        let root = binding_field_root(e)?;
        read_after.get(&stmt).is_some_and(|names| names.contains(&root)).then_some(root)
    };
    element_stmts(body, true, &pick, &mut out);
    // Within one literal the elements evaluate in order: `Hit { id: node.id, d: dist(node) }` moves
    // `node.id` before `node` is read whole by a later element.
    let mut literals = Vec::new();
    body.iter().for_each(|s| stmt_literals(s, &mut literals));
    for elems in literals {
        for (i, e) in elems.iter().enumerate() {
            if let Some(root) = binding_field_root(e) {
                if elems[i + 1..].iter().any(|later| reads_whole(later, &root)) {
                    out.insert(node_id(e), root);
                }
            }
        }
    }
    out
}

/// The element lists of every struct, tuple and array literal in a statement.
fn stmt_literals<'a, 'ast>(s: &'a Statement<'ast>, out: &mut Vec<Vec<&'a Expression<'ast>>>) {
    match s {
        Statement::Let { value, .. } | Statement::Assignment { value, .. } | Statement::Return { value: Some(value), .. } => {
            expr_literals(value, out)
        }
        Statement::Expression { expr, .. } => expr_literals(expr, out),
        Statement::If { condition, then_block, else_block, .. } => {
            expr_literals(condition, out);
            then_block.iter().chain(else_block.iter().flatten()).for_each(|t| stmt_literals(t, out));
        }
        Statement::For { body, .. } | Statement::While { body, .. } | Statement::Loop { body, .. } => {
            body.iter().for_each(|t| stmt_literals(t, out))
        }
        Statement::Match { value, arms, .. } => {
            expr_literals(value, out);
            arms.iter().for_each(|a| expr_literals(a.body, out));
        }
        _ => {}
    }
}

fn expr_literals<'a, 'ast>(e: &'a Expression<'ast>, out: &mut Vec<Vec<&'a Expression<'ast>>>) {
    match e {
        Expression::StructLiteral { fields, .. } => {
            out.push(fields.iter().map(|(_, f)| &**f).collect());
            fields.iter().for_each(|(_, f)| expr_literals(f, out));
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            out.push(elements.iter().map(|x| &**x).collect());
            elements.iter().for_each(|x| expr_literals(x, out));
        }
        Expression::Call { arguments, .. } => arguments.iter().for_each(|(_, a)| expr_literals(a, out)),
        Expression::MethodCall { object, arguments, .. } => {
            expr_literals(object, out);
            arguments.iter().for_each(|(_, a)| expr_literals(a, out));
        }
        Expression::Block { statements, .. } => statements.iter().for_each(|s| stmt_literals(s, out)),
        _ => {}
    }
}

/// `e` reads `root` whole (not only through a field projection `root.f`).
fn reads_whole(e: &Expression<'_>, root: &str) -> bool {
    match e {
        Expression::Identifier { name, .. } => name == root,
        Expression::FieldAccess { object, .. } => {
            !matches!(&**object, Expression::Identifier { .. }) && reads_whole(object, root)
        }
        Expression::Call { function, arguments, .. } => {
            reads_whole(function, root) || arguments.iter().any(|(_, a)| reads_whole(a, root))
        }
        Expression::MethodCall { object, arguments, .. } => {
            reads_whole(object, root) || arguments.iter().any(|(_, a)| reads_whole(a, root))
        }
        Expression::Binary { left, right, .. } => reads_whole(left, root) || reads_whole(right, root),
        Expression::Unary { operand, .. } => reads_whole(operand, root),
        Expression::Cast { expr, .. } | Expression::TryOp { expr, .. } => reads_whole(expr, root),
        Expression::Index { object, index, .. } => reads_whole(object, root) || reads_whole(index, root),
        Expression::StructLiteral { fields, .. } => fields.iter().any(|(_, f)| reads_whole(f, root)),
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements.iter().any(|x| reads_whole(x, root)),
        Expression::MacroInvocation { args, .. } => args.iter().any(|x| reads_whole(x, root)),
        _ => false,
    }
}

fn binding_field_root(e: &Expression<'_>) -> Option<String> {
    let Expression::FieldAccess { object, .. } = e else { return None };
    let mut cur: &Expression<'_> = object;
    loop {
        match cur {
            Expression::FieldAccess { object, .. } => cur = object,
            Expression::Identifier { name, .. } if name != "self" => return Some(name.clone()),
            _ => return None,
        }
    }
}

fn element_root(e: &Expression<'_>) -> Option<String> {
    let Expression::Index { object, .. } = e else { return None };
    let mut cur: &Expression<'_> = object;
    loop {
        match cur {
            Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => cur = object,
            Expression::Identifier { name, .. } => return Some(name.clone()),
            _ => return None,
        }
    }
}

/// Recognizes a site: the expression and the innermost statement holding it (by node identity).
type Pick<'p> = &'p dyn Fn(&Expression<'_>, usize) -> Option<String>;

fn element_value(e: &Expression<'_>, stmt: usize, pick: Pick, out: &mut BTreeMap<usize, String>) {
    if let Some(root) = pick(e, stmt) {
        out.insert(node_id(e), root);
        return;
    }
    match e {
        Expression::Block { statements, .. } => element_stmts(statements, true, pick, out),
        Expression::StructLiteral { fields, .. } => fields.iter().for_each(|(_, f)| element_value(f, stmt, pick, out)),
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            elements.iter().for_each(|x| element_value(x, stmt, pick, out))
        }
        Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
            // Not the arguments themselves (the formal decides), but values built inside them.
            arguments.iter().for_each(|(_, a)| nested_values(a, stmt, pick, out))
        }
        _ => nested_values(e, stmt, pick, out),
    }
}

/// Owned positions nested in an expression that is not itself one (`Some(Row { a: v[0] })`).
fn nested_values(e: &Expression<'_>, stmt: usize, pick: Pick, out: &mut BTreeMap<usize, String>) {
    match e {
        Expression::StructLiteral { .. } | Expression::Tuple { .. } | Expression::Array { .. } | Expression::Block { .. } => {
            element_value(e, stmt, pick, out)
        }
        Expression::Call { arguments, .. } | Expression::MethodCall { arguments, .. } => {
            arguments.iter().for_each(|(_, a)| nested_values(a, stmt, pick, out))
        }
        _ => {}
    }
}

fn element_stmts(body: &[&Statement<'_>], value_tail: bool, pick: Pick, out: &mut BTreeMap<usize, String>) {
    for (i, s) in body.iter().enumerate() {
        let is_tail = value_tail && i + 1 == body.len();
        let stmt = *s as *const Statement<'_> as *const () as usize;
        match s {
            // A `let` initialiser is not included: the binding may borrow the element
            // (`let node = &nodes[0]` when it is only read), which the local's ownership decides.
            Statement::Let { value, .. } => nested_values(value, stmt, pick, out),
            Statement::Assignment { value, .. } => element_value(value, stmt, pick, out),
            Statement::Return { value: Some(v), .. } => element_value(v, stmt, pick, out),
            Statement::Expression { expr, .. } if is_tail => element_value(expr, stmt, pick, out),
            Statement::Expression { expr, .. } => nested_values(expr, stmt, pick, out),
            Statement::If { then_block, else_block, .. } => {
                element_stmts(then_block, is_tail, pick, out);
                if let Some(b) = else_block {
                    element_stmts(b, is_tail, pick, out);
                }
            }
            Statement::For { body, .. } | Statement::While { body, .. } | Statement::Loop { body, .. } => {
                element_stmts(body, false, pick, out)
            }
            Statement::Match { arms, .. } => arms.iter().for_each(|a| {
                if is_tail {
                    element_value(a.body, stmt, pick, out)
                } else {
                    nested_values(a.body, stmt, pick, out)
                }
            }),
            _ => {}
        }
    }
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
    fn owned_element_reads_are_value_positions_not_call_arguments() {
        let src = "struct Row { a: string }\nfn f(v: Vec<string>) -> Row {\n let x = v[0]\n take(v[1])\n let n = v[2].len()\n Row { a: v[3] }\n}\nfn take(s: string) {}\n";
        let d = decl_of(src, "f");
        let reads = owned_element_reads(&d.body, true);
        assert_eq!(reads.len(), 1, "the struct field `v[3]`; not a let initialiser, an argument or a receiver");
        assert!(reads.values().all(|r| r == "v"));
    }

    #[test]
    fn a_field_of_a_binding_read_again_is_a_copy_site() {
        let src = "struct P { id: string, note: string }\nstruct E { id: string }\nfn f(p: P) -> P {\n let e = E { id: p.id }\n let n = E { id: p.note }\n p\n}\nfn g(p: P) -> E {\n E { id: p.note }\n}\n";
        let d = decl_of(src, "f");
        let ps: Vec<&str> = d.parameters.iter().map(|p| p.name.as_str()).collect();
        let ra = crate::ir::liveness::read_after(&ps, &d.body, false);
        assert_eq!(field_copy_sites(&d.body, &ra).len(), 2, "`p` is returned after both field reads");
        let d = decl_of(src, "g");
        let ps: Vec<&str> = d.parameters.iter().map(|p| p.name.as_str()).collect();
        let ra = crate::ir::liveness::read_after(&ps, &d.body, false);
        assert!(field_copy_sites(&d.body, &ra).is_empty(), "`p` is not read after its field moves");
    }

    #[test]
    fn a_field_moved_before_a_later_literal_element_reads_the_binding_is_a_copy_site() {
        let src = "struct N { id: string, x: f64 }\nstruct H { id: string, d: f64 }\nfn d(n: N) -> f64 { n.x }\nfn f(n: N) -> H {\n H { id: n.id, d: d(n) }\n}\nfn g(n: N) -> H {\n H { id: n.id, d: n.x }\n}\n";
        let d = decl_of(src, "f");
        let ps: Vec<&str> = d.parameters.iter().map(|p| p.name.as_str()).collect();
        let ra = crate::ir::liveness::read_after(&ps, &d.body, false);
        assert_eq!(field_copy_sites(&d.body, &ra).len(), 1, "`d(n)` reads `n` whole after `n.id` moved");
        let d = decl_of(src, "g");
        let ps: Vec<&str> = d.parameters.iter().map(|p| p.name.as_str()).collect();
        let ra = crate::ir::liveness::read_after(&ps, &d.body, false);
        assert!(field_copy_sites(&d.body, &ra).is_empty(), "`n.x` reads another field: distinct fields may move");
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

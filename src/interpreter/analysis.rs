//! Inferred-ownership analysis: which parameters does a function body mutate?
//!
//! Windjammer infers `&mut` for parameters the body mutates; the interpreter mirrors that by
//! passing a `&mut` reference to the caller's place for exactly those parameters.

use super::engine::{FuncDef, Interpreter};
use super::call::Resolved;
use crate::parser::{EnumPatternBinding, Expression, Pattern, Statement, Type, UnaryOp};
use std::collections::HashSet;
use std::rc::Rc;

pub fn analyze(it: &mut Interpreter, f: &Rc<FuncDef>) -> (Vec<bool>, bool) {
    let mut set: HashSet<&'static str> = HashSet::new();
    for s in f.body {
        scan_stmt(it, f.module, s, &mut set);
    }
    let mut out = Vec::new();
    for p in f.params.iter().filter(|p| p.name != "self") {
        let by_type = matches!(p.type_, Type::MutableReference(_));
        out.push(by_type || set.contains(p.name.as_str()));
    }
    let self_mut = set.contains("self")
        || f.params.first().map(|p| p.name == "self" && p.ownership == crate::parser::OwnershipHint::Mut).unwrap_or(false);
    // a method that returns `self` (builder style) consumes it instead of borrowing
    let returns_self = matches!(f.ret, Some(Type::Custom(n)) if n == "Self" || Some(n.as_str()) == f.self_ty.as_deref());
    (out, self_mut && !returns_self)
}

fn root(e: &'static Expression<'static>) -> Option<&'static str> {
    match e {
        Expression::Identifier { name, .. } => Some(name.as_str()),
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => root(object),
        Expression::Unary { op: UnaryOp::Deref | UnaryOp::MutRef | UnaryOp::Ref, operand, .. } => root(operand),
        Expression::MethodCall { object, method, .. }
            if matches!(
                method.as_str(),
                "get_mut" | "iter_mut" | "last_mut" | "first_mut" | "unwrap" | "expect" | "entry" | "or_insert" | "or_insert_with" | "or_default" | "as_mut" | "borrow_mut" | "values_mut"
            ) =>
        {
            root(object)
        }
        _ => None,
    }
}

fn pattern_has_ref_mut(p: &Pattern<'static>) -> bool {
    match p {
        Pattern::RefMut(_) => true,
        Pattern::Tuple(ps) | Pattern::Or(ps) => ps.iter().any(pattern_has_ref_mut),
        Pattern::Reference(i) => pattern_has_ref_mut(i),
        Pattern::EnumVariant(_, b) => match b {
            EnumPatternBinding::Tuple(ps) => ps.iter().any(pattern_has_ref_mut),
            EnumPatternBinding::Struct(fs, _) => fs.iter().any(|(_, p)| pattern_has_ref_mut(p)),
            _ => false,
        },
        _ => false,
    }
}

fn scan_stmt(it: &mut Interpreter, module: usize, s: &'static Statement<'static>, out: &mut HashSet<&'static str>) {
    match s {
        Statement::Let { value, else_block, .. } => {
            scan_expr(it, module, value, out);
            if let Some(b) = else_block {
                for x in b {
                    scan_stmt(it, module, x, out);
                }
            }
        }
        Statement::Const { value, .. } | Statement::Static { value, .. } => scan_expr(it, module, value, out),
        Statement::Assignment { target, value, .. } => {
            if let Some(r) = root(target) {
                out.insert(r);
            }
            scan_expr(it, module, target, out);
            scan_expr(it, module, value, out);
        }
        Statement::Return { value, .. } => {
            if let Some(v) = value {
                scan_expr(it, module, v, out);
            }
        }
        Statement::Expression { expr, .. } => scan_expr(it, module, expr, out),
        Statement::If { condition, then_block, else_block, .. } => {
            scan_expr(it, module, condition, out);
            for x in then_block {
                scan_stmt(it, module, x, out);
            }
            if let Some(b) = else_block {
                for x in b {
                    scan_stmt(it, module, x, out);
                }
            }
        }
        Statement::Match { value, arms, .. } => {
            scan_expr(it, module, value, out);
            if arms.iter().any(|a| pattern_has_ref_mut(&a.pattern)) {
                if let Some(r) = root(value) {
                    out.insert(r);
                }
            }
            for a in arms {
                if let Some(g) = a.guard {
                    scan_expr(it, module, g, out);
                }
                scan_expr(it, module, a.body, out);
            }
        }
        Statement::For { iterable, body, .. } => {
            scan_expr(it, module, iterable, out);
            for x in body {
                scan_stmt(it, module, x, out);
            }
        }
        Statement::Loop { body, .. } | Statement::Thread { body, .. } | Statement::Async { body, .. } => {
            for x in body {
                scan_stmt(it, module, x, out);
            }
        }
        Statement::While { condition, body, .. } => {
            scan_expr(it, module, condition, out);
            for x in body {
                scan_stmt(it, module, x, out);
            }
        }
        Statement::Defer { statement, .. } => scan_stmt(it, module, statement, out),
        Statement::Break { value: Some(v), .. } => scan_expr(it, module, v, out),
        Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => {}
    }
}

fn scan_expr(it: &mut Interpreter, module: usize, e: &'static Expression<'static>, out: &mut HashSet<&'static str>) {
    match e {
        Expression::Literal { .. } | Expression::Identifier { .. } => {}
        Expression::Binary { left, right, .. } => {
            scan_expr(it, module, left, out);
            scan_expr(it, module, right, out);
        }
        Expression::Unary { op, operand, .. } => {
            if *op == UnaryOp::MutRef {
                if let Some(r) = root(operand) {
                    out.insert(r);
                }
            }
            scan_expr(it, module, operand, out);
        }
        Expression::Call { function, arguments, .. } => {
            scan_expr(it, module, function, out);
            for (_, a) in arguments {
                scan_expr(it, module, a, out);
            }
            if let Expression::Identifier { name, .. } = function {
                let segs: Vec<&str> = name.split("::").collect();
                if let Ok(Some(Resolved::Func(g))) = it.resolve_in(module, &segs) {
                    let gm = it.param_mutation(&g);
                    let has_self = g.params.first().map(|p| p.name == "self").unwrap_or(false);
                    let offset = if has_self { 1 } else { 0 };
                    for (i, (_, a)) in arguments.iter().enumerate() {
                        let mutated = if i < offset { it.self_mutation(&g) } else { gm.get(i - offset).copied().unwrap_or(false) };
                        if mutated {
                            if let Some(r) = root(a) {
                                out.insert(r);
                            }
                        }
                    }
                }
            }
        }
        Expression::MethodCall { object, method, arguments, .. } => {
            scan_expr(it, module, object, out);
            for (_, a) in arguments {
                scan_expr(it, module, a, out);
            }
            if let Some(r) = root(object) {
                let mut mutating = it.builtins.is_mutating_name(method);
                if !mutating {
                    // user methods with this name that mutate their receiver
                    let cands: Vec<Rc<FuncDef>> = it
                        .methods
                        .iter()
                        .filter(|((_, m), _)| m == method)
                        .map(|(_, f)| f.clone())
                        .collect();
                    for g in cands {
                        if it.self_mutation(&g) {
                            mutating = true;
                            break;
                        }
                    }
                }
                if mutating {
                    out.insert(r);
                }
            }
        }
        Expression::FieldAccess { object, .. } => scan_expr(it, module, object, out),
        Expression::StructLiteral { fields, .. } => {
            for (_, v) in fields {
                scan_expr(it, module, v, out);
            }
        }
        Expression::MapLiteral { pairs, .. } => {
            for (k, v) in pairs {
                scan_expr(it, module, k, out);
                scan_expr(it, module, v, out);
            }
        }
        Expression::Range { start, end, .. } => {
            scan_expr(it, module, start, out);
            scan_expr(it, module, end, out);
        }
        Expression::Closure { body, .. } => scan_expr(it, module, body, out),
        Expression::Cast { expr, .. }
        | Expression::TryOp { expr, .. }
        | Expression::Await { expr, .. }
        | Expression::AsyncCall { expr, .. }
        | Expression::SpawnCall { expr, .. } => scan_expr(it, module, expr, out),
        Expression::Index { object, index, .. } => {
            scan_expr(it, module, object, out);
            scan_expr(it, module, index, out);
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            for x in elements {
                scan_expr(it, module, x, out);
            }
        }
        Expression::MacroInvocation { args, .. } => {
            for a in args {
                scan_expr(it, module, a, out);
            }
        }
        Expression::ChannelSend { channel, value, .. } => {
            scan_expr(it, module, channel, out);
            scan_expr(it, module, value, out);
        }
        Expression::ChannelRecv { channel, .. } => scan_expr(it, module, channel, out),
        Expression::Block { statements, .. } => {
            for s in statements {
                scan_stmt(it, module, s, out);
            }
        }
    }
}

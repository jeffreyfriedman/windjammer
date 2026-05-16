//! Cache locality analysis: AoSoA / SoA candidates for ECS-style `Vec<Struct>` loops.
//!
//! See `windjammer-game/CACHE_LOCALITY_DESIGN.md` for architecture and PGO integration.

use super::AnalyzedFunction;
use crate::parser::*;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Kind of memory access inferred from loop shape (best-effort).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessPatternKind {
    /// `for elem in vec_identifier` and the loop body does not subscript the same `Vec`.
    SequentialIteration,
    /// Body indexes the iterable (e.g. `items[i]`) alongside `for x in items` — mixed/random potential.
    IterableAlsoIndexedInBody,
}

/// Stable JSON snapshot of analyzer output for merging with LLVM PGO / custom tooling.
pub fn cache_locality_json_report(analyzed: &[AnalyzedFunction<'_>]) -> String {
    let mut candidates: Vec<Value> = Vec::new();
    for af in analyzed {
        for c in &af.cache_locality.aosoa_candidates {
            let counts: Vec<Value> = c
                .field_access_counts
                .iter()
                .map(|(n, k)| json!({ "field": n, "accesses": k }))
                .collect();
            candidates.push(json!({
                "function": c.function_name,
                "loop_var": c.loop_var,
                "iterable_var": c.iterable_var,
                "element_struct": c.element_struct,
                "field_access_counts": counts,
                "hot_fields": c.hot_fields,
                "cold_fields": c.cold_fields,
                "access_pattern": format!("{:?}", c.pattern_kind),
                "simd_friendly_layout": c.simd_friendly_layout,
            }));
        }
    }
    serde_json::to_string_pretty(&json!({ "aosoa_candidates": candidates })).unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq)]
pub struct AoSoACandidate {
    /// Windjammer function containing the loop.
    pub function_name: String,
    /// Loop binding (`for binding in iterable`).
    pub loop_var: String,
    /// Iterable local/param (`entities` in `for e in entities`).
    pub iterable_var: String,
    /// Inner struct name (`Entity` for `Vec<Entity>`).
    pub element_struct: String,
    /// Per-field access counts in the loop body (reads + writes via field syntax).
    pub field_access_counts: Vec<(String, u64)>,
    pub hot_fields: Vec<String>,
    pub cold_fields: Vec<String>,
    pub pattern_kind: AccessPatternKind,
    /// True if all struct fields are numeric scalars suitable for SIMD packs (heuristic).
    pub simd_friendly_layout: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CacheLocalityAnalysis {
    pub aosoa_candidates: Vec<AoSoACandidate>,
}

impl<'ast> super::Analyzer<'ast> {
    pub(crate) fn analyze_cache_locality(
        &self,
        program: &Program<'ast>,
        func: &FunctionDecl<'ast>,
    ) -> CacheLocalityAnalysis {
        let structs = collect_struct_layouts(&program.items);
        let mut locals = HashMap::new();
        for p in &func.parameters {
            locals.insert(p.name.clone(), p.type_.clone());
        }
        collect_annotated_locals(&func.body, &mut locals);

        let mut candidates = Vec::new();
        scan_for_aosoa_loops(
            self,
            func,
            &structs,
            &locals,
            &func.body,
            &mut candidates,
        );

        CacheLocalityAnalysis {
            aosoa_candidates: candidates,
        }
    }
}

fn collect_struct_layouts(items: &[Item<'_>]) -> HashMap<String, Vec<(String, Type)>> {
    let mut out = HashMap::new();
    collect_struct_layouts_rec(items, &mut out);
    out
}

fn collect_struct_layouts_rec(items: &[Item<'_>], out: &mut HashMap<String, Vec<(String, Type)>>) {
    for item in items {
        match item {
            Item::Struct { decl, .. } if !decl.is_extern && decl.tuple_fields.is_none() => {
                let fields: Vec<(String, Type)> = decl
                    .fields
                    .iter()
                    .map(|f| (f.name.clone(), f.field_type.clone()))
                    .collect();
                out.insert(decl.name.clone(), fields);
            }
            Item::Mod { items: inner, .. } => collect_struct_layouts_rec(inner, out),
            _ => {}
        }
    }
}

fn pattern_binding_name(pat: &Pattern<'_>) -> Option<String> {
    match pat {
        Pattern::Identifier(s) => Some(s.clone()),
        Pattern::MutBinding(s) => Some(s.clone()),
        Pattern::Ref(s) | Pattern::RefMut(s) => Some(s.clone()),
        _ => None,
    }
}

fn collect_annotated_locals(stmts: &[&Statement<'_>], out: &mut HashMap<String, Type>) {
    for st in stmts {
        match st {
            Statement::Let {
                pattern,
                type_: Some(ty),
                ..
            } => {
                if let Some(name) = pattern_binding_name(pattern) {
                    out.insert(name, ty.clone());
                }
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                collect_annotated_locals(then_block, out);
                if let Some(eb) = else_block {
                    collect_annotated_locals(eb, out);
                }
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => {
                collect_annotated_locals(body, out);
            }
            Statement::For { body, .. } => {
                collect_annotated_locals(body, out);
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    collect_annotated_locals_in_expr(&arm.body, out);
                }
            }
            Statement::Thread { body, .. } | Statement::Async { body, .. } => {
                collect_annotated_locals(body, out);
            }
            _ => {}
        }
    }
}

fn collect_annotated_locals_in_expr(expr: &Expression<'_>, out: &mut HashMap<String, Type>) {
    match expr {
        Expression::Block { statements, .. } => collect_annotated_locals(statements, out),
        _ => {}
    }
}

fn vec_element_struct_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Vec(inner) => match inner.as_ref() {
            Type::Custom(name) => Some(name.clone()),
            _ => None,
        },
        Type::Parameterized(base, args) if base == "Vec" && args.len() == 1 => match &args[0] {
            Type::Custom(name) => Some(name.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn scan_for_aosoa_loops_in_expr<'ast>(
    analyzer: &super::Analyzer<'ast>,
    func: &FunctionDecl<'ast>,
    structs: &HashMap<String, Vec<(String, Type)>>,
    locals: &HashMap<String, Type>,
    expr: &'ast Expression<'ast>,
    out: &mut Vec<AoSoACandidate>,
) {
    match expr {
        Expression::Block { statements, .. } => {
            scan_for_aosoa_loops(analyzer, func, structs, locals, statements, out);
        }
        _ => {}
    }
}

fn scan_for_aosoa_loops<'ast>(
    analyzer: &super::Analyzer<'ast>,
    func: &FunctionDecl<'ast>,
    structs: &HashMap<String, Vec<(String, Type)>>,
    locals: &HashMap<String, Type>,
    stmts: &[&'ast Statement<'ast>],
    out: &mut Vec<AoSoACandidate>,
) {
    for st in stmts {
        match st {
            Statement::For {
                pattern,
                iterable,
                body,
                ..
            } => {
                let Some(loop_var) = pattern_binding_name(pattern) else {
                    scan_for_aosoa_loops(analyzer, func, structs, locals, body, out);
                    continue;
                };
                if let Expression::Identifier {
                    name: iterable_var,
                    ..
                } = iterable
                {
                    if let Some(ty) = locals.get(iterable_var.as_str()) {
                        if let Some(elem) = vec_element_struct_name(ty) {
                            if let Some(fields) = structs.get(&elem) {
                                if !fields.is_empty()
                                    && fields
                                        .iter()
                                        .all(|(_, ft)| analyzer.is_copy_type(ft))
                                {
                                    let counts =
                                        count_field_accesses_on_var(&loop_var, body);
                                    let total: u64 = counts.values().sum();
                                    if total > 0 {
                                        let mut pairs: Vec<(String, u64)> =
                                            counts.into_iter().collect();
                                        pairs.sort_by(|a, b| {
                                            b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))
                                        });
                                        let (hot, cold) = split_hot_cold(&pairs, fields, total);
                                        let simd_friendly = fields.iter().all(|(_, ft)| {
                                            matches!(
                                                ft,
                                                Type::Float
                                                    | Type::Int
                                                    | Type::Int32
                                                    | Type::Uint
                                                    | Type::Bool
                                            )
                                        });
                                        let pattern_kind = if body_indexes_ident(
                                            iterable_var.as_str(),
                                            body,
                                        ) {
                                            AccessPatternKind::IterableAlsoIndexedInBody
                                        } else {
                                            AccessPatternKind::SequentialIteration
                                        };
                                        out.push(AoSoACandidate {
                                            function_name: func.name.clone(),
                                            loop_var,
                                            iterable_var: iterable_var.clone(),
                                            element_struct: elem,
                                            field_access_counts: pairs,
                                            hot_fields: hot,
                                            cold_fields: cold,
                                            pattern_kind,
                                            simd_friendly_layout: simd_friendly,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                scan_for_aosoa_loops(analyzer, func, structs, locals, body, out);
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                scan_for_aosoa_loops(analyzer, func, structs, locals, then_block, out);
                if let Some(eb) = else_block {
                    scan_for_aosoa_loops(analyzer, func, structs, locals, eb, out);
                }
            }
            Statement::While { body, .. } | Statement::Loop { body, .. } => {
                scan_for_aosoa_loops(analyzer, func, structs, locals, body, out);
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    scan_for_aosoa_loops_in_expr(analyzer, func, structs, locals, arm.body, out);
                }
            }
            Statement::Thread { body, .. } | Statement::Async { body, .. } => {
                scan_for_aosoa_loops(analyzer, func, structs, locals, body, out);
            }
            _ => {}
        }
    }
}

/// True if any `iterable[var]` appears in `body` (suggests non-stride-1 access of the same buffer).
fn body_indexes_ident(iterable: &str, body: &[&Statement<'_>]) -> bool {
    for st in body {
        if stmt_indexes_ident(iterable, st) {
            return true;
        }
    }
    false
}

fn stmt_indexes_ident(iterable: &str, st: &Statement<'_>) -> bool {
    match st {
        Statement::Let { value, .. } => expr_indexes_ident(iterable, value),
        Statement::Assignment { target, value, .. } => {
            expr_indexes_ident(iterable, target) || expr_indexes_ident(iterable, value)
        }
        Statement::Return {
            value: Some(expr), ..
        } => expr_indexes_ident(iterable, expr),
        Statement::Expression { expr, .. } => expr_indexes_ident(iterable, expr),
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            if expr_indexes_ident(iterable, condition) {
                return true;
            }
            for s in then_block {
                if stmt_indexes_ident(iterable, s) {
                    return true;
                }
            }
            if let Some(eb) = else_block {
                for s in eb {
                    if stmt_indexes_ident(iterable, s) {
                        return true;
                    }
                }
            }
            false
        }
        Statement::While {
            condition, body, ..
        } => {
            if expr_indexes_ident(iterable, condition) {
                return true;
            }
            for s in body {
                if stmt_indexes_ident(iterable, s) {
                    return true;
                }
            }
            false
        }
        Statement::For {
            iterable: it_expr, body, ..
        } => {
            if expr_indexes_ident(iterable, it_expr) {
                return true;
            }
            for s in body {
                if stmt_indexes_ident(iterable, s) {
                    return true;
                }
            }
            false
        }
        Statement::Loop { body, .. } => {
            for s in body {
                if stmt_indexes_ident(iterable, s) {
                    return true;
                }
            }
            false
        }
        Statement::Match { value, arms, .. } => {
            if expr_indexes_ident(iterable, value) {
                return true;
            }
            for arm in arms {
                if let Some(g) = arm.guard {
                    if expr_indexes_ident(iterable, g) {
                        return true;
                    }
                }
                match arm.body {
                    Expression::Block { statements, .. } => {
                        for s in statements {
                            if stmt_indexes_ident(iterable, s) {
                                return true;
                            }
                        }
                    }
                    ref e => {
                        if expr_indexes_ident(iterable, e) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        Statement::Thread { body, .. } | Statement::Async { body, .. } => {
            for s in body {
                if stmt_indexes_ident(iterable, s) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn expr_indexes_ident(iterable: &str, expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Index { object, index, .. } => {
            if let Expression::Identifier { name, .. } = &**object {
                if name.as_str() == iterable {
                    return true;
                }
            }
            expr_indexes_ident(iterable, object) || expr_indexes_ident(iterable, index)
        }
        Expression::FieldAccess { object, .. } => expr_indexes_ident(iterable, object),
        Expression::Binary { left, right, .. } => {
            expr_indexes_ident(iterable, left) || expr_indexes_ident(iterable, right)
        }
        Expression::Unary { operand, .. } => expr_indexes_ident(iterable, operand),
        Expression::Call { function, arguments, .. } => {
            expr_indexes_ident(iterable, function)
                || arguments
                    .iter()
                    .any(|(_, a)| expr_indexes_ident(iterable, a))
        }
        Expression::MethodCall {
            object,
            arguments,
            ..
        } => {
            expr_indexes_ident(iterable, object)
                || arguments
                    .iter()
                    .any(|(_, a)| expr_indexes_ident(iterable, a))
        }
        Expression::StructLiteral { fields, .. } => {
            fields.iter().any(|(_, v)| expr_indexes_ident(iterable, v))
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            elements.iter().any(|e| expr_indexes_ident(iterable, e))
        }
        Expression::Cast { expr, .. } => expr_indexes_ident(iterable, expr),
        Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
            expr_indexes_ident(iterable, expr)
        }
        Expression::MacroInvocation { args, .. } => {
            args.iter().any(|a| expr_indexes_ident(iterable, a))
        }
        Expression::Range { start, end, .. } => {
            expr_indexes_ident(iterable, start) || expr_indexes_ident(iterable, end)
        }
        Expression::Closure { body, .. } => expr_indexes_ident(iterable, body),
        Expression::Block { statements, .. } => {
            statements.iter().any(|s| stmt_indexes_ident(iterable, s))
        }
        Expression::MapLiteral { pairs, .. } => {
            pairs.iter().any(|(k, v)| {
                expr_indexes_ident(iterable, k) || expr_indexes_ident(iterable, v)
            })
        }
        Expression::ChannelSend { channel, value, .. } => {
            expr_indexes_ident(iterable, channel) || expr_indexes_ident(iterable, value)
        }
        Expression::ChannelRecv { channel, .. } => expr_indexes_ident(iterable, channel),
        _ => false,
    }
}

/// Include fields until cumulative access ≥ 70% of total (stable tie-break by field order).
fn split_hot_cold(
    sorted_counts: &[(String, u64)],
    all_fields: &[(String, Type)],
    total_access: u64,
) -> (Vec<String>, Vec<String>) {
    use std::collections::HashSet;
    let thresh = (total_access * 7).div_ceil(10).max(1);
    let mut acc = 0u64;
    let mut hot = Vec::new();
    for (name, c) in sorted_counts {
        if acc < thresh || hot.is_empty() {
            hot.push(name.clone());
            acc += c;
        }
    }
    let hot_set: HashSet<_> = hot.iter().cloned().collect();
    let cold: Vec<String> = all_fields
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| !hot_set.contains(n))
        .collect();
    (hot, cold)
}

fn count_field_accesses_on_var(
    loop_var: &str,
    stmts: &[&Statement<'_>],
) -> HashMap<String, u64> {
    let mut m = HashMap::new();
    for st in stmts {
        count_field_accesses_stmt(loop_var, st, &mut m);
    }
    m
}

fn count_field_accesses_match_body(
    loop_var: &str,
    body: &Expression<'_>,
    m: &mut HashMap<String, u64>,
) {
    match body {
        Expression::Block { statements, .. } => {
            for s in statements {
                count_field_accesses_stmt(loop_var, s, m);
            }
        }
        _ => count_field_accesses_expr(loop_var, body, m),
    }
}

fn count_field_accesses_stmt(loop_var: &str, st: &Statement<'_>, m: &mut HashMap<String, u64>) {
    match st {
        Statement::Let { value, .. } => {
            count_field_accesses_expr(loop_var, value, m);
        }
        Statement::Assignment { target, value, .. } => {
            count_field_accesses_expr(loop_var, target, m);
            count_field_accesses_expr(loop_var, value, m);
        }
        Statement::Return {
            value: Some(expr), ..
        } => {
            count_field_accesses_expr(loop_var, expr, m);
        }
        Statement::Expression { expr, .. } => {
            count_field_accesses_expr(loop_var, expr, m);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            count_field_accesses_expr(loop_var, condition, m);
            for s in then_block {
                count_field_accesses_stmt(loop_var, s, m);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    count_field_accesses_stmt(loop_var, s, m);
                }
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            count_field_accesses_expr(loop_var, condition, m);
            for s in body {
                count_field_accesses_stmt(loop_var, s, m);
            }
        }
        Statement::For {
            iterable, body, ..
        } => {
            count_field_accesses_expr(loop_var, iterable, m);
            for s in body {
                count_field_accesses_stmt(loop_var, s, m);
            }
        }
        Statement::Loop { body, .. } => {
            for s in body {
                count_field_accesses_stmt(loop_var, s, m);
            }
        }
        Statement::Match { value, arms, .. } => {
            count_field_accesses_expr(loop_var, value, m);
            for arm in arms {
                if let Some(g) = arm.guard {
                    count_field_accesses_expr(loop_var, g, m);
                }
                count_field_accesses_match_body(loop_var, arm.body, m);
            }
        }
        Statement::Thread { body, .. } | Statement::Async { body, .. } => {
            for s in body {
                count_field_accesses_stmt(loop_var, s, m);
            }
        }
        _ => {}
    }
}

fn count_field_accesses_expr(loop_var: &str, expr: &Expression<'_>, m: &mut HashMap<String, u64>) {
    match expr {
        Expression::FieldAccess { object, field, .. } => {
            if let Expression::Identifier { name, .. } = object {
                if name == loop_var {
                    *m.entry(field.clone()).or_insert(0) += 1;
                }
            }
            count_field_accesses_expr(loop_var, object, m);
        }
        Expression::Binary { left, right, .. } => {
            count_field_accesses_expr(loop_var, left, m);
            count_field_accesses_expr(loop_var, right, m);
        }
        Expression::Unary { operand, .. } => {
            count_field_accesses_expr(loop_var, operand, m);
        }
        Expression::Call { function, arguments, .. } => {
            count_field_accesses_expr(loop_var, function, m);
            for (_, arg) in arguments {
                count_field_accesses_expr(loop_var, arg, m);
            }
        }
        Expression::MethodCall {
            object,
            arguments,
            ..
        } => {
            count_field_accesses_expr(loop_var, object, m);
            for (_, arg) in arguments {
                count_field_accesses_expr(loop_var, arg, m);
            }
        }
        Expression::Index { object, index, .. } => {
            count_field_accesses_expr(loop_var, object, m);
            count_field_accesses_expr(loop_var, index, m);
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, fv) in fields {
                count_field_accesses_expr(loop_var, fv, m);
            }
        }
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            for e in elements {
                count_field_accesses_expr(loop_var, e, m);
            }
        }
        Expression::Cast { expr, .. } => count_field_accesses_expr(loop_var, expr, m),
        Expression::TryOp { expr, .. } | Expression::Await { expr, .. } => {
            count_field_accesses_expr(loop_var, expr, m);
        }
        Expression::MacroInvocation { args, .. } => {
            for a in args {
                count_field_accesses_expr(loop_var, a, m);
            }
        }
        Expression::Range {
            start,
            end,
            ..
        } => {
            count_field_accesses_expr(loop_var, start, m);
            count_field_accesses_expr(loop_var, end, m);
        }
        Expression::Closure { body, .. } => count_field_accesses_expr(loop_var, body, m),
        Expression::Block {
            statements, ..
        } => {
            for s in statements {
                count_field_accesses_stmt(loop_var, s, m);
            }
        }
        Expression::MapLiteral { pairs, .. } => {
            for (k, v) in pairs {
                count_field_accesses_expr(loop_var, k, m);
                count_field_accesses_expr(loop_var, v, m);
            }
        }
        Expression::ChannelSend { channel, value, .. } => {
            count_field_accesses_expr(loop_var, channel, m);
            count_field_accesses_expr(loop_var, value, m);
        }
        Expression::ChannelRecv { channel, .. } => {
            count_field_accesses_expr(loop_var, channel, m);
        }
        _ => {}
    }
}

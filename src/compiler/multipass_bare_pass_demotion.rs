//! Multipass library builds: sibling modules that pass bare bindings (`f(path)` not
//! `f(path.clone())`) signal that callee formals should demote to shared/mut borrow
//! (WDB-112 / cross-crate Vec / WDB-113).

use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use crate::parser::{Expression, Item, Program, Statement, Type};

/// Scan all library programs for bare-identifier call sites and promote callee
/// `param_ownership` in the merged global registry before per-file codegen.
pub fn promote_callees_from_bare_pass_callers(
    registry: &mut SignatureRegistry,
    programs: &[&Program],
    copy_types: &std::collections::HashSet<String>,
) {
    let mut hints: Vec<(String, usize, OwnershipMode)> = Vec::new();
    for program in programs {
        collect_bare_pass_hints(program, programs, registry, copy_types, &mut hints);
    }
    hints.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    hints.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
    for (callee_key, param_idx, mode) in hints {
        if bare_pass_hint_should_skip(programs, registry, &callee_key, param_idx, mode) {
            continue;
        }
        apply_bare_pass_hint(registry, &callee_key, param_idx, mode);
    }
}

fn collect_bare_pass_hints(
    program: &Program,
    programs: &[&Program],
    registry: &SignatureRegistry,
    copy_types: &std::collections::HashSet<String>,
    hints: &mut Vec<(String, usize, OwnershipMode)>,
) {
    for item in &program.items {
        let Item::Function { decl, .. } = item else {
            continue;
        };
        walk_statements_for_calls(&decl.body, programs, registry, copy_types, hints);
    }
    for item in &program.items {
        let Item::Impl { block, .. } = item else {
            continue;
        };
        for method in &block.functions {
            walk_statements_for_calls(&method.body, programs, registry, copy_types, hints);
        }
    }
}

fn walk_statements_for_calls<'ast>(
    stmts: &[&'ast Statement<'ast>],
    programs: &[&Program],
    registry: &SignatureRegistry,
    copy_types: &std::collections::HashSet<String>,
    hints: &mut Vec<(String, usize, OwnershipMode)>,
) {
    for stmt in stmts {
        match stmt {
            Statement::Expression { expr, .. }
            | Statement::Return {
                value: Some(expr), ..
            } => visit_expr_for_calls(expr, programs, registry, copy_types, hints),
            Statement::Let { value, else_block, .. } => {
                visit_expr_for_calls(value, programs, registry, copy_types, hints);
                if let Some(b) = else_block {
                    walk_statements_for_calls(b, programs, registry, copy_types, hints);
                }
            }
            Statement::If {
                then_block,
                else_block,
                condition,
                ..
            } => {
                visit_expr_for_calls(condition, programs, registry, copy_types, hints);
                walk_statements_for_calls(then_block, programs, registry, copy_types, hints);
                if let Some(b) = else_block {
                    walk_statements_for_calls(b, programs, registry, copy_types, hints);
                }
            }
            Statement::While { body, condition, .. } => {
                visit_expr_for_calls(condition, programs, registry, copy_types, hints);
                walk_statements_for_calls(body, programs, registry, copy_types, hints);
            }
            Statement::For { body, iterable, .. } => {
                visit_expr_for_calls(iterable, programs, registry, copy_types, hints);
                walk_statements_for_calls(body, programs, registry, copy_types, hints);
            }
            Statement::Match { value, arms, .. } => {
                visit_expr_for_calls(value, programs, registry, copy_types, hints);
                for arm in arms {
                    visit_expr_for_calls(&arm.body, programs, registry, copy_types, hints);
                }
            }
            _ => {}
        }
    }
}

fn visit_expr_for_calls(
    expr: &Expression,
    programs: &[&Program],
    registry: &SignatureRegistry,
    copy_types: &std::collections::HashSet<String>,
    hints: &mut Vec<(String, usize, OwnershipMode)>,
) {
    match expr {
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            record_bare_pass_call_hints(function, arguments, registry, copy_types, hints);
            visit_expr_for_calls(function, programs, registry, copy_types, hints);
            for (_, arg) in arguments {
                visit_expr_for_calls(arg, programs, registry, copy_types, hints);
            }
        }
        Expression::MethodCall {
            object,
            arguments,
            ..
        } => {
            visit_expr_for_calls(object, programs, registry, copy_types, hints);
            for (_, arg) in arguments {
                visit_expr_for_calls(arg, programs, registry, copy_types, hints);
            }
        }
        Expression::Block { statements, .. } => {
            walk_statements_for_calls(statements, programs, registry, copy_types, hints);
        }
        Expression::Binary { left, right, .. } => {
            visit_expr_for_calls(left, programs, registry, copy_types, hints);
            visit_expr_for_calls(right, programs, registry, copy_types, hints);
        }
        Expression::Unary { operand, .. }
        | Expression::FieldAccess { object: operand, .. }
        | Expression::Index { object: operand, .. }
        | Expression::TryOp { expr: operand, .. }
        | Expression::Await { expr: operand, .. }
        | Expression::Cast { expr: operand, .. } => {
            visit_expr_for_calls(operand, programs, registry, copy_types, hints);
        }
        Expression::Array { elements, .. } | Expression::Tuple { elements, .. } => {
            for elem in elements {
                visit_expr_for_calls(elem, programs, registry, copy_types, hints);
            }
        }
        _ => {}
    }
}

fn record_bare_pass_call_hints<'ast>(
    function: &'ast Expression<'ast>,
    arguments: &[(Option<String>, &'ast Expression<'ast>)],
    registry: &SignatureRegistry,
    copy_types: &std::collections::HashSet<String>,
    hints: &mut Vec<(String, usize, OwnershipMode)>,
) {
    let Some(callee_name) = callee_name_from_expr(function) else {
        return;
    };
    for key in callee_registry_keys(&callee_name, registry) {
        let Some(sig) = registry.get_signature(&key) else {
            continue;
        };
        for (i, (_, arg)) in arguments.iter().enumerate() {
            if !is_bare_binding_pass(arg) {
                continue;
            }
            let pidx = sig.arg_param_index(i);
            let formal_ty = sig
                .formal_param_types
                .get(pidx)
                .or_else(|| sig.param_types.get(pidx));
            let Some(formal_ty) = formal_ty else {
                continue;
            };
            if matches!(
                sig.param_ownership.get(pidx),
                Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
            ) {
                continue;
            }
            let Some(mode) = bare_pass_target_ownership(formal_ty, copy_types) else {
                continue;
            };
            hints.push((key.clone(), pidx, mode));
        }
    }
}

fn callee_name_from_expr(function: &Expression) -> Option<String> {
    match function {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, field, .. } => {
            let base = callee_name_from_expr(object)?;
            Some(format!("{base}::{field}"))
        }
        _ => None,
    }
}

fn callee_registry_keys(callee_name: &str, registry: &SignatureRegistry) -> Vec<String> {
    let mut keys: Vec<String> = registry
        .signatures
        .keys()
        .filter(|k| {
            **k == callee_name
                || k.ends_with(&format!("::{callee_name}"))
                || k.rsplit("::").next() == Some(callee_name)
        })
        .cloned()
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

fn is_bare_binding_pass(expr: &Expression) -> bool {
    matches!(expr, Expression::Identifier { .. })
}

fn bare_pass_target_ownership(
    formal_ty: &Type,
    copy_types: &std::collections::HashSet<String>,
) -> Option<OwnershipMode> {
    if crate::codegen::rust::types::is_windjammer_text_type(formal_ty) {
        return Some(OwnershipMode::Borrowed);
    }
    if is_vec_container_type(formal_ty) {
        return Some(OwnershipMode::Borrowed);
    }
    if let Type::Custom(name) = formal_ty {
        if copy_types.contains(name) {
            return None;
        }
        return Some(OwnershipMode::MutBorrowed);
    }
    None
}

fn is_vec_container_type(ty: &Type) -> bool {
    matches!(ty, Type::Vec(_))
        || matches!(ty, Type::Parameterized(name, _) if name == "Vec")
        || matches!(ty, Type::Custom(name) if name.starts_with("Vec"))
}

fn apply_bare_pass_hint(
    registry: &mut SignatureRegistry,
    key: &str,
    param_idx: usize,
    mode: OwnershipMode,
) {
    let Some(mut sig) = registry.get_signature(key).cloned() else {
        return;
    };
    if sig.param_ownership.is_empty() {
        let n = sig
            .formal_param_types
            .len()
            .max(sig.param_types.len());
        sig.param_ownership = vec![OwnershipMode::Owned; n];
    }
    if matches!(
        sig.param_ownership.get(param_idx),
        Some(OwnershipMode::Borrowed | OwnershipMode::MutBorrowed)
    ) {
        return;
    }
    if sig.param_ownership.len() <= param_idx {
        return;
    }
    sig.param_ownership[param_idx] = mode;
    let n = sig.param_ownership.len();
    let mut ref_flags = sig
        .emitted_rust_ref_params
        .clone()
        .unwrap_or_else(|| vec![false; n]);
    if ref_flags.len() < n {
        ref_flags.resize(n, false);
    }
    ref_flags[param_idx] = matches!(mode, OwnershipMode::Borrowed);
    sig.emitted_rust_ref_params = Some(ref_flags);
    wrap_param_type_for_borrow(&mut sig, param_idx, mode);
    registry.signatures.insert(key.to_string(), sig.clone());
    if let Some(bare) = key.rsplit("::").next() {
        if bare != key && registry.signatures.contains_key(bare) {
            registry.signatures.insert(bare.to_string(), sig);
        }
    }
}

fn wrap_param_type_for_borrow(sig: &mut FunctionSignature, param_idx: usize, mode: OwnershipMode) {
    let bare = sig
        .formal_param_types
        .get(param_idx)
        .cloned()
        .or_else(|| sig.param_types.get(param_idx).cloned());
    let Some(bare) = bare else {
        return;
    };
    let wrapped = match mode {
        OwnershipMode::Borrowed if crate::codegen::rust::types::is_windjammer_text_type(&bare) => {
            Type::Reference(Box::new(Type::String))
        }
        OwnershipMode::Borrowed => Type::Reference(Box::new(bare.clone())),
        OwnershipMode::MutBorrowed => Type::MutableReference(Box::new(bare.clone())),
        OwnershipMode::Owned => return,
    };
    if sig.param_types.len() > param_idx {
        sig.param_types[param_idx] = wrapped;
    }
}

fn bare_pass_hint_should_skip(
    programs: &[&Program],
    registry: &SignatureRegistry,
    callee_key: &str,
    param_idx: usize,
    mode: OwnershipMode,
) -> bool {
    let Some(sig) = registry.get_signature(callee_key) else {
        return false;
    };
    if matches!(mode, OwnershipMode::Borrowed)
        && callee_owned_text_builder_stores_payload(&sig, param_idx)
    {
        return true;
    }
    let Some((param_name, body)) = find_function_body_for_registry_key(programs, callee_key, param_idx)
    else {
        return false;
    };
    match mode {
        OwnershipMode::MutBorrowed => param_forwards_fields_in_call_args_only(body, param_name),
        OwnershipMode::Borrowed => {
            param_stored_in_struct_literal(body, param_name)
                || param_forwards_fields_in_call_args_only(body, param_name)
        }
        OwnershipMode::Owned => false,
    }
}
/// Owned `string`/`Vec` formals on builders that return a Custom type store payload — keep
/// owned even when callers pass bare bindings (HTTP `ServerResponse::json(body: string)`).
fn callee_owned_text_builder_stores_payload(sig: &FunctionSignature, param_idx: usize) -> bool {
    let owned_text = sig
        .formal_param_types
        .get(param_idx)
        .or_else(|| sig.param_types.get(param_idx))
        .is_some_and(|t| {
            crate::codegen::rust::types::is_windjammer_text_type(t)
                && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
        });
    owned_text
        && sig.return_type.as_ref().is_some_and(|t| {
            matches!(t, Type::Custom(_))
                || matches!(t, Type::Parameterized(name, _) if name != "Result" && name != "Option")
        })
}

fn find_function_body_for_registry_key<'a>(
    programs: &'a [&Program],
    registry_key: &str,
    param_idx: usize,
) -> Option<(&'a str, &'a [&'a Statement<'a>])> {
    let simple = registry_key.rsplit("::").next().unwrap_or(registry_key);
    for program in programs {
        for item in &program.items {
            if let Item::Function { decl, .. } = item {
                if decl.name == simple || registry_key.ends_with(&format!("::{simple}")) {
                    let param = non_self_param(&decl.parameters, param_idx)?;
                    return Some((param.name.as_str(), decl.body.as_slice()));
                }
            }
            if let Item::Impl { block, .. } = item {
                for method in &block.functions {
                    if method.name == simple || registry_key.ends_with(&format!("::{simple}")) {
                        let param = non_self_param(&method.parameters, param_idx)?;
                        return Some((param.name.as_str(), method.body.as_slice()));
                    }
                }
            }
        }
    }
    None
}

fn non_self_param<'a>(
    parameters: &'a [crate::parser::Parameter<'a>],
    param_idx: usize,
) -> Option<&'a crate::parser::Parameter<'a>> {
    parameters
        .iter()
        .filter(|p| p.name != "self")
        .nth(param_idx)
}

/// Callee body only reads `param` via `param.field` / `param[i]` in call arguments (HTTP
/// adapter `to_response(reply) { base_response(reply.status, reply.body) }`).
pub fn param_forwards_fields_in_call_args_only(body: &[&Statement], param_name: &str) -> bool {
    let mut saw = false;
    for stmt in body {
        match stmt_param_usage(stmt, param_name) {
            ParamUsage::None => {}
            ParamUsage::FieldInCallArg => saw = true,
            ParamUsage::Other => return false,
        }
    }
    saw
}

fn param_stored_in_struct_literal(body: &[&Statement], param_name: &str) -> bool {
    body.iter()
        .any(|stmt| statement_stores_param_in_struct_literal(stmt, param_name))
}

enum ParamUsage {
    None,
    FieldInCallArg,
    Other,
}

fn stmt_param_usage(stmt: &Statement, param_name: &str) -> ParamUsage {
    match stmt {
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expr_param_usage(expr, param_name),
        Statement::Return { .. } => ParamUsage::None,
        Statement::Let { value, else_block, .. } => {
            let mut usage = expr_param_usage(value, param_name);
            if let Some(b) = else_block {
                for s in b {
                    usage = usage.merge(stmt_param_usage(s, param_name));
                }
            }
            usage
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            let mut usage = ParamUsage::None;
            for s in then_block {
                usage = usage.merge(stmt_param_usage(s, param_name));
            }
            if let Some(b) = else_block {
                for s in b {
                    usage = usage.merge(stmt_param_usage(s, param_name));
                }
            }
            usage
        }
        Statement::While { body, .. } | Statement::For { body, .. } => {
            let mut usage = ParamUsage::None;
            for s in body {
                usage = usage.merge(stmt_param_usage(s, param_name));
            }
            usage
        }
        Statement::Match { arms, .. } => {
            let mut usage = ParamUsage::None;
            for arm in arms {
                usage = usage.merge(expr_param_usage(&arm.body, param_name));
            }
            usage
        }
        _ => {
            if statement_mentions_param(stmt, param_name) {
                ParamUsage::Other
            } else {
                ParamUsage::None
            }
        }
    }
}

impl ParamUsage {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (ParamUsage::Other, _) | (_, ParamUsage::Other) => ParamUsage::Other,
            (ParamUsage::FieldInCallArg, ParamUsage::FieldInCallArg) => ParamUsage::FieldInCallArg,
            (ParamUsage::FieldInCallArg, ParamUsage::None) | (ParamUsage::None, ParamUsage::FieldInCallArg) => {
                ParamUsage::FieldInCallArg
            }
            (ParamUsage::None, ParamUsage::None) => ParamUsage::None,
        }
    }
}

fn expr_param_usage(expr: &Expression, param_name: &str) -> ParamUsage {
    match expr {
        Expression::Identifier { name, .. } if name == param_name => ParamUsage::Other,
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
            if matches!(
                object,
                Expression::Identifier { name, .. } if name == param_name
            ) {
                ParamUsage::FieldInCallArg
            } else {
                expr_param_usage(object, param_name)
            }
        }
        Expression::Call { arguments, .. } => {
            let mut usage = ParamUsage::None;
            for (_, arg) in arguments {
                usage = usage.merge(call_arg_param_usage(arg, param_name));
            }
            usage
        }
        Expression::MethodCall { object, arguments, .. } => {
            let mut usage = expr_param_usage(object, param_name);
            for (_, arg) in arguments {
                usage = usage.merge(call_arg_param_usage(arg, param_name));
            }
            usage
        }
        Expression::StructLiteral { fields, .. } => {
            if fields
                .iter()
                .any(|(_, v)| expr_mentions_param(v, param_name))
            {
                ParamUsage::Other
            } else {
                ParamUsage::None
            }
        }
        Expression::Binary { left, right, .. } => {
            expr_param_usage(left, param_name).merge(expr_param_usage(right, param_name))
        }
        Expression::Unary { operand, .. } => expr_param_usage(operand, param_name),
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
            let mut usage = ParamUsage::None;
            for elem in elements {
                usage = usage.merge(expr_param_usage(elem, param_name));
            }
            usage
        }
        Expression::Block { statements, .. } => {
            let mut usage = ParamUsage::None;
            for s in statements {
                usage = usage.merge(stmt_param_usage(s, param_name));
            }
            usage
        }
        _ => {
            if expr_mentions_param(expr, param_name) {
                ParamUsage::Other
            } else {
                ParamUsage::None
            }
        }
    }
}

fn call_arg_param_usage(expr: &Expression, param_name: &str) -> ParamUsage {
    match expr {
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. }
            if matches!(
                object,
                Expression::Identifier { name, .. } if name == param_name
            ) =>
        {
            ParamUsage::FieldInCallArg
        }
        Expression::Identifier { name, .. } if name == param_name => ParamUsage::FieldInCallArg,
        _ => expr_param_usage(expr, param_name),
    }
}

fn statement_stores_param_in_struct_literal(stmt: &Statement, param_name: &str) -> bool {
    match stmt {
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expr_stores_param_in_struct_literal(expr, param_name),
        Statement::Let { value, else_block, .. } => {
            expr_stores_param_in_struct_literal(value, param_name)
                || else_block.as_ref().is_some_and(|b| {
                    b.iter()
                        .any(|s| statement_stores_param_in_struct_literal(s, param_name))
                })
        }
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            then_block
                .iter()
                .any(|s| statement_stores_param_in_struct_literal(s, param_name))
                || else_block.as_ref().is_some_and(|b| {
                    b.iter()
                        .any(|s| statement_stores_param_in_struct_literal(s, param_name))
                })
        }
        _ => false,
    }
}

fn expr_stores_param_in_struct_literal(expr: &Expression, param_name: &str) -> bool {
    match expr {
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_mentions_param(v, param_name)),
        Expression::Call { arguments, .. } => arguments.iter().any(|(_, arg)| {
            expr_stores_param_in_struct_literal(arg, param_name)
                || matches!(arg, Expression::Identifier { name, .. } if name == param_name)
        }),
        _ => false,
    }
}

fn statement_mentions_param(stmt: &Statement, param_name: &str) -> bool {
    match stmt {
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expr_mentions_param(expr, param_name),
        Statement::Let { value, else_block, .. } => {
            expr_mentions_param(value, param_name)
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| statement_mentions_param(s, param_name)))
        }
        _ => false,
    }
}

fn expr_mentions_param(expr: &Expression, param_name: &str) -> bool {
    match expr {
        Expression::Identifier { name, .. } => name == param_name,
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => {
            expr_mentions_param(object, param_name)
        }
        Expression::Call { function, arguments, .. } => {
            expr_mentions_param(function, param_name)
                || arguments
                    .iter()
                    .any(|(_, arg)| expr_mentions_param(arg, param_name))
        }
        Expression::MethodCall { object, arguments, .. } => {
            expr_mentions_param(object, param_name)
                || arguments
                    .iter()
                    .any(|(_, arg)| expr_mentions_param(arg, param_name))
        }
        Expression::Binary { left, right, .. } => {
            expr_mentions_param(left, param_name) || expr_mentions_param(right, param_name)
        }
        Expression::Unary { operand, .. } => expr_mentions_param(operand, param_name),
        Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => elements
            .iter()
            .any(|e| expr_mentions_param(e, param_name)),
        Expression::Block { statements, .. } => statements
            .iter()
            .any(|s| statement_mentions_param(s, param_name)),
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_mentions_param(v, param_name)),
        _ => false,
    }
}

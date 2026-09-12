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
    // Undo MutBorrowed on Custom formals that only forward fields into callees
    // (`to_response(reply) { base_response(reply.status, reply.body) }`). Those must
    // stay owned moves — `&mut HttpReply` cannot move `reply.body`.
    restore_owned_field_forward_formals(registry, programs);
    // Undo bare-pass MutBorrowed when the callee returns the whole binding (identity /
    // chain helpers like `col_string(row, name) -> (Row, string)`).
    restore_owned_returned_formals(registry, programs);
}

/// When a formal was demoted to `MutBorrowed` but the body only forwards `param.field`
/// into call arguments, restore owned `T` (HTTP adapter / consume-via-fields pattern).
pub fn restore_owned_field_forward_formals(
    registry: &mut SignatureRegistry,
    programs: &[&Program],
) {
    let keys: Vec<String> = registry.signatures.keys().cloned().collect();
    for key in keys {
        let Some(sig) = registry.get_signature(&key).cloned() else {
            continue;
        };
        let n = sig.param_ownership.len();
        let mut changed = false;
        let mut new_sig = sig.clone();
        for idx in 0..n {
            if !matches!(
                new_sig.param_ownership.get(idx),
                Some(OwnershipMode::MutBorrowed)
            ) {
                continue;
            }
            let formal_ty = new_sig
                .formal_param_types
                .get(idx)
                .or_else(|| new_sig.param_types.get(idx));
            let Some(formal_ty) = formal_ty else {
                continue;
            };
            let bare = match formal_ty {
                Type::Custom(name) => name.clone(),
                Type::MutableReference(inner) => match inner.as_ref() {
                    Type::Custom(name) => name.clone(),
                    _ => continue,
                },
                _ => continue,
            };
            if is_copy_formal_name(&bare, &std::collections::HashSet::new()) {
                continue;
            }
            let Some((param_name, body)) =
                find_function_body_for_registry_key(programs, &key, idx)
            else {
                continue;
            };
            if !(param_forwards_fields_in_call_args_only(body, param_name)
                || param_stored_in_struct_literal(body, param_name)
                || param_whole_binding_returned(body, param_name)
                || param_used_only_as_match_scrutinee(body, param_name))
            {
                continue;
            }
            new_sig.param_ownership[idx] = OwnershipMode::Owned;
            let owned_ty = Type::Custom(bare);
            if new_sig.param_types.len() > idx {
                new_sig.param_types[idx] = owned_ty.clone();
            }
            if new_sig.formal_param_types.len() > idx {
                // Keep AST formal as the bare Custom when present.
                if matches!(
                    new_sig.formal_param_types[idx],
                    Type::MutableReference(_)
                ) {
                    new_sig.formal_param_types[idx] = owned_ty;
                }
            }
            if let Some(ref mut flags) = new_sig.emitted_rust_ref_params {
                if flags.len() > idx {
                    flags[idx] = false;
                }
            }
            changed = true;
        }
        if changed {
            registry.signatures.insert(key.clone(), new_sig.clone());
            if let Some(bare) = key.rsplit("::").next() {
                if bare != key.as_str() && registry.signatures.contains_key(bare) {
                    registry.signatures.insert(bare.to_string(), new_sig);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn parse_program(src: &'static str) -> &'static Program<'static> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        Box::leak(Box::new(parser.parse().expect("parse")))
    }

    fn owned_custom_sig(name: &str, ty: &str) -> FunctionSignature {
        FunctionSignature {
            name: name.to_string(),
            param_types: vec![Type::Custom(ty.into())],
            formal_param_types: vec![Type::Custom(ty.into())],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: Some(Type::Int),
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
            emitted_rust_ref_params: None,
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        }
    }

    #[test]
    fn bare_pass_skips_match_scrutinee_owned_forwarder() {
        // WDB-155 strengthened: emit_sql(ast) { match bind_ast(ast) { … } }
        let caller = parse_program(
            r#"
struct SqlAst { table: string }
fn parse_then_emit(ast: SqlAst) -> bool {
    let _ = emit_sql(ast)
    true
}
"#,
        );
        let callee = parse_program(
            r#"
struct SqlAst { table: string }
struct SqlResolved { table: string }
struct SqlEmit { table: string }
fn bind_ast(ast: SqlAst) -> Option<SqlResolved> {
    Some(SqlResolved { table: ast.table })
}
fn emit_sql(ast: SqlAst) -> Option<SqlEmit> {
    match bind_ast(ast) {
        Some(resolved) => Some(SqlEmit { table: resolved.table }),
        None => None,
    }
}
"#,
        );
        let mut registry = SignatureRegistry::new();
        registry.signatures.insert(
            "emit_sql".to_string(),
            owned_custom_sig("emit_sql", "SqlAst"),
        );
        registry.signatures.insert(
            "bind_ast".to_string(),
            owned_custom_sig("bind_ast", "SqlAst"),
        );
        let programs = vec![caller, callee];
        let copy_types = std::collections::HashSet::new();
        promote_callees_from_bare_pass_callers(&mut registry, &programs, &copy_types);
        let emit = registry.signatures.get("emit_sql").unwrap();
        assert_eq!(
            emit.param_ownership[0],
            OwnershipMode::Owned,
            "WDB-155: match-scrutinee forwarder must stay owned, got {:?}",
            emit.param_ownership
        );
    }

    #[test]
    fn bare_pass_skips_option_struct_literal_field_forward_custom() {
        // WDB-155 class: `emit_sql(ast) { Some(SqlEmit { table: ast.table }) }`
        let caller = parse_program(
            r#"
struct SqlAst { table: string }
struct SqlEmit { table: string }
fn parse_then_emit(sql: string) -> bool {
    let ast = match parse_ast(sql) {
        Some(a) => a,
        None => { return false }
    }
    let _ = emit_sql(ast)
    true
}
fn parse_ast(sql: string) -> Option<SqlAst> {
    Some(SqlAst { table: sql })
}
"#,
        );
        let callee = parse_program(
            r#"
struct SqlAst { table: string }
struct SqlEmit { table: string }
fn emit_sql(ast: SqlAst) -> Option<SqlEmit> {
    Some(SqlEmit { table: ast.table })
}
"#,
        );
        let mut registry = SignatureRegistry::new();
        registry.signatures.insert(
            "emit_sql".to_string(),
            owned_custom_sig("emit_sql", "SqlAst"),
        );
        let programs = vec![caller, callee];
        let copy_types = std::collections::HashSet::new();
        promote_callees_from_bare_pass_callers(&mut registry, &programs, &copy_types);
        let sig = registry.signatures.get("emit_sql").unwrap();
        assert_eq!(
            sig.param_ownership[0],
            OwnershipMode::Owned,
            "WDB-155: Option+struct-literal field forward must stay owned, got {:?}",
            sig.param_ownership
        );
    }

    #[test]
    fn bare_pass_does_not_demote_copy_char_formal() {
        let caller = parse_program(
            r#"
fn parse(text: string) -> Option<int> {
    for ch in strings.chars(text) {
        let _ = char_to_digit(ch)
    }
    None
}
"#,
        );
        let callee = parse_program(
            r#"
fn char_to_digit(ch: char) -> Option<int> {
    if ch == '0' { return Some(0) }
    None
}
"#,
        );
        let mut registry = SignatureRegistry::new();
        registry.signatures.insert(
            "char_to_digit".to_string(),
            FunctionSignature {
                name: "char_to_digit".into(),
                param_types: vec![Type::Custom("char".into())],
                formal_param_types: vec![Type::Custom("char".into())],
                param_ownership: vec![OwnershipMode::Owned],
                return_type: Some(Type::Option(Box::new(Type::Int))),
                return_ownership: OwnershipMode::Owned,
                has_self_receiver: false,
                is_extern: false,
                emitted_rust_ref_params: None,
                string_ref_string_formal_params: None,
                field_extract_params: None,
                forwarding_borrow_params: None,
            },
        );
        let programs = vec![caller, callee];
        let copy_types = std::collections::HashSet::new();
        promote_callees_from_bare_pass_callers(&mut registry, &programs, &copy_types);
        let sig = registry.signatures.get("char_to_digit").unwrap();
        assert_eq!(
            sig.param_ownership[0],
            OwnershipMode::Owned,
            "char formal must stay owned, got {:?}",
            sig.param_ownership
        );
    }

    #[test]
    fn bare_pass_skips_http_reply_field_forward_helper() {
        let program = parse_program(
            r#"
struct HttpReply { status: u16, body: string }
fn dispatch(reply: HttpReply) -> int {
    to_response(reply)
}
fn to_response(reply: HttpReply) -> int {
    let resp = base_response(reply.status, reply.body)
    resp
}
fn base_response(status: u16, body: string) -> int {
    status
}
"#,
        );
        let mut registry = SignatureRegistry::new();
        registry
            .signatures
            .insert("to_response".to_string(), owned_custom_sig("to_response", "HttpReply"));
        let programs = vec![program];
        let copy_types = std::collections::HashSet::new();
        promote_callees_from_bare_pass_callers(&mut registry, &programs, &copy_types);
        let sig = registry.signatures.get("to_response").unwrap();
        assert_eq!(
            sig.param_ownership[0],
            OwnershipMode::Owned,
            "field-forward HttpReply helper must stay owned, got {:?}",
            sig.param_ownership
        );
    }

    #[test]
    fn restore_undoes_preexisting_mut_borrow_on_field_forward() {
        let program = parse_program(
            r#"
struct HttpReply { status: u16, body: string }
fn to_response(reply: HttpReply) -> int {
    let resp = base_response(reply.status, reply.body)
    resp
}
fn base_response(status: u16, body: string) -> int {
    status
}
"#,
        );
        let mut registry = SignatureRegistry::new();
        let mut sig = owned_custom_sig("to_response", "HttpReply");
        sig.param_ownership[0] = OwnershipMode::MutBorrowed;
        sig.param_types[0] = Type::MutableReference(Box::new(Type::Custom("HttpReply".into())));
        registry.signatures.insert("to_response".to_string(), sig);
        let programs = vec![program];
        let copy_types = std::collections::HashSet::new();
        promote_callees_from_bare_pass_callers(&mut registry, &programs, &copy_types);
        let out = registry.signatures.get("to_response").unwrap();
        assert_eq!(
            out.param_ownership[0],
            OwnershipMode::Owned,
            "restore must undo MutBorrowed field-forward formal; got {:?}",
            out.param_ownership
        );
        assert!(
            matches!(out.param_types[0], Type::Custom(ref n) if n == "HttpReply"),
            "param type must unwrap to owned HttpReply; got {:?}",
            out.param_types[0]
        );
    }

    #[test]
    fn bare_pass_skips_row_col_chain_tuple_return_helper() {
        let domain = parse_program(
            r#"
struct Row { leftover: string }
pub fn col_string(row: Row, name: string) -> (Row, string) {
    (row, name)
}
"#,
        );
        let adapter = parse_program(
            r#"
use crate::row::{Row, col_string}
pub fn parse_payment(row: Row) -> string {
    let (row, id) = col_string(row, "id")
    let _ = row
    id
}
"#,
        );
        let mut registry = SignatureRegistry::new();
        registry
            .signatures
            .insert("col_string".to_string(), owned_custom_sig("col_string", "Row"));
        let programs = vec![adapter, domain];
        let copy_types = std::collections::HashSet::new();
        promote_callees_from_bare_pass_callers(&mut registry, &programs, &copy_types);
        let sig = registry.signatures.get("col_string").unwrap();
        assert_eq!(
            sig.param_ownership[0],
            OwnershipMode::Owned,
            "tuple-return Row chain helper must stay owned, got {:?}",
            sig.param_ownership
        );
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
        // Copy primitives (`char`, `i64`, …) and known Copy aggregates stay owned —
        // bare-pass demotion must not invent `&mut char` for comparison-only helpers.
        if is_copy_formal_name(name, copy_types) {
            return None;
        }
        return Some(OwnershipMode::MutBorrowed);
    }
    None
}

fn is_copy_formal_name(name: &str, copy_types: &std::collections::HashSet<String>) -> bool {
    let base = name.split("::").last().unwrap_or(name);
    copy_types.contains(name)
        || copy_types.contains(base)
        || crate::type_classification::is_copy_primitive(base)
        || crate::type_classification::is_known_copy_aggregate(base)
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
    if matches!(mode, OwnershipMode::Borrowed)
        && callee_pub_owned_text_api(&sig, programs, callee_key, param_idx)
    {
        return true;
    }
    let Some((param_name, body)) = find_function_body_for_registry_key(programs, callee_key, param_idx)
    else {
        return false;
    };
    match mode {
        // WDB-155: non-Copy Custom bare-pass targets MutBorrowed, but read-only
        // `Some(Emit { table: ast.table })` field projection must stay owned (same
        // skip as Borrowed). Asymmetric skip caused `&mut SqlAst` + E0596.
        OwnershipMode::MutBorrowed => {
            param_stored_in_struct_literal(body, param_name)
                || param_forwards_fields_in_call_args_only(body, param_name)
                || param_whole_binding_returned(body, param_name)
                // WDB-158: compare helpers that only `match` the formal must stay owned
                // (not `&mut Cell` / `&mut Value`).
                || param_used_only_as_match_scrutinee(body, param_name)
        }
        OwnershipMode::Borrowed => {
            param_stored_in_struct_literal(body, param_name)
                || param_forwards_fields_in_call_args_only(body, param_name)
                || param_whole_binding_returned(body, param_name)
                || param_used_only_as_match_scrutinee(body, param_name)
        }
        OwnershipMode::Owned => false,
    }
}

/// WDB-158: formals used only as `match` scrutinees (equality/discriminant compare)
/// must not demote to `&mut T`. Nested match on the same binding still counts.
pub fn param_used_only_as_match_scrutinee(body: &[&Statement], param_name: &str) -> bool {
    let mut saw_scrutinee = false;
    for stmt in body {
        match stmt_match_scrutinee_only(stmt, param_name, &mut saw_scrutinee) {
            ScrutineeScan::OtherUse => return false,
            ScrutineeScan::Ok => {}
        }
    }
    saw_scrutinee
}

enum ScrutineeScan {
    Ok,
    OtherUse,
}

fn stmt_match_scrutinee_only(
    stmt: &Statement,
    param_name: &str,
    saw: &mut bool,
) -> ScrutineeScan {
    match stmt {
        Statement::Match { value, arms, .. } => {
            if expr_is_bare_param(value, param_name) {
                *saw = true;
            } else if expr_mentions_param(value, param_name) {
                return ScrutineeScan::OtherUse;
            }
            for arm in arms {
                if let ScrutineeScan::OtherUse =
                    expr_match_scrutinee_only(&arm.body, param_name, saw)
                {
                    return ScrutineeScan::OtherUse;
                }
                if let Some(g) = &arm.guard {
                    if expr_mentions_param(g, param_name) {
                        return ScrutineeScan::OtherUse;
                    }
                }
            }
            ScrutineeScan::Ok
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            if expr_mentions_param(condition, param_name) {
                return ScrutineeScan::OtherUse;
            }
            for s in then_block {
                if let ScrutineeScan::OtherUse = stmt_match_scrutinee_only(s, param_name, saw) {
                    return ScrutineeScan::OtherUse;
                }
            }
            if let Some(b) = else_block {
                for s in b {
                    if let ScrutineeScan::OtherUse = stmt_match_scrutinee_only(s, param_name, saw) {
                        return ScrutineeScan::OtherUse;
                    }
                }
            }
            ScrutineeScan::Ok
        }
        Statement::Expression { expr, .. }
        | Statement::Return {
            value: Some(expr), ..
        } => expr_match_scrutinee_only(expr, param_name, saw),
        Statement::Return { .. } => ScrutineeScan::Ok,
        Statement::Let { value, else_block, .. } => {
            if let ScrutineeScan::OtherUse = expr_match_scrutinee_only(value, param_name, saw) {
                return ScrutineeScan::OtherUse;
            }
            if let Some(b) = else_block {
                for s in b {
                    if let ScrutineeScan::OtherUse = stmt_match_scrutinee_only(s, param_name, saw) {
                        return ScrutineeScan::OtherUse;
                    }
                }
            }
            ScrutineeScan::Ok
        }
        _ => {
            if statement_mentions_param(stmt, param_name) {
                ScrutineeScan::OtherUse
            } else {
                ScrutineeScan::Ok
            }
        }
    }
}

fn expr_match_scrutinee_only(
    expr: &Expression,
    param_name: &str,
    saw: &mut bool,
) -> ScrutineeScan {
    match expr {
        Expression::Block { statements, .. } => {
            for s in statements {
                if let ScrutineeScan::OtherUse = stmt_match_scrutinee_only(s, param_name, saw) {
                    return ScrutineeScan::OtherUse;
                }
            }
            ScrutineeScan::Ok
        }
        _ => {
            if expr_mentions_param(expr, param_name) {
                ScrutineeScan::OtherUse
            } else {
                ScrutineeScan::Ok
            }
        }
    }
}

fn expr_is_bare_param(expr: &Expression, param_name: &str) -> bool {
    matches!(expr, Expression::Identifier { name, .. } if name == param_name)
}

/// Callee returns the whole `param` binding (directly or in a tuple) — must stay owned
/// so `(Row, T)` chain helpers can move `Row` out of the return type.
pub fn param_whole_binding_returned(body: &[&Statement], param_name: &str) -> bool {
    let len = body.len();
    for (i, stmt) in body.iter().enumerate() {
        let is_last = i == len - 1;
        match stmt {
            Statement::Return {
                value: Some(expr), ..
            } => {
                if expr_returns_whole_binding(param_name, expr) {
                    return true;
                }
            }
            Statement::Expression { expr, .. } if is_last => {
                let is_void_call = if let Expression::Call { function, .. } = expr {
                    matches!(
                        &**function,
                        Expression::Identifier { name, .. }
                            if matches!(
                                name.as_str(),
                                "println" | "print" | "eprintln" | "eprint" | "assert" | "panic"
                            )
                    )
                } else {
                    false
                };
                if !is_void_call && expr_returns_whole_binding(param_name, expr) {
                    return true;
                }
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                if param_whole_binding_returned(then_block, param_name) {
                    return true;
                }
                if let Some(b) = else_block {
                    if param_whole_binding_returned(b, param_name) {
                        return true;
                    }
                }
            }
            Statement::Match { arms, .. } => {
                for arm in arms {
                    if expr_returns_whole_binding(param_name, &arm.body) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

fn expr_returns_whole_binding(param_name: &str, expr: &Expression) -> bool {
    match expr {
        Expression::Identifier { name, .. } if name == param_name => true,
        Expression::Tuple { elements, .. } => elements
            .iter()
            .any(|elem| expr_returns_whole_binding(param_name, elem)),
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            if let Expression::Identifier { name: fn_name, .. } = &**function {
                if crate::type_classification::is_language_level_payload_call_name(fn_name) {
                    return arguments.iter().any(|(_, arg)| {
                        matches!(arg, Expression::Identifier { name, .. } if name == param_name)
                    });
                }
            }
            false
        }
        _ => false,
    }
}

/// Restore owned Custom formals demoted by bare-pass when the body returns the whole param.
pub fn restore_owned_returned_formals(
    registry: &mut SignatureRegistry,
    programs: &[&Program],
) {
    let keys: Vec<String> = registry.signatures.keys().cloned().collect();
    for key in keys {
        let Some(sig) = registry.get_signature(&key).cloned() else {
            continue;
        };
        let n = sig.param_ownership.len();
        let mut changed = false;
        let mut new_sig = sig.clone();
        for idx in 0..n {
            if !matches!(
                new_sig.param_ownership.get(idx),
                Some(OwnershipMode::MutBorrowed | OwnershipMode::Borrowed)
            ) {
                continue;
            }
            let formal_ty = new_sig
                .formal_param_types
                .get(idx)
                .or_else(|| new_sig.param_types.get(idx));
            let Some(formal_ty) = formal_ty else {
                continue;
            };
            let bare = match formal_ty {
                Type::Custom(name) => name.clone(),
                Type::Reference(inner) | Type::MutableReference(inner) => match inner.as_ref() {
                    Type::Custom(name) => name.clone(),
                    _ => continue,
                },
                _ => continue,
            };
            if is_copy_formal_name(&bare, &std::collections::HashSet::new()) {
                continue;
            }
            let Some((param_name, body)) =
                find_function_body_for_registry_key(programs, &key, idx)
            else {
                continue;
            };
            if !param_whole_binding_returned(body, param_name) {
                continue;
            }
            new_sig.param_ownership[idx] = OwnershipMode::Owned;
            let owned_ty = Type::Custom(bare);
            if new_sig.param_types.len() > idx {
                new_sig.param_types[idx] = owned_ty.clone();
            }
            if new_sig.formal_param_types.len() > idx {
                new_sig.formal_param_types[idx] = owned_ty;
            }
            if let Some(ref mut flags) = new_sig.emitted_rust_ref_params {
                if flags.len() > idx {
                    flags[idx] = false;
                }
            }
            changed = true;
        }
        if changed {
            registry.signatures.insert(key.clone(), new_sig.clone());
            if let Some(bare) = key.rsplit("::").next() {
                if bare != key.as_str() && registry.signatures.contains_key(bare) {
                    registry.signatures.insert(bare.to_string(), new_sig);
                }
            }
        }
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

/// Public module APIs (`pub fn generate_page(path: string, …) -> string`) keep owned
/// `String` formals even when cross-module callers pass bare bindings in pass 1.
fn callee_pub_owned_text_api(
    sig: &FunctionSignature,
    programs: &[&Program],
    callee_key: &str,
    param_idx: usize,
) -> bool {
    let owned_text = sig
        .formal_param_types
        .get(param_idx)
        .or_else(|| sig.param_types.get(param_idx))
        .is_some_and(|t| {
            crate::codegen::rust::types::is_windjammer_text_type(t)
                && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
        });
    if !owned_text {
        return false;
    }
    let simple = callee_key.rsplit("::").next().unwrap_or(callee_key);
    for program in programs {
        for item in &program.items {
            if let Item::Function { decl, .. } = item {
                if (decl.name == simple || callee_key.ends_with(&format!("::{simple}")))
                    && decl.is_pub
                    && decl.parent_type.is_none()
                {
                    return true;
                }
            }
        }
    }
    false
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
        Statement::Match { value, arms, .. } => {
            // WDB-155: `match bind_ast(ast) { … }` — the bare/field forward is in the
            // scrutinee call; arm bodies often never mention `ast` again.
            let mut usage = expr_param_usage(value, param_name);
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

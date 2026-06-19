//! Codegen Helper Functions
//!
//! Pure utility functions for code generation tasks like location extraction
//! and where-clause formatting. These functions have no state dependencies
//! and can be used independently.

use crate::parser::{Expression, FunctionDecl, ImplBlock, Item, Statement};
use crate::source_map::Location;
use std::collections::HashMap;

/// Extract location from an Expression
///
/// Returns the location information attached to the expression, if any.
/// Used for error reporting and source mapping.
pub fn get_expression_location(expr: &Expression) -> Option<Location> {
    expr.location().clone()
}

/// Extract location from a Statement
///
/// Returns the location information attached to the statement, if any.
/// Used for error reporting and source mapping.
pub fn get_statement_location(stmt: &Statement) -> Option<Location> {
    stmt.location().clone()
}

/// Extract location from an Item
///
/// Returns the location information attached to the item, if any.
/// Used for error reporting and source mapping.
pub fn get_item_location(item: &Item) -> Option<Location> {
    item.location().clone()
}

/// Format where clause for Rust output
///
/// Converts a list of type parameter constraints into a properly formatted
/// where clause for generated Rust code.
///
/// # Examples
/// - Empty: `""` (no where clause)
/// - Single: `[("T", ["Display"])]` → `"\nwhere\n    T: Display"`
/// - Multiple bounds: `[("T", ["Display", "Clone"])]` → `"\nwhere\n    T: Display + Clone"`
/// - Multiple params: `[("T", ["Display"]), ("U", ["Debug"])]` → `"\nwhere\n    T: Display,\n    U: Debug"`
pub fn format_where_clause(where_clause: &[(String, Vec<String>)]) -> String {
    if where_clause.is_empty() {
        return String::new();
    }

    let clauses: Vec<String> = where_clause
        .iter()
        .map(|(type_param, bounds)| format!("    {}: {}", type_param, bounds.join(" + ")))
        .collect();

    format!("\nwhere\n{}", clauses.join(",\n"))
}

/// When `parent::symbol` is written but `symbol` lives in a child module file
/// (`parent/child/symbol` from the library layout), Rust needs `parent::child::symbol`.
///
/// Built from multipass sources: for module path `[..., parent, child]` and an `extern fn`
/// or `struct` in that file, maps `(parent, symbol) -> child`. Conflicting definitions
/// (same parent + symbol from different children) are dropped from the map.
pub fn qualify_parent_child_external_path(
    map: &HashMap<(String, String), String>,
    path: &str,
) -> String {
    if map.is_empty() || !path.contains("::") {
        return path.to_string();
    }
    let parts: Vec<&str> = path.split("::").collect();
    if parts.len() == 2 {
        if let Some(child) = map.get(&(parts[0].to_string(), parts[1].to_string())) {
            return format!("{}::{}::{}", parts[0], child, parts[1]);
        }
    } else if parts.len() == 3 && parts[0] == "crate" {
        if let Some(child) = map.get(&(parts[1].to_string(), parts[2].to_string())) {
            return format!("crate::{}::{}::{}", parts[1], child, parts[2]);
        }
    }
    path.to_string()
}

/// Rust requires `impl<T> Uniform<T>` when the user writes `impl Uniform<T>` without `impl<T>`.
/// Returns type parameter names to place after `impl` (e.g. `["T"]`), or `None` if not applicable.
pub fn infer_impl_header_type_params_from_type_name(type_name: &str) -> Option<Vec<String>> {
    let open = type_name.find('<')?;
    let close = type_name.rfind('>')?;
    if close <= open || close + 1 != type_name.len() {
        return None;
    }
    let inner = type_name[open + 1..close].trim();
    if inner.is_empty() {
        return None;
    }
    let params: Vec<String> = inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if params.is_empty() {
        return None;
    }
    if !params.iter().all(|p| is_simple_rust_type_parameter_name(p)) {
        return None;
    }
    Some(params)
}

fn is_simple_rust_type_parameter_name(p: &str) -> bool {
    !p.is_empty()
        && p.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Type parameters declared on an `impl` block (explicit `impl<T>` or inferred `impl ComponentArray<T>`).
pub fn impl_block_type_param_names(impl_block: &ImplBlock<'_>) -> Vec<String> {
    if !impl_block.type_params.is_empty() {
        impl_block
            .type_params
            .iter()
            .map(|p| p.name.clone())
            .collect()
    } else {
        infer_impl_header_type_params_from_type_name(&impl_block.type_name).unwrap_or_default()
    }
}

fn expr_is_self_chain(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Identifier { name, .. } => name == "self",
        Expression::FieldAccess { object, .. } => expr_is_self_chain(object),
        _ => false,
    }
}

/// `self.dense` or `self.dense[..]` (field name `dense` on a `self` chain).
fn is_self_dense_access(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::FieldAccess { object, field, .. } if field == "dense" => {
            expr_is_self_chain(object)
        }
        _ => false,
    }
}

/// Receiver of `.clone()` is `self.dense` or `self.dense[index]` (typical `Vec<T>` storage in generic ECS arrays).
fn clone_receiver_is_self_dense_path(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::Index { object, .. } => is_self_dense_access(object),
        _ => is_self_dense_access(expr),
    }
}

fn expr_may_need_generic_clone_bound(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::MethodCall {
            object,
            method,
            arguments,
            ..
        } => {
            if matches!(
                method.as_str(),
                "clone" | "to_owned" | "to_vec" | "into_iter"
            ) && clone_receiver_is_self_dense_path(object)
            {
                return true;
            }
            if matches!(method.as_str(), "clone" | "to_owned")
                && matches!(
                    &**object,
                    Expression::FieldAccess { field, .. } if field == "data"
                )
            {
                return true;
            }
            if expr_may_need_generic_clone_bound(object) {
                return true;
            }
            for (_, arg) in arguments {
                if expr_may_need_generic_clone_bound(arg) {
                    return true;
                }
            }
            false
        }
        Expression::Binary { left, right, .. } => {
            expr_may_need_generic_clone_bound(left) || expr_may_need_generic_clone_bound(right)
        }
        Expression::Unary { operand, .. } => expr_may_need_generic_clone_bound(operand),
        Expression::Call {
            function,
            arguments,
            ..
        } => {
            if expr_may_need_generic_clone_bound(function) {
                return true;
            }
            for (_, arg) in arguments {
                if expr_may_need_generic_clone_bound(arg) {
                    return true;
                }
            }
            false
        }
        Expression::FieldAccess { object, field, .. } => {
            if field == "data"
                && matches!(&**object, Expression::Identifier { .. })
            {
                return true;
            }
            if field == "dense" && is_self_dense_access(expr) {
                return true;
            }
            expr_may_need_generic_clone_bound(object)
        }
        Expression::Index { object, index, .. } => {
            // Generic impls returning `self.dense[i]` codegen adds `.clone()` on the element.
            if is_self_dense_access(object) {
                return true;
            }
            expr_may_need_generic_clone_bound(object) || expr_may_need_generic_clone_bound(index)
        }
        Expression::StructLiteral { fields, .. } => fields
            .iter()
            .any(|(_, e)| expr_may_need_generic_clone_bound(e)),
        Expression::Tuple { elements, .. } => elements
            .iter()
            .any(|e| expr_may_need_generic_clone_bound(e)),
        Expression::Array { elements, .. } => elements
            .iter()
            .any(|e| expr_may_need_generic_clone_bound(e)),
        Expression::Block { statements, .. } => statements
            .iter()
            .any(|s| stmt_may_need_generic_clone_bound(s)),
        Expression::Cast { expr, .. } => expr_may_need_generic_clone_bound(expr),
        Expression::TryOp { expr, .. } => expr_may_need_generic_clone_bound(expr),
        Expression::Await { expr, .. } => expr_may_need_generic_clone_bound(expr),
        Expression::Closure { body, .. } => expr_may_need_generic_clone_bound(body),
        Expression::MacroInvocation { args, .. } => {
            args.iter().any(|e| expr_may_need_generic_clone_bound(e))
        }
        Expression::MapLiteral { pairs, .. } => pairs.iter().any(|(k, v)| {
            expr_may_need_generic_clone_bound(k) || expr_may_need_generic_clone_bound(v)
        }),
        Expression::Range { start, end, .. } => {
            expr_may_need_generic_clone_bound(start) || expr_may_need_generic_clone_bound(end)
        }
        Expression::ChannelSend { channel, value, .. } => {
            expr_may_need_generic_clone_bound(channel) || expr_may_need_generic_clone_bound(value)
        }
        Expression::ChannelRecv { channel, .. } => expr_may_need_generic_clone_bound(channel),
        _ => false,
    }
}

fn stmt_may_need_generic_clone_bound(stmt: &Statement<'_>) -> bool {
    match stmt {
        Statement::Let {
            value, else_block, ..
        } => {
            if expr_may_need_generic_clone_bound(value) {
                return true;
            }
            if let Some(else_b) = else_block {
                if else_b.iter().any(|s| stmt_may_need_generic_clone_bound(s)) {
                    return true;
                }
            }
            false
        }
        Statement::Const { value, .. } | Statement::Static { value, .. } => {
            expr_may_need_generic_clone_bound(value)
        }
        Statement::Assignment { target, value, .. } => {
            expr_may_need_generic_clone_bound(target) || expr_may_need_generic_clone_bound(value)
        }
        Statement::Return { value, .. } => value
            .map(|e| expr_may_need_generic_clone_bound(e))
            .unwrap_or(false),
        Statement::Expression { expr, .. } => expr_may_need_generic_clone_bound(expr),
        Statement::If {
            condition,
            then_block,
            else_block,
            ..
        } => {
            expr_may_need_generic_clone_bound(condition)
                || then_block
                    .iter()
                    .any(|s| stmt_may_need_generic_clone_bound(s))
                || else_block
                    .as_ref()
                    .map(|b| b.iter().any(|s| stmt_may_need_generic_clone_bound(s)))
                    .unwrap_or(false)
        }
        Statement::Match { value, arms, .. } => {
            expr_may_need_generic_clone_bound(value)
                || arms.iter().any(|arm| {
                    arm.guard
                        .map(|g| expr_may_need_generic_clone_bound(g))
                        .unwrap_or(false)
                        || expr_may_need_generic_clone_bound(arm.body)
                })
        }
        Statement::For { iterable, body, .. } => {
            expr_may_need_generic_clone_bound(iterable)
                || body.iter().any(|s| stmt_may_need_generic_clone_bound(s))
        }
        Statement::While {
            condition, body, ..
        } => {
            expr_may_need_generic_clone_bound(condition)
                || body.iter().any(|s| stmt_may_need_generic_clone_bound(s))
        }
        Statement::Loop { body, .. }
        | Statement::Thread { body, .. }
        | Statement::Async { body, .. } => {
            body.iter().any(|s| stmt_may_need_generic_clone_bound(s))
        }
        Statement::Break { .. } | Statement::Continue { .. } | Statement::Use { .. } => false,
        Statement::Defer { statement, .. } => stmt_may_need_generic_clone_bound(statement),
    }
}

fn function_may_need_dense_clone_bound(func: &FunctionDecl<'_>) -> bool {
    func.body
        .iter()
        .any(|s| stmt_may_need_generic_clone_bound(s))
}

/// When a generic `impl Foo<T>` clones `self.dense` (or `self.dense[i]`), Rust needs `T: Clone`
/// for `Vec<T>::clone` and element clones. Infers `where T: Clone` for single-type-parameter impls.
pub fn infer_clone_where_bounds_for_impl(impl_block: &ImplBlock<'_>) -> Vec<(String, Vec<String>)> {
    let params = impl_block_type_param_names(impl_block);
    infer_clone_where_bounds_for_generic_params(impl_block.functions.iter(), &params)
}

/// Per-method `T: Clone` when only that method clones generic storage/elements.
pub fn infer_clone_where_bounds_for_function(
    func: &FunctionDecl<'_>,
    generic_params: &[String],
) -> Vec<(String, Vec<String>)> {
    infer_clone_where_bounds_for_generic_params(std::iter::once(func), generic_params)
}

fn infer_clone_where_bounds_for_generic_params<'a>(
    functions: impl IntoIterator<Item = &'a FunctionDecl<'a>>,
    generic_params: &[String],
) -> Vec<(String, Vec<String>)> {
    if generic_params.len() != 1 {
        return Vec::new();
    }
    let needs = functions
        .into_iter()
        .any(|f| function_may_need_dense_clone_bound(f));
    if !needs {
        return Vec::new();
    }
    vec![(generic_params[0].clone(), vec!["Clone".to_string()])]
}

/// Merge `where` clauses, deduplicating trait bounds per type parameter.
pub fn merge_where_clauses(
    mut base: Vec<(String, Vec<String>)>,
    extra: Vec<(String, Vec<String>)>,
) -> Vec<(String, Vec<String>)> {
    for (param, bounds) in extra {
        if let Some((_, existing)) = base.iter_mut().find(|(p, _)| p == &param) {
            for b in bounds {
                if !existing.contains(&b) {
                    existing.push(b);
                }
            }
        } else {
            base.push((param, bounds));
        }
    }
    base
}

#[cfg(test)]
mod codegen_helpers_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn qualify_inserts_child_module_segment() {
        let mut m = HashMap::new();
        m.insert(
            ("ffi".to_string(), "tilemap_check_collision".to_string()),
            "api".to_string(),
        );
        assert_eq!(
            qualify_parent_child_external_path(&m, "ffi::tilemap_check_collision"),
            "ffi::api::tilemap_check_collision"
        );
        assert_eq!(
            qualify_parent_child_external_path(&m, "crate::ffi::tilemap_check_collision"),
            "crate::ffi::api::tilemap_check_collision"
        );
    }

    #[test]
    fn infer_impl_params_from_uniform_t() {
        assert_eq!(
            infer_impl_header_type_params_from_type_name("Uniform<T>"),
            Some(vec!["T".to_string()])
        );
    }

    #[test]
    fn infer_impl_params_rejects_concrete_type_args() {
        assert!(infer_impl_header_type_params_from_type_name("Vec<i32>").is_none());
    }
}

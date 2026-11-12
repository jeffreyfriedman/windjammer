//! A read-only `let j = v[i]` of a non-Copy element is emitted as a borrow (`let j = &v[i]`). That borrow keeps the
//! collection borrowed while `j` is live, so when the collection is changed before a later read of `j`
//! (`v.remove(i)` then `j.name`, rustc E0502) the element is copied instead. A change is an assignment into the
//! collection or a call of a method whose declared receiver is `&mut self` (std surface or the program's own
//! signatures); the binding's reads are found by liveness order over the statements after the `let`.

use crate::parser::{Expression, Statement};

use super::CodeGenerator;

/// `a.b.c` as text for an identifier/field chain (no calls, no indexing).
fn path_of(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, field, .. } => path_of(object).map(|p| format!("{p}.{field}")),
        _ => None,
    }
}

/// The identifier/field path an assignment target writes into (`v[i].x` -> `v`).
fn written_path(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::Index { object, .. } => written_path(object),
        Expression::FieldAccess { object, .. } => path_of(e).or_else(|| written_path(object)),
        other => path_of(other),
    }
}

fn overlaps(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&format!("{b}.")) || b.starts_with(&format!("{a}."))
}

impl<'ast> CodeGenerator<'ast> {
    /// `let name = <collection>[i]` (the `let` whose value is `value`) is read after `collection` is changed.
    pub(in crate::codegen::rust) fn element_borrow_outlived_by_change(&self, name: &str, value: &Expression<'ast>) -> bool {
        let Expression::Index { object, .. } = value else { return false };
        let Some(collection) = path_of(object) else { return false };
        let body: Vec<&Statement<'ast>> = if self.full_function_body_snapshot.is_empty() {
            self.current_function_body.clone()
        } else {
            self.full_function_body_snapshot.clone()
        };
        let Some(after) = Self::statements_after_let(&body, value) else { return false };
        let mut changed = false;
        for s in after {
            let changes = self.statement_changes_path(s, &collection);
            let reads = Self::variable_used_in_statements(&[s], name);
            if reads && (changed || changes) {
                return true;
            }
            changed |= changes;
        }
        false
    }

    /// The statements after the `let` whose value is `value`, in the block that holds it.
    fn statements_after_let<'b>(block: &'b [&'b Statement<'ast>], value: &Expression<'ast>) -> Option<Vec<&'b Statement<'ast>>> {
        for (i, s) in block.iter().enumerate() {
            match s {
                Statement::Let { value: v, .. } if std::ptr::eq(*v, value) => return Some(block[i + 1..].to_vec()),
                Statement::If { then_block, else_block, .. } => {
                    let found = Self::statements_after_let(then_block, value)
                        .or_else(|| else_block.as_ref().and_then(|b| Self::statements_after_let(b, value)));
                    if found.is_some() {
                        return found;
                    }
                }
                Statement::For { body, .. } | Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    if let Some(found) = Self::statements_after_let(body, value) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn statement_changes_path(&self, s: &Statement<'ast>, path: &str) -> bool {
        match s {
            Statement::Assignment { target, value, .. } => {
                written_path(target).is_some_and(|w| overlaps(&w, path)) || self.expression_changes_path(value, path)
            }
            Statement::Expression { expr, .. } | Statement::Let { value: expr, .. } => self.expression_changes_path(expr, path),
            Statement::Return { value: Some(expr), .. } => self.expression_changes_path(expr, path),
            Statement::If { condition, then_block, else_block, .. } => {
                self.expression_changes_path(condition, path)
                    || then_block.iter().chain(else_block.iter().flatten()).any(|t| self.statement_changes_path(t, path))
            }
            Statement::For { body, .. } | Statement::While { body, .. } | Statement::Loop { body, .. } => {
                body.iter().any(|t| self.statement_changes_path(t, path))
            }
            _ => false,
        }
    }

    /// A call of a `&mut self` method on `path` (or a path overlapping it), anywhere in `e`'s call chain or arguments.
    fn expression_changes_path(&self, e: &Expression<'ast>, path: &str) -> bool {
        let Expression::MethodCall { object, method, arguments, .. } = e else { return false };
        let on_path = path_of(object).is_some_and(|p| overlaps(&p, path));
        if on_path {
            let receiver = self.infer_expression_type(object).as_ref().and_then(Self::type_to_name);
            let receiver = receiver.map(|r| r.split('<').next().unwrap_or(&r).to_string());
            if crate::analyzer::stdlib_method_traits::method_mutates_receiver_qualified(method, receiver.as_deref(), &self.signature_registry) {
                return true;
            }
        }
        self.expression_changes_path(object, path) || arguments.iter().any(|(_, a)| self.expression_changes_path(a, path))
    }
}

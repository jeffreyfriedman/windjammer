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

/// The binding a field/element path starts from (`self.forms[h].s` -> `self`).
fn path_root(e: &Expression<'_>) -> Option<String> {
    match e {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, .. } | Expression::Index { object, .. } => path_root(object),
        _ => None,
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// `self.special(self.forms[h].s, ..)` where `special` takes `&mut self` and the argument is passed by shared
    /// reference: the argument borrows the receiver while the call borrows it mutably (rustc E0502), so the argument is
    /// copied first (`&(self.forms[h].s.clone())`; the copy is a temporary, the receiver is free when the call runs).
    pub(in crate::codegen::rust) fn copy_receiver_borrows_into_mut_call(
        &self,
        object: &Expression<'ast>,
        method: &str,
        arguments: &[(Option<String>, &'ast Expression<'ast>)],
        sig: &Option<crate::analyzer::FunctionSignature>,
        mut args: Vec<String>,
    ) -> Vec<String> {
        for ((_, arg), text) in arguments.iter().zip(args.iter_mut()) {
            if !self.arg_borrows_mut_receiver(object, method, sig.as_ref(), Some(arg)) || text.starts_with("&mut") {
                continue;
            }
            // The copy is parenthesized (`&(self.forms[h].s.clone())`): later argument passes drop a trailing `.clone()`
            // into a shared-reference formal, and this one is required.
            if let Some(inner) = text.strip_prefix('&') {
                *text = format!("&({}.clone())", inner.trim());
            } else if !text.ends_with(".clone()") && self.infer_expression_type(arg).is_some_and(|t| !self.is_type_copy(&t)) {
                // The borrow is added later: copy the value first.
                *text = format!("({text}.clone())");
            }
        }
        args
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// `for o in out` where `out` is a `&mut Vec` parameter: iterating the reference itself moves it, so a later
    /// `out.push(..)` is rustc E0382. The loop reborrows (`&*out`, or `&mut *out` when the binding writes through).
    pub(in crate::codegen::rust) fn reborrow_mut_param_iterable(&self, iterable: &Expression<'ast>, writes: bool, iter_expr: &mut String) {
        let Expression::Identifier { name, .. } = iterable else { return };
        if self.inferred_mut_borrowed_params.contains(name) && *iter_expr == *name {
            *iter_expr = if writes { format!("&mut *{name}") } else { format!("&*{name}") };
        }
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// `arg` is a field or element path of the receiver of a `&mut self` call (`self.special(self.forms[h].s, ..)`):
    /// passing a borrow of it while the call borrows the receiver mutably is rustc E0502, so the argument is a copy.
    /// Later argument passes must keep that copy even where the formal is a shared reference.
    pub(in crate::codegen::rust) fn arg_borrows_mut_receiver(
        &self,
        object: &Expression<'ast>,
        method: &str,
        sig: Option<&crate::analyzer::FunctionSignature>,
        arg: Option<&Expression<'ast>>,
    ) -> bool {
        let Some(arg) = arg.filter(|a| matches!(a, Expression::FieldAccess { .. } | Expression::Index { .. })) else { return false };
        let mut_receiver = sig.is_some_and(|s| {
            s.has_self_receiver && matches!(s.param_ownership.first(), Some(crate::analyzer::OwnershipMode::MutBorrowed))
        }) || self.ir_receiver_is_mut(object, method);
        // Disjoint fields of one root borrow separately (`self.renderer.upload(&self.palette)`): only an argument whose
        // place overlaps the receiver's place conflicts with the `&mut` receiver.
        let same_root = path_root(object).is_some_and(|root| path_root(arg).as_deref() == Some(root.as_str()));
        let disjoint = matches!((path_of(object), written_path(arg)), (Some(r), Some(a)) if !overlaps(&r, &a));
        mut_receiver && same_root && !disjoint
    }

    /// The IR solved the callee `Type::method` (of the receiver's type) to take `&mut self`.
    fn ir_receiver_is_mut(&self, object: &Expression<'ast>, method: &str) -> bool {
        let Some(ty) = self.infer_type_name(object) else { return false };
        let key = format!("{}::{method}", ty.split('<').next().unwrap_or(&ty));
        self.ir_module.as_ref().is_some_and(|m| {
            m.functions.iter().any(|f| {
                f.name == key && matches!(f.param_types.get("self").map(|t| &t.ownership), Some(crate::ir::safety_type::OwnedType::MutRef(_)))
            })
        })
    }
}

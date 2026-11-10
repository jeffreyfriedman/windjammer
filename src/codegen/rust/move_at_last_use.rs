//! MOVE-LAST-USE (docs/spec/inference.md section 3): the last use of an owned non-Copy binding
//! moves it; no clone is emitted there.
//!
//! The decision is the semantic layer's liveness fact (`ir::liveness::last_uses`, loaded per
//! function into `CodeGenerator::last_use_nodes`; `ir::liveness::dying_in` per statement). This
//! module only hands the fact to the clone decision: while a statement is emitted, the bindings
//! whose last use it holds are put in `AutoCloneAnalysis::dying_here`, which every reuse-clone
//! query (`needs_clone`, `needs_clone_anywhere`) consults first. The one backend-local input is
//! whether the binding is an owned value in the emitted Rust: a `&T` binding handed to an owned
//! formal is converted, not moved (CLONE-BORROWED-TO-OWNED), so it is never listed.

use super::CodeGenerator;
use crate::parser::{Statement, Type};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// The C6 facts the Rust backend reads for the function being emitted (all from `crate::ir`).
#[derive(Default)]
pub(crate) struct CloneFacts {
    /// MOVE-LAST-USE: reads that are the last use of a plain binding (`liveness::last_moves`).
    pub(crate) last_use_nodes: BTreeSet<usize>,
    /// CLONE-USE-AFTER-MOVE: per statement, bindings read after it (`liveness::read_after`).
    pub(crate) read_after: BTreeMap<usize, BTreeSet<String>>,
    /// The statements being emitted (innermost last) and their re-read reads (`liveness::reread_in`).
    pub(crate) stmt_stack: Vec<usize>,
    pub(crate) reread_stack: Vec<BTreeSet<usize>>,
    /// Parameters and plain `let` / loop bindings (`liveness::plain_bindings`).
    pub(crate) plain_bindings: BTreeSet<String>,
    /// `let xs = [..]` bindings emitted as `Vec` (ADR-0014), whatever their recorded type says.
    pub(crate) list_literal_locals: BTreeSet<String>,
    /// CLONE-BORROWED-TO-OWNED: returned parameter fields, the borrowed ones, element reads (`clone_facts`).
    pub(crate) returned_fields: BTreeMap<usize, String>,
    pub(crate) clone_sites: BTreeMap<usize, String>,
    pub(crate) element_reads: BTreeMap<usize, String>,
    /// CLONE-USE-AFTER-MOVE for partial moves (`IrFunction::field_copy_sites`).
    pub(crate) field_copy_sites: BTreeMap<usize, String>,
}

impl<'ast> CodeGenerator<'ast> {
    /// Load the facts for `func` (called once per function from the codegen environment setup).
    pub(in crate::codegen::rust) fn load_clone_facts(&mut self, func: &crate::parser::FunctionDecl<'_>) {
        let entry: Vec<&str> = func.parameters.iter().map(|p| p.name.as_str()).collect();
        let always_move = self.config.liveness_mutation_always_move;
        let ir = self.current_ir_function.as_ref();
        self.clone_facts = CloneFacts {
            last_use_nodes: crate::ir::liveness::last_moves(&entry, &func.body, always_move),
            read_after: crate::ir::liveness::read_after(&entry, &func.body, always_move),
            plain_bindings: crate::ir::liveness::plain_bindings(&entry, &func.body),
            returned_fields: ir.map(|f| f.returned_fields.clone()).unwrap_or_default(),
            clone_sites: ir.map(|f| f.clone_sites.clone()).unwrap_or_default(),
            element_reads: ir.map(|f| f.element_reads.clone()).unwrap_or_default(),
            field_copy_sites: if always_move { BTreeMap::new() } else { ir.map(|f| f.field_copy_sites.clone()).unwrap_or_default() },
            ..CloneFacts::default()
        };
    }

    /// An owned `self` receiver (`n.advance()` with `fn advance(self)`) that is read again is copied.
    pub(in crate::codegen::rust) fn copy_consumed_receiver(&self, object: &crate::parser::Expression<'_>, method: &str, obj_str: String) -> String {
        if !matches!(object, crate::parser::Expression::Identifier { .. }) {
            return obj_str;
        }
        let qualified = self.infer_expression_type(object).as_ref().and_then(Self::type_to_name).map(|tn| format!("{tn}::{method}"));
        let consumes = qualified.as_ref().is_some_and(|q| {
            self.get_signature_with_global(q).is_some_and(|sig| {
                sig.has_self_receiver && matches!(self.effective_method_self_ownership(q, sig), crate::analyzer::OwnershipMode::Owned)
            })
        });
        if consumes {
            self.copy_if_read_again(object, obj_str)
        } else {
            obj_str
        }
    }

    /// The binding is an owned value (not a Rust reference) in the emitted function.
    /// A parameter or local this function emits as a reference (an alias of it is a reference too).
    pub(in crate::codegen::rust) fn binding_is_known_borrow(&self, name: &str) -> bool {
        let known = self.current_function_params.iter().any(|p| p.name == name) || self.local_var_types.contains_key(name);
        known && name != "self" && !self.binding_emits_owned_value(name)
    }

    fn binding_emits_owned_value(&self, name: &str) -> bool {
        let is_ref = |t: &Type| matches!(t, Type::Reference(_) | Type::MutableReference(_));
        // Emission truth only: `inferred_*` can lag signature promotion.
        if self.emitted_rust_ref_formals.contains(name)
            || self.str_ref_optimized_params.contains(name)
            || self.borrowed_iterator_vars.contains(name)
            || self.mut_borrowed_iterator_vars.contains(name)
            || self.explicit_ref_let_bindings.contains(name)
        {
            return false;
        }
        if let Some(p) = self.current_function_params.iter().find(|p| p.name == name) {
            return p.name != "self" && !is_ref(&p.type_);
        }
        self.local_var_types.get(name).is_some_and(|t| !is_ref(t))
    }

    /// CLONE-BORROWED-TO-OWNED: `Some(clone?)` when `expr` is a returned field of a parameter
    /// (the IR fact's domain): clone exactly when the solver borrowed its root and the field is
    /// not Copy. `None` outside the domain.
    pub(in crate::codegen::rust) fn returned_field_clone(&self, expr: &crate::parser::Expression<'_>) -> Option<bool> {
        let node = crate::ir::numeric_defaults::node_id(expr);
        self.clone_facts.returned_fields.get(&node)?;
        let copy = self.infer_expression_type(expr).as_ref().is_some_and(|t| self.is_type_copy(t));
        Some(self.clone_facts.clone_sites.contains_key(&node) && !copy)
    }

    /// CLONE-BORROWED-TO-OWNED for elements: `Some(clone?)` when `expr` is an owned element read
    /// (the IR fact's domain): clone unless the element type is known to be Copy.
    pub(in crate::codegen::rust) fn element_read_clone(&self, expr: &crate::parser::Expression<'_>) -> Option<bool> {
        self.clone_facts.element_reads.get(&crate::ir::numeric_defaults::node_id(expr))?;
        let copy = self.infer_expression_type(expr).as_ref().is_some_and(|t| self.is_type_copy(t));
        Some(!copy)
    }

    /// CLONE-USE-AFTER-MOVE (C6): `expr` is a bare plain binding emitted as a move (`emitted` is just
    /// its name) of a known non-Copy owned value that is read again (later in the statement, after
    /// it, or in the next loop iteration; `ir::liveness`): the move copies instead. Everything else
    /// passes through unchanged. With the liveness mutation (nothing read again) this never fires.
    /// The one CLONE-USE-AFTER-MOVE decision, for every consuming position. `site` only says which reads
    /// count as "again": an element of a collection, tuple or struct literal ignores other reads in the
    /// same statement (they may be in an exclusive `match` arm: `Ok(_) => (tx, true), Err(_) => (tx, false)`).
    pub(in crate::codegen::rust) fn copy_if_read_again_at(&self, expr: &crate::parser::Expression<'_>, emitted: String, site: MoveSite) -> String {
        // Never copy a value without `Clone`, whichever rule proposed it (a binding, a field such as a
        // channel end): the move stays, and `wj check` reports the later use (WJ0007) instead of rustc
        // rejecting a missing `clone` method.
        if let Some(base) = emitted.strip_suffix(".clone()") {
            let uncloneable = match expr {
                crate::parser::Expression::Identifier { name, .. } => self.binding_known_uncloneable(name),
                crate::parser::Expression::FieldAccess { .. } => {
                    self.infer_expression_type(expr).is_some_and(|t| !self.type_known_clone(&t, 0))
                }
                _ => false,
            };
            if uncloneable {
                return base.to_string();
            }
        }
        if let crate::parser::Expression::FieldAccess { .. } = expr {
            return self.copy_field_if_root_read_again(expr, emitted, site);
        }
        let crate::parser::Expression::Identifier { name, .. } = expr else { return emitted };
        if emitted != *name || self.auto_clone_analysis.is_none() || !self.clone_facts.plain_bindings.contains(name) {
            return emitted;
        }
        // A value without `Clone` (a runtime handle, a struct holding a channel end) cannot be
        // copied: that use stays a move, and `wj check` reports it (WJ0007).
        if !self.binding_emits_owned_value(name) || !self.binding_known_non_copy(name) || self.binding_is_runtime_non_clone(name) {
            return emitted;
        }
        let ty = self.current_function_params.iter().find(|p| p.name == *name).map(|p| p.type_.clone());
        if !ty.or_else(|| self.local_var_types.get(name).cloned()).is_some_and(|t| self.type_known_clone(&t, 0)) {
            return emitted;
        }
        let node = crate::ir::numeric_defaults::node_id(expr);
        let again_in_stmt = site == MoveSite::Argument && self.clone_facts.reread_stack.last().is_some_and(|r| r.contains(&node));
        let again_after = match self.clone_facts.stmt_stack.last() {
            Some(stmt) => self.clone_facts.read_after.get(stmt).is_some_and(|names| names.contains(name)),
            None => false,
        };
        if again_in_stmt || again_after {
            format!("{name}.clone()")
        } else {
            emitted
        }
    }

    /// CLONE-USE-AFTER-MOVE for a partial move: `p.f` emitted as a move out of an owned plain binding
    /// `p` at a site the IR marked (`IrFunction::field_copy_sites`: `p` is read again afterwards, as in
    /// `Event { id: proposal.id }` then `let next = proposal`): the field is copied, so `p` stays whole.
    /// A Copy field, a reference, or a field without `Clone` passes through.
    fn copy_field_if_root_read_again(&self, expr: &crate::parser::Expression<'_>, emitted: String, site: MoveSite) -> String {
        let mut root = expr;
        let mut path = Vec::new();
        while let crate::parser::Expression::FieldAccess { object, field, .. } = root {
            path.push(field.as_str());
            root = object;
        }
        let crate::parser::Expression::Identifier { name, .. } = root else { return emitted };
        path.reverse();
        if emitted != format!("{name}.{}", path.join(".")) || self.auto_clone_analysis.is_none() || name == "self" {
            return emitted;
        }
        if !self.clone_facts.plain_bindings.contains(name) || !self.binding_emits_owned_value(name) {
            return emitted;
        }
        let Some(ty) = self.infer_expression_type(expr) else { return emitted };
        if self.is_type_copy(&ty) || matches!(ty, Type::Reference(_) | Type::MutableReference(_)) || !self.type_known_clone(&ty, 0) {
            return emitted;
        }
        // The decision is the IR's (`field_copy_sites`, from liveness); this only prints it.
        let _ = site;
        if self.clone_facts.field_copy_sites.contains_key(&crate::ir::numeric_defaults::node_id(expr)) {
            format!("{emitted}.clone()")
        } else {
            emitted
        }
    }

    pub(in crate::codegen::rust) fn copy_if_read_again(&self, expr: &crate::parser::Expression<'_>, emitted: String) -> String {
        self.copy_if_read_again_at(expr, emitted, MoveSite::Argument)
    }

    /// The binding's type is known and has no `Clone` in the emitted Rust.
    fn binding_known_uncloneable(&self, name: &str) -> bool {
        let ty = self.current_function_params.iter().find(|p| p.name == name).map(|p| p.type_.clone());
        ty.or_else(|| self.local_var_types.get(name).cloned()).is_some_and(|t| !self.type_known_clone(&t, 0))
    }

    /// `ty` implements `Clone` in the emitted Rust: no field (transitively, through the program's
    /// structs) is a type the derive rules leave without `Clone` (`type_precludes_auto_debug_clone`).
    fn type_known_clone(&self, ty: &Type, depth: u8) -> bool {
        if self.type_precludes_auto_debug_clone(ty) {
            return false;
        }
        // `Clone` of a generic value (or a type applied to one) depends on a bound we cannot see.
        if matches!(ty, Type::Generic(_))
            || matches!(ty, Type::Vec(e) | Type::Option(e) if !self.type_known_clone(e, depth + 1))
            || matches!(ty, Type::Parameterized(_, args) if args.iter().any(|a| !self.type_known_clone(a, depth + 1)))
        {
            return false;
        }
        let base = match ty {
            Type::Custom(n) | Type::Parameterized(n, _) => n.as_str(),
            _ => return true,
        };
        let leaf = base.rsplit("::").next().unwrap_or(base);
        match self.lookup_struct_field_types(leaf) {
            Some(fields) if depth < 4 => fields.values().all(|t| self.type_known_clone(t, depth + 1)),
            _ => true,
        }
    }

    /// D-CLONE-REDUNDANT: a borrow the user wrote (`&a.name`) never needs a copy to borrow from:
    /// `&a.name.clone()` becomes `&a.name`.
    pub(in crate::codegen::rust) fn drop_clone_under_borrow(&self, expr: &crate::parser::Expression<'_>, emitted: String) -> String {
        let is_user_borrow = matches!(expr, crate::parser::Expression::Unary { op: crate::parser::UnaryOp::Ref, .. });
        match emitted.strip_prefix('&').and_then(|r| r.strip_suffix(".clone()")) {
            Some(place) if is_user_borrow && !place.starts_with("mut ") && !place.contains(' ') => format!("&{place}"),
            _ => emitted,
        }
    }

    /// The binding's type is known and not Copy (a `&T` binding is Copy).
    pub(in crate::codegen::rust) fn binding_known_non_copy(&self, name: &str) -> bool {
        // A `let xs = [..]` list literal is a `Vec` even where the local type records an array.
        if self.clone_facts.list_literal_locals.contains(name) {
            return true;
        }
        let ty = self
            .current_function_params
            .iter()
            .find(|p| p.name == name)
            .map(|p| &p.type_);
        ty.or_else(|| self.local_var_types.get(name))
            .is_some_and(|t| {
                !matches!(t, Type::Reference(_) | Type::MutableReference(_))
                    && !self.is_type_copy(t)
            })
    }

    /// CLONE-USE-AFTER-MOVE: `name` is read after the statement being emitted (no fact: assume so).
    pub(in crate::codegen::rust) fn read_after_current_statement(&self, name: &str) -> bool {
        match self.clone_facts.stmt_stack.last() {
            Some(stmt) => self.clone_facts.read_after.get(stmt).is_none_or(|names| names.contains(name)),
            None => true,
        }
    }

    /// Publish the bindings that die in `stmt`; returns the enclosing statement's set.
    pub(in crate::codegen::rust) fn enter_statement_liveness(
        &mut self,
        stmt: &Statement<'_>,
    ) -> HashSet<String> {
        self.clone_facts.stmt_stack.push(stmt as *const Statement<'_> as *const () as usize);
        self.clone_facts.reread_stack.push(crate::ir::liveness::reread_in(stmt));
        let dying: HashSet<String> = if self.auto_clone_analysis.is_some() {
            let mut never = crate::ir::liveness::dying_in(stmt, &self.clone_facts.last_use_nodes);
            never.retain(|n| self.binding_emits_owned_value(n));
            // A value without `Clone` (a struct holding a channel end) is never copied: its reuse
            // after a move stays a move, which `wj check` reports (WJ0007), not a `.clone()` rustc rejects.
            never.extend(crate::ir::liveness::reads_in(stmt).into_iter().filter(|n| self.binding_known_uncloneable(n)));
            never.into_iter().collect()
        } else {
            HashSet::new()
        };
        if crate::debug_env::enabled("WJ_DEBUG_LIVENESS") {
            eprintln!("liveness {:?}: dying {:?}", self.current_function_name, dying);
        }
        match self.auto_clone_analysis.as_mut() {
            Some(a) => std::mem::replace(&mut a.dying_here, dying),
            None => HashSet::new(),
        }
    }

    pub(in crate::codegen::rust) fn exit_statement_liveness(&mut self, outer: HashSet<String>) {
        self.clone_facts.stmt_stack.pop();
        self.clone_facts.reread_stack.pop();
        if let Some(a) = self.auto_clone_analysis.as_mut() {
            a.dying_here = outer;
        }
    }
}

/// Where a value is consumed (argument / receiver / `let` alias, or a literal element).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::codegen::rust) enum MoveSite {
    Argument,
    Element,
}

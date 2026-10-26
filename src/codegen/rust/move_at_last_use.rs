//! MOVE-LAST-USE (docs/spec/inference.md section 3): the last use of an owned non-Copy binding
//! moves it; no clone is emitted there.
//!
//! The decision is the semantic layer's liveness fact (`ir::liveness::last_uses`, loaded per
//! function into `CodeGenerator::last_use_keys`; `ir::liveness::dying_in` per statement). This
//! module only hands the fact to the clone decision: while a statement is emitted, the bindings
//! whose last use it holds are put in `AutoCloneAnalysis::dying_here`, which every reuse-clone
//! query (`needs_clone`, `needs_clone_anywhere`) consults first. The one backend-local input is
//! whether the binding is an owned value in the emitted Rust: a `&T` binding handed to an owned
//! formal is converted, not moved (CLONE-BORROWED-TO-OWNED), so it is never listed.

use super::CodeGenerator;
use crate::parser::{Statement, Type};
use std::collections::HashSet;

impl<'ast> CodeGenerator<'ast> {
    /// The binding is an owned value (not a Rust reference) in the emitted function.
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

    /// CLONE-USE-AFTER-MOVE: `name` is read after the statement being emitted (no fact: assume so).
    pub(in crate::codegen::rust) fn read_after_current_statement(&self, name: &str) -> bool {
        match self.read_after.1.last().copied().flatten() {
            Some(k) => self.read_after.0.get(&k).is_none_or(|names| names.contains(name)),
            None => true,
        }
    }

    /// Publish the bindings that die in `stmt`; returns the enclosing statement's set.
    pub(in crate::codegen::rust) fn enter_statement_liveness(&mut self, stmt: &Statement<'_>) -> HashSet<String> {
        self.read_after.1.push(stmt.location().as_ref().map(|l| (l.line, l.column)));
        if crate::debug_env::enabled("WJ_DEBUG_LIVENESS") {
            eprintln!("liveness {:?}: {} last-use keys, analysis {}", self.current_function_name, self.last_use_keys.len(), self.auto_clone_analysis.is_some());
        }
        let dying: HashSet<String> = if self.auto_clone_analysis.is_some() && !self.last_use_keys.is_empty() {
            let all = crate::ir::liveness::dying_in(stmt, &self.last_use_keys);
            if crate::debug_env::enabled("WJ_DEBUG_LIVENESS") {
                let owned: Vec<_> = all.iter().filter(|n| self.binding_emits_owned_value(n)).collect();
                eprintln!("liveness {:?}: dying {:?}, owned {:?}", self.current_function_name, all, owned);
            }
            all.into_iter().filter(|n| self.binding_emits_owned_value(n)).collect()
        } else {
            HashSet::new()
        };
        match self.auto_clone_analysis.as_mut() {
            Some(a) => std::mem::replace(&mut a.dying_here, dying),
            None => HashSet::new(),
        }
    }

    pub(in crate::codegen::rust) fn exit_statement_liveness(&mut self, outer: HashSet<String>) {
        self.read_after.1.pop();
        if let Some(a) = self.auto_clone_analysis.as_mut() {
            a.dying_here = outer;
        }
    }
}

//! Binding identity in the Rust emitter: a `let` that rebinds the name of a borrowed parameter declares a new,
//! owned local, so from that statement to the end of its block the name is no longer an inferred-borrowed
//! parameter (no `*v` deref of the `usize` that `let v = v.len()` produced). The sets are name-keyed and
//! function-wide; each block restores them on exit, so a shadow in a nested block does not leak out.

use super::generator::CodeGenerator;
use crate::parser::Statement;
use std::collections::HashSet;

/// The name sets a block saved on entry (only when it rebinds one of their names).
pub(super) type SavedBorrowSets = Option<(HashSet<String>, HashSet<String>)>;

impl<'ast> CodeGenerator<'ast> {
    fn rebound_borrowed_names(&self, stmt: &Statement<'ast>) -> Vec<String> {
        let Statement::Let { pattern, .. } = stmt else { return Vec::new() };
        self.inferred_borrowed_params
            .iter()
            .chain(self.inferred_mut_borrowed_params.iter())
            .filter(|n| crate::analyzer::param_scope::binds(pattern, n))
            .cloned()
            .collect()
    }

    /// Save the sets when some `let` of this block rebinds a borrowed parameter's name.
    pub(super) fn enter_param_shadow_scope(&self, stmts: &[&'ast Statement<'ast>]) -> SavedBorrowSets {
        if self.inferred_borrowed_params.is_empty() && self.inferred_mut_borrowed_params.is_empty() {
            return None;
        }
        stmts
            .iter()
            .any(|s| !self.rebound_borrowed_names(s).is_empty())
            .then(|| (self.inferred_borrowed_params.clone(), self.inferred_mut_borrowed_params.clone()))
    }

    pub(super) fn leave_param_shadow_scope(&mut self, saved: SavedBorrowSets) {
        if let Some((borrowed, mut_borrowed)) = saved {
            self.inferred_borrowed_params = borrowed;
            self.inferred_mut_borrowed_params = mut_borrowed;
        }
    }

    /// Before statement `i`: the `let` at `i - 1` (whose initializer still read the parameter) has rebound its names.
    pub(super) fn shadow_params_after_let(&mut self, stmts: &[&'ast Statement<'ast>], i: usize) {
        if i == 0 {
            return;
        }
        for name in self.rebound_borrowed_names(stmts[i - 1]) {
            self.inferred_borrowed_params.remove(&name);
            self.inferred_mut_borrowed_params.remove(&name);
        }
    }
}

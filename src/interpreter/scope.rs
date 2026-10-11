//! Variable scopes (one `Scope` per function activation).

use super::engine::Interpreter;
use super::error::{Ctl, R};
use super::value::{Slot, Value};
use std::cell::RefCell;
use std::rc::Rc;

impl Interpreter {
    pub(crate) fn lookup_var(&self, name: &str) -> Option<Slot> {
        self.scope
            .vars
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map(|(_, s)| s.clone())
    }

    pub(crate) fn define(&mut self, name: &'static str, v: Value) {
        if name == "_" {
            return;
        }
        self.scope.vars.push((name, Rc::new(RefCell::new(v))));
    }

    pub(crate) fn push_scope(&mut self) {
        let n = self.scope.vars.len();
        self.scope.marks.push(n);
    }

    /// Leave a block scope. Variables declared in it are dropped (destructors run for values that
    /// are no longer reachable); `r` is the block's outcome, whose value stays alive.
    pub(crate) fn pop_scope(&mut self, r: R) -> R {
        let Some(n) = self.scope.marks.pop() else { return r };
        if self.drop_types.is_empty() {
            self.scope.vars.truncate(n);
            return r;
        }
        let popped = self.scope.vars.split_off(n);
        if matches!(&r, Err(e) if Self::is_exit(e)) {
            return r;
        }
        let vals = Self::slot_values_rev(&popped);
        let keep: Vec<Value> = match &r {
            Ok(v) | Err(Ctl::Return(v)) => vec![v.clone()],
            _ => vec![],
        };
        let keep_refs: Vec<&Value> = keep.iter().collect();
        match (r, self.drop_unreachable(&vals, &keep_refs)) {
            (Ok(_), Err(e)) => Err(e),
            (r, _) => r,
        }
    }

    /// Function/closure exit: drop the variables from index `from` on (parameters and locals).
    pub(crate) fn exit_scope_vars(&mut self, from: usize, r: R) -> R {
        if self.drop_types.is_empty() || matches!(&r, Err(e) if Self::is_exit(e)) {
            return r;
        }
        let popped = self.scope.vars.split_off(from.min(self.scope.vars.len()));
        let vals = Self::slot_values_rev(&popped);
        let keep: Vec<Value> = match &r {
            Ok(v) | Err(Ctl::Return(v)) => vec![v.clone()],
            _ => vec![],
        };
        let keep_refs: Vec<&Value> = keep.iter().collect();
        match (r, self.drop_unreachable(&vals, &keep_refs)) {
            (Ok(_), Err(e)) => Err(e),
            (r, _) => r,
        }
    }
}

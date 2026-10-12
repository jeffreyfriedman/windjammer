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

    /// Keep a finished frame's variable vector for the next call (saves an allocation per call).
    pub(crate) fn recycle_vars(&mut self, mut vars: Vec<(&'static str, Slot)>) {
        if self.vec_pool.len() < 64 && vars.capacity() <= 64 {
            vars.clear();
            self.vec_pool.push(vars);
        }
    }

    pub(crate) fn push_scope(&mut self) {
        let n = self.scope.vars.len();
        self.scope.marks.push(n);
    }

    /// Leave a block scope. Variables declared in it are dropped (destructors run for values that
    /// are no longer reachable); `r` is the block's outcome, whose value stays alive.
    pub(crate) fn pop_scope(&mut self, r: R) -> R {
        let Some(n) = self.scope.marks.pop() else { return r };
        self.poison_held_guards(&r, n);
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
    /// A panic unwinding past live lock guards poisons their locks (Rust's `PoisonError`).
    pub(crate) fn poison_held_guards(&self, r: &R, from: usize) {
        let Err(Ctl::Err(e)) = r else { return };
        if e.kind != super::error::ErrorKind::Panic {
            return;
        }
        for (_, slot) in self.scope.vars.iter().skip(from) {
            if let Value::Ref(p) = &*slot.borrow() {
                if let Some(t) = &p.guard {
                    if t.poisons.get() && !t.released.get() {
                        if let Some(st) = t.state() {
                            st.poisoned.set(true);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn exit_scope_vars(&mut self, from: usize, r: R) -> R {
        self.poison_held_guards(&r, from);
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

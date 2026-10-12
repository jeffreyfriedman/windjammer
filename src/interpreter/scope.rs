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
        let slot = match self.slot_pool.pop() {
            Some(s) => {
                *s.borrow_mut() = v;
                s
            }
            None => Rc::new(RefCell::new(v)),
        };
        self.scope.vars.push((name, slot));
    }

    /// Return slots nobody else holds (not captured by a closure, no `&mut` to them) to the pool.
    fn release_slots(&mut self, from: usize) {
        for (_, slot) in self.scope.vars.drain(from..) {
            if Rc::strong_count(&slot) == 1 && Rc::weak_count(&slot) == 0 && self.slot_pool.len() < 256 {
                *slot.borrow_mut() = Value::Unit;
                self.slot_pool.push(slot);
            }
        }
    }

    /// Keep a finished frame's variable vector for the next call (saves an allocation per call).
    pub(crate) fn recycle_vars(&mut self, mut vars: Vec<(&'static str, Slot)>) {
        if self.vec_pool.len() < 64 && vars.capacity() <= 64 {
            for (_, slot) in vars.drain(..) {
                if Rc::strong_count(&slot) == 1 && Rc::weak_count(&slot) == 0 && self.slot_pool.len() < 256 {
                    *slot.borrow_mut() = Value::Unit;
                    self.slot_pool.push(slot);
                }
            }
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
        if self.drop_types.is_empty() {
            self.release_slots(n);
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

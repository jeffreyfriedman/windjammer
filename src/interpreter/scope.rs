//! Variable scopes (one `Scope` per function activation).

use super::engine::Interpreter;
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

    pub(crate) fn pop_scope(&mut self) {
        if let Some(n) = self.scope.marks.pop() {
            self.scope.vars.truncate(n);
        }
    }
}

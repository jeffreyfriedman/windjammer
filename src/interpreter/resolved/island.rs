//! Walker islands: a statement or expression the resolved compiler does not lower runs in the
//! tree walker, inside the compiled function, so one unsupported construct no longer sends the whole
//! function to the walker.
//!
//! The island runs against a walker scope that holds the locals it may name. Those locals are
//! *boxed*: their `rstack` slot holds a `Value::Ref` over a walker slot (`Rc<RefCell<Value>>`), so
//! the walker reads and writes the very slots compiled code uses, takes `&mut` places to them, and
//! closures it creates capture them -- the walker's own semantics, by construction. Which locals an
//! island may name is over-approximated from the identifiers in its source tree; an island that
//! creates a closure sees every local, as the walker's closures capture the whole scope.

use super::compile::Cx;
use crate::interpreter::engine::{Interpreter, Scope};
use crate::interpreter::error::R;
use crate::interpreter::value::{Place, Value};
use crate::parser::{EnumPatternBinding, Pattern, Type};
use std::collections::HashSet;

pub(super) struct Island {
    /// Locals the walker may see, in declaration order: (name, slot).
    names: Vec<(&'static str, usize)>,
    /// Locals a `let` island declares: bound to the walker's new slots once it has run.
    declares: Vec<(&'static str, usize)>,
    module: usize,
    ret: Option<&'static Type>,
    self_ty: Option<std::rc::Rc<str>>,
}

impl Island {
    /// Compile-time: the island for a construct whose `Debug` text is `text` (`declares` are
    /// declared by the caller after this, so they are not visible to the island itself).
    pub(super) fn new(cx: &mut Cx, text: &str) -> Island {
        let names = if text.contains("Closure {") { cx.all_locals() } else { cx.visible_locals(&identifiers(text)) };
        for (_, s) in &names {
            cx.need_box(*s);
        }
        Island { names, declares: Vec::new(), module: cx.module, ret: cx.ret, self_ty: cx.self_ty.clone() }
    }

    pub(super) fn declaring(mut self, declares: Vec<(&'static str, usize)>) -> Island {
        self.declares = declares;
        self
    }

    /// Run `f` (the walker) in a scope holding the island's locals; then bind what it declared.
    pub(super) fn run(&self, it: &mut Interpreter, b: usize, f: impl FnOnce(&mut Interpreter) -> R) -> R {
        let mut scope = Scope::new(self.module);
        scope.ret_ty = self.ret;
        scope.self_ty = self.self_ty.clone();
        scope.vars = it.vec_pool.pop().unwrap_or_default();
        for (n, s) in &self.names {
            if let Value::Ref(p) = &it.rstack[b + s] {
                scope.vars.push((n, p.slot.clone()));
            }
        }
        let start = scope.vars.len();
        let saved = std::mem::replace(&mut it.scope, scope);
        it.frames.push(saved);
        let r = f(it);
        if r.is_ok() {
            for (n, s) in &self.declares {
                if let Some((_, slot)) = it.scope.vars[start.min(it.scope.vars.len())..].iter().rev().find(|(m, _)| m == n) {
                    let v = Value::Ref(Place::of_slot(slot.clone()));
                    super::fast::set(it, b + s, v);
                }
            }
        }
        let caller = it.frames.pop().expect("frame stack");
        let done = std::mem::replace(&mut it.scope, caller);
        it.recycle_vars(done.vars);
        r
    }
}

/// Identifier-like tokens of a source tree's `Debug` text (a superset of the names it uses).
fn identifiers(text: &str) -> HashSet<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect()
}

/// Names a `let` pattern binds (as the walker's `match_pattern` defines them).
pub(super) fn pattern_names(p: &'static Pattern<'static>, out: &mut Vec<&'static str>) {
    match p {
        Pattern::Identifier(n) | Pattern::MutBinding(n) | Pattern::Ref(n) | Pattern::RefMut(n) => {
            if n != "_" {
                out.push(n.as_str());
            }
        }
        Pattern::Tuple(ps) => ps.iter().for_each(|q| pattern_names(q, out)),
        Pattern::Or(ps) => {
            if let Some(q) = ps.first() {
                pattern_names(q, out);
            }
        }
        Pattern::Reference(q) => pattern_names(q, out),
        Pattern::EnumVariant(_, b) => match b {
            EnumPatternBinding::Single(n) if n != "_" => out.push(n.as_str()),
            EnumPatternBinding::Tuple(ps) => ps.iter().for_each(|q| pattern_names(q, out)),
            EnumPatternBinding::Struct(fs, _) => fs.iter().for_each(|(_, q)| pattern_names(q, out)),
            _ => {}
        },
        Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range { .. } => {}
    }
}

/// Read a boxed local as the walker evaluates a variable (`eval`: references are followed).
#[inline]
pub(super) fn read_boxed(it: &mut Interpreter, i: usize, name: &'static str) -> R {
    let v = match &it.rstack[i] {
        Value::Ref(p) => p.slot.borrow().clone(),
        // a name a walker `let` was expected to bind but did not: resolve it as the walker would
        _ => return crate::interpreter::place::strip_refs(it.eval_path_value(name)?),
    };
    match v {
        Value::Ref(_) => crate::interpreter::place::strip_refs(v),
        v => Ok(v),
    }
}

/// Read a boxed local as a call argument (`eval_keep_guard`: a guard stays a guard).
#[inline]
pub(super) fn read_boxed_arg(it: &mut Interpreter, i: usize, name: &'static str) -> R {
    let v = match &it.rstack[i] {
        Value::Ref(p) => p.slot.borrow().clone(),
        _ => it.eval_path_value(name)?,
    };
    match &v {
        Value::Ref(p) if p.guard.is_some() => Ok(v),
        Value::Ref(_) => crate::interpreter::place::strip_refs(v),
        _ => Ok(v),
    }
}

/// A fresh walker slot holding `v` (what the walker's `define` creates).
#[inline]
pub(super) fn boxed(v: Value) -> Value {
    Value::Ref(Place::temp(v))
}

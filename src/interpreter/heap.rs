//! Reachability over the interpreter's live values, and what is built on it:
//! `Rc::strong_count`, `Weak::upgrade` and `impl Drop`.
//!
//! The value model has no move tracking (`let b = a` copies a shared tree), so "who owns this?"
//! is answered by asking what is still reachable from the roots: every variable slot of the
//! running function and of its callers, module globals, and the values a caller passes in
//! explicitly (a function's return value). An `Rc` is alive while some reachable slot holds it;
//! a struct with a destructor is dropped when a scope exit leaves it unreachable.
//!
//! Moves (`moves.rs`) take moved-out slots and fields out of the roots. Known limit (documented in
//! docs/INTERPRETER.md): values held only by a pending Rust-side temporary are invisible.

use super::engine::{Interpreter, Scope};
use super::error::{ErrorKind, Ctl, R};
use super::value::*;
use std::collections::HashSet;
use std::rc::Rc;

pub(crate) struct Walker {
    seen: HashSet<usize>,
}

impl Walker {
    pub(crate) fn new() -> Self {
        Walker { seen: HashSet::new() }
    }

    /// Visit `v` and everything it holds, calling `f` on every node (pre-order).
    pub(crate) fn walk(&mut self, v: &Value, f: &mut dyn FnMut(&Value)) {
        f(v);
        match v {
            Value::Vec(items) | Value::Tuple(items) => {
                for x in items.iter() {
                    self.walk(x, f);
                }
            }
            Value::Struct(s) => {
                for (_, x) in &s.fields {
                    self.walk(x, f);
                }
            }
            Value::Enum(e) => match &e.payload {
                Payload::Unit => {}
                Payload::Tuple(xs) => {
                    for x in xs {
                        self.walk(x, f);
                    }
                }
                Payload::Struct(fs) => {
                    for (_, x) in fs {
                        self.walk(x, f);
                    }
                }
            },
            Value::Map(m) | Value::Set(m) => {
                for (k, x) in m.iter() {
                    self.walk(&k.0, f);
                    self.walk(x, f);
                }
            }
            Value::Func(c) => {
                if let Callable::Closure { env, .. } = &**c {
                    for (_, slot) in env {
                        self.walk_slot(slot, f);
                    }
                }
            }
            Value::Ref(p) => self.walk_slot(&p.slot, f),
            Value::Rc(slot) | Value::Cell(_, slot, _) => self.walk_slot(slot, f),
            Value::Native(n) => n.for_each_value(&mut |x| self.walk(x, f)),
            Value::Iter(it) => {
                if let super::iter::IterSrc::Buf { items, pos, end } = &it.src {
                    for x in &items[pos.get()..end.get()] {
                        self.walk(x, f);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn walk_slot(&mut self, slot: &Slot, f: &mut dyn FnMut(&Value)) {
        if self.seen.insert(Rc::as_ptr(slot) as usize) {
            let v = slot.borrow().clone();
            self.walk(&v, f);
        }
    }

    fn walk_scope(&mut self, s: &Scope, moves: &super::moves::Moves, f: &mut dyn FnMut(&Value)) {
        for (_, slot) in &s.vars {
            if moves.is_moved(slot) {
                continue; // the object is owned elsewhere (or nowhere)
            }
            match moves.owned_value(slot) {
                Some(v) if moves.has_moved_fields(slot) => {
                    if self.seen.insert(Rc::as_ptr(slot) as usize) {
                        self.walk(&v, f);
                    }
                }
                _ => self.walk_slot(slot, f),
            }
        }
    }
}

impl Interpreter {
    /// Visit every value reachable from the roots (plus `extra`).
    pub(crate) fn walk_roots(&self, extra: &[&Value], f: &mut dyn FnMut(&Value)) {
        let mut w = Walker::new();
        w.walk_scope(&self.scope, &self.moves, f);
        for fr in &self.frames {
            w.walk_scope(fr, &self.moves, f);
        }
        let (scopes, vals) = self.sched.parked_roots();
        for s in scopes {
            w.walk_scope(s, &self.moves, f);
        }
        for v in vals {
            w.walk(v, f);
        }
        for v in &self.rstack {
            w.walk(v, f);
        }
        for slot in self.globals.values() {
            w.walk_slot(slot, f);
        }
        for v in extra {
            w.walk(v, f);
        }
    }

    /// `Rc::strong_count`: reachable `Value::Rc` handles on `target`.
    pub(crate) fn strong_count(&self, target: &Slot) -> usize {
        let mut n = 0;
        self.walk_roots(&[], &mut |v| {
            if let Value::Rc(s) = v {
                if Rc::ptr_eq(s, target) {
                    n += 1;
                }
            }
        });
        n
    }

    pub(crate) fn weak_count(&self, target: &Slot) -> usize {
        let mut n = 0;
        self.walk_roots(&[], &mut |v| {
            if let Value::Weak(w) = v {
                if std::ptr::eq(w.as_ptr(), Rc::as_ptr(target)) {
                    n += 1;
                }
            }
        });
        n
    }

    pub(crate) fn make_struct(&mut self, name: &str, fields: Vec<(Rc<str>, Value)>, copy: bool) -> Value {
        let uid = if self.drop_types.contains(name) {
            self.next_uid += 1;
            self.next_uid
        } else {
            0
        };
        Value::Struct(Rc::new(StructVal { name: Rc::from(name), fields, copy, uid }))
    }

    /// Give every destructor-carrying struct inside `v` a fresh identity (`.clone()` makes new objects).
    pub(crate) fn fresh_identities(&mut self, v: &Value) -> Value {
        if self.drop_types.is_empty() {
            return v.clone();
        }
        match v {
            Value::Struct(s) => {
                let fields = s.fields.iter().map(|(n, x)| (n.clone(), self.fresh_identities(x))).collect();
                let uid = if s.uid != 0 {
                    self.next_uid += 1;
                    self.next_uid
                } else {
                    0
                };
                Value::Struct(Rc::new(StructVal { name: s.name.clone(), fields, copy: s.copy, uid }))
            }
            Value::Vec(xs) => Value::vec(xs.iter().map(|x| self.fresh_identities(x)).collect()),
            Value::Tuple(xs) => Value::Tuple(Rc::new(xs.iter().map(|x| self.fresh_identities(x)).collect())),
            Value::Enum(e) => {
                let payload = match &e.payload {
                    Payload::Unit => Payload::Unit,
                    Payload::Tuple(xs) => Payload::Tuple(xs.iter().map(|x| self.fresh_identities(x)).collect()),
                    Payload::Struct(fs) => Payload::Struct(fs.iter().map(|(n, x)| (n.clone(), self.fresh_identities(x))).collect()),
                };
                Value::Enum(Rc::new(EnumVal { ty: e.ty.clone(), variant: e.variant.clone(), idx: e.idx, payload }))
            }
            other => other.clone(),
        }
    }

    /// Destructor-carrying structs inside `v`, owner before fields, in declaration/element order.
    /// The flag is true for objects reached through an `Rc` (shared ownership).
    fn glue_order(v: &Value, out: &mut Vec<(u32, Value, bool)>, seen: &mut HashSet<usize>) {
        Self::glue_order_in(v, out, seen, false)
    }

    fn glue_order_in(v: &Value, out: &mut Vec<(u32, Value, bool)>, seen: &mut HashSet<usize>, shared: bool) {
        match v {
            Value::Struct(s) => {
                if s.uid != 0 {
                    out.push((s.uid, v.clone(), shared));
                }
                for (_, x) in &s.fields {
                    Self::glue_order_in(x, out, seen, shared);
                }
            }
            Value::Vec(xs) | Value::Tuple(xs) => {
                for x in xs.iter() {
                    Self::glue_order_in(x, out, seen, shared);
                }
            }
            Value::Enum(e) => match &e.payload {
                Payload::Unit => {}
                Payload::Tuple(xs) => {
                    for x in xs {
                        Self::glue_order_in(x, out, seen, shared);
                    }
                }
                Payload::Struct(fs) => {
                    for (_, x) in fs {
                        Self::glue_order_in(x, out, seen, shared);
                    }
                }
            },
            Value::Map(m) => {
                for x in m.values() {
                    Self::glue_order_in(x, out, seen, shared);
                }
            }
            Value::Rc(slot) | Value::Cell(_, slot, _) => {
                if seen.insert(Rc::as_ptr(slot) as usize) {
                    let inner = slot.borrow().clone();
                    Self::glue_order_in(&inner, out, seen, true);
                }
            }
            _ => {}
        }
    }

    /// Run destructors for the destructor-carrying values in `vals` that are no longer
    /// reachable from the roots (or `extra`). `vals` is in drop order (reverse declaration).
    pub(crate) fn drop_unreachable(&mut self, vals: &[Value], extra: &[&Value]) -> R<()> {
        self.drop_values(vals, extra, false)
    }

    /// `force`: owned objects in `vals` (not behind an `Rc`) are dropped even if still reachable.
    pub(crate) fn drop_values(&mut self, vals: &[Value], extra: &[&Value], force: bool) -> R<()> {
        if self.drop_types.is_empty() || vals.is_empty() {
            return Ok(());
        }
        let mut order = Vec::new();
        let mut seen = HashSet::new();
        for v in vals {
            Self::glue_order(v, &mut order, &mut seen);
        }
        if order.is_empty() {
            return Ok(());
        }
        let mut live: HashSet<u32> = HashSet::new();
        self.walk_roots(extra, &mut |v| {
            if let Value::Struct(s) = v {
                if s.uid != 0 {
                    live.insert(s.uid);
                }
            }
        });
        let mut first_err: Option<Ctl> = None;
        for (uid, val, shared) in order {
            if (live.contains(&uid) && !(force && !shared)) || !self.dropped.insert(uid) {
                continue;
            }
            let ty = val.type_name();
            let Some(f) = self.methods.get(&(ty, "drop".to_string())).cloned() else { continue };
            let me = Value::Ref(Place::temp(val));
            if let Err(e) = self.call_func(&f, vec![], Some(me)) {
                if first_err.is_none() {
                    first_err = Some(e);
                }
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Values of the slots being popped, in drop order (last declared first).
    /// Moved-out slots and fields are left out: their objects belong to someone else now.
    pub(crate) fn slot_values_rev(&mut self, slots: &[(&'static str, Slot)]) -> Vec<Value> {
        let vals = slots.iter().rev().filter_map(|(_, s)| self.moves.owned_value(s)).collect();
        self.moves.forget(slots);
        vals
    }

    /// An error that is a `process::exit`, which never unwinds (no destructors run).
    pub(crate) fn is_exit(e: &Ctl) -> bool {
        matches!(e, Ctl::Err(r) if matches!(r.kind, ErrorKind::Exit(_)))
    }
}

//! Move tracking for destructor timing (only active when the program has an `impl Drop`).
//!
//! Values are shared trees, so a move does not change what a variable reads; it changes who
//! *owns* the object. A moved-from slot (or a moved-out field of a struct in a slot) stays
//! readable but stops being a reachability root and is left out of its scope's drop list, so the
//! destructor runs where the new owner dies, as in Rust:
//!
//! * `let y = x` / `let y = p.field` (a partial move), aggregate literals (`S { f: x }`,
//!   `(x, y)`, `[x]`, `vec![x]`, `Some(x)`, enum variants) and consuming builtins
//!   (`push`, `push_back`, `push_front`, `insert`, `send`) move a bare local or field;
//! * a call `f(x)` links `f`'s parameter slot to the caller's `x`: when `f` moves its parameter
//!   (any rule here, `drop(p)` included), `x` is moved too, so the object dies at `f`'s end
//!   (or wherever it went) instead of at the caller's scope end. A parameter that is owned by
//!   rule -- `owned`, a `mut` binding (Q-015), or a generic `T` (always by value) -- moves `x` at the
//!   call. A parameter the callee only reads is a borrow: `x` keeps the object.
//!
//! Assigning a whole variable (or the moved field) makes it an owner again.

use super::engine::Interpreter;
use super::value::{Slot, Value};
use crate::parser::{Expression, OwnershipHint, Type};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

type E = &'static Expression<'static>;

#[derive(Default)]
pub(crate) struct Moves {
    /// Slots whose whole value has been moved out.
    slots: HashSet<usize>,
    /// Fields moved out of the struct held by a slot.
    fields: HashMap<usize, Vec<Rc<str>>>,
    /// Callee parameter slot -> the caller slot its argument came from.
    links: HashMap<usize, Slot>,
    /// Caller slots for the arguments of the call about to bind (keyed by the callee).
    pending: Option<(usize, Vec<Option<Slot>>)>,
}

fn key(s: &Slot) -> usize {
    Rc::as_ptr(s) as usize
}

impl Moves {
    pub(crate) fn is_moved(&self, s: &Slot) -> bool {
        self.slots.contains(&key(s))
    }

    pub(crate) fn has_moved_fields(&self, s: &Slot) -> bool {
        self.fields.get(&key(s)).is_some_and(|f| !f.is_empty())
    }

    /// The value a slot still owns: `None` if moved out, moved fields removed.
    pub(crate) fn owned_value(&self, s: &Slot) -> Option<Value> {
        let k = key(s);
        if self.slots.contains(&k) {
            return None;
        }
        let v = s.borrow().clone();
        match (self.fields.get(&k), &v) {
            (Some(gone), Value::Struct(sv)) => {
                let mut sv = (**sv).clone();
                sv.fields.retain(|(n, _)| !gone.contains(n));
                sv.uid = 0; // a partially moved struct has no destructor of its own (Rust forbids it)
                Some(Value::Struct(Rc::new(sv)))
            }
            _ => Some(v),
        }
    }

    /// Forget slots that are going away.
    pub(crate) fn forget(&mut self, popped: &[(&'static str, Slot)]) {
        if self.slots.is_empty() && self.fields.is_empty() && self.links.is_empty() {
            return;
        }
        for (_, s) in popped {
            let k = key(s);
            self.slots.remove(&k);
            self.fields.remove(&k);
            self.links.remove(&k);
        }
    }
}

/// Does `v` hold an object with a destructor (directly or inside an aggregate)?
fn owns_destructor(v: &Value) -> bool {
    match v {
        Value::Struct(s) => s.uid != 0 || s.fields.iter().any(|(_, x)| owns_destructor(x)),
        Value::Vec(_) | Value::Tuple(_) => v.items().iter().any(owns_destructor),
        Value::Enum(e) => match &e.payload {
            super::value::Payload::Unit => false,
            super::value::Payload::Tuple(xs) => xs.iter().any(owns_destructor),
            super::value::Payload::Struct(fs) => fs.iter().any(|(_, x)| owns_destructor(x)),
        },
        Value::Map(m) => m.values().any(owns_destructor),
        Value::Rc(_) => true,
        _ => false,
    }
}

impl Interpreter {
    fn tracking(&self) -> bool {
        !self.drop_types.is_empty()
    }

    /// `e` was used by value in a moving position: if it names a local (or a field of one) that
    /// owns a destructor, ownership leaves it.
    pub(crate) fn note_move(&mut self, e: E) {
        if !self.tracking() {
            return;
        }
        match e {
            Expression::Identifier { name, .. } => {
                if let Some(s) = self.lookup_var(name) {
                    if owns_destructor(&s.borrow()) {
                        self.move_slot(s);
                    }
                }
            }
            Expression::FieldAccess { object: Expression::Identifier { name, .. }, field, .. } => {
                if let Some(s) = self.lookup_var(name) {
                    let owns = match &*s.borrow() {
                        Value::Struct(sv) => sv.fields.iter().any(|(n, x)| **n == **field && owns_destructor(x)),
                        _ => false,
                    };
                    if owns {
                        self.move_field(s, Rc::from(field.as_str()));
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn note_moves(&mut self, es: impl IntoIterator<Item = E>) {
        if self.tracking() {
            for e in es {
                self.note_move(e);
            }
        }
    }

    fn move_slot(&mut self, mut s: Slot) {
        loop {
            self.moves.slots.insert(key(&s));
            match self.moves.links.get(&key(&s)) {
                Some(caller) => s = caller.clone(),
                None => return,
            }
        }
    }

    fn move_field(&mut self, mut s: Slot, f: Rc<str>) {
        loop {
            let e = self.moves.fields.entry(key(&s)).or_default();
            if !e.contains(&f) {
                e.push(f.clone());
            }
            match self.moves.links.get(&key(&s)) {
                Some(caller) => s = caller.clone(),
                None => return,
            }
        }
    }

    /// Assignment to `target`: the variable (or field) owns its new value.
    pub(crate) fn note_assign(&mut self, target: E) {
        if !self.tracking() {
            return;
        }
        match target {
            Expression::Identifier { name, .. } => {
                if let Some(s) = self.lookup_var(name) {
                    self.moves.slots.remove(&key(&s));
                    self.moves.fields.remove(&key(&s));
                }
            }
            Expression::FieldAccess { object: Expression::Identifier { name, .. }, field, .. } => {
                if let Some(s) = self.lookup_var(name) {
                    if let Some(fs) = self.moves.fields.get_mut(&key(&s)) {
                        fs.retain(|f| **f != **field);
                    }
                }
            }
            _ => {}
        }
    }

    /// Before calling `f` with `arguments`: params owned by rule move their argument now; the
    /// others remember the caller slot so a move inside `f` reaches back (see `bind_links`).
    pub(crate) fn prepare_arg_moves(&mut self, f: &Rc<super::engine::FuncDef>, arguments: &[(Option<String>, E)], offset: usize) {
        if !self.tracking() {
            return;
        }
        let params: Vec<_> = f.params.iter().filter(|p| p.name != "self").collect();
        let mut links = Vec::with_capacity(params.len());
        for (i, (_, a)) in arguments.iter().enumerate().skip(offset) {
            let Some(p) = params.get(i - offset) else { break };
            let owned_by_rule = p.ownership == OwnershipHint::Owned
                || p.is_mutable
                || matches!(&p.type_, Type::Custom(n) | Type::Generic(n) | Type::ImplTrait(n) if f.type_params.contains(n) || matches!(p.type_, Type::ImplTrait(_)));
            if owned_by_rule {
                self.note_move(a);
                links.push(None);
                continue;
            }
            links.push(match a {
                Expression::Identifier { name, .. } => self.lookup_var(name).filter(|s| owns_destructor(&s.borrow())),
                _ => None,
            });
        }
        self.moves.pending = links.iter().any(Option::is_some).then(|| (Rc::as_ptr(f) as usize, links));
    }

    /// After `f`'s parameters are defined (the last `n` variables of the new scope): link them.
    pub(crate) fn bind_links(&mut self, f: &Rc<super::engine::FuncDef>, first_param_var: usize) {
        let Some((fp, links)) = self.moves.pending.take() else { return };
        // destructuring parameters bind several variables: no 1:1 link
        if fp != Rc::as_ptr(f) as usize || f.params.iter().any(|p| !matches!(p.pattern, None | Some(crate::parser::Pattern::Identifier(_)))) {
            return;
        }
        let vars: Vec<Slot> = self.scope.vars.iter().skip(first_param_var).map(|(_, s)| s.clone()).collect();
        for (callee, caller) in vars.iter().zip(links) {
            if let Some(c) = caller {
                self.moves.links.insert(key(callee), c);
            }
        }
    }
}

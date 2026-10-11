//! `Rc`/`Arc`/`Weak` and the interior-mutability cells (`RefCell`, `Cell`, `Mutex`, `RwLock`).
//!
//! `Rc` is a shared slot; strong/weak counts are answered by reachability (`heap.rs`), so they
//! follow variables going out of scope and `drop(x)`. Borrow-state panics (`already borrowed`) and
//! mutex poisoning are not modelled: `borrow_mut()`/`lock()` always succeed.

use super::*;
use crate::interpreter::locks::{CellState, GuardToken};
use crate::interpreter::sched::Wait;

/// The argument with `&`-style references followed but an `Rc` kept as is.
fn raw(args: &[Value], i: usize, what: &str) -> R<Value> {
    let mut v = args.get(i).cloned().ok_or_else(|| semantic(format!("`{}` is missing argument {}", what, i + 1)))?;
    while let Value::Ref(p) = v {
        v = p.read()?;
    }
    Ok(v)
}

fn rc_slot(v: &Value, what: &str) -> R<Slot> {
    match v {
        Value::Rc(s) => Ok(s.clone()),
        other => Err(semantic(format!("`{}` expects an Rc/Arc, found {}", what, other.type_name()))),
    }
}

fn new_slot(v: Value) -> Slot {
    Rc::new(RefCell::new(v))
}

pub fn register(r: &mut Registry) {
    for t in ["Rc", "Arc", "Weak", "RefCell", "Cell", "Mutex", "RwLock"] {
        r.type_name(t);
    }
    for n in ["Rc::new", "Arc::new"] {
        r.func(n, |_, a| Ok(Value::Rc(new_slot(raw(&a, 0, "new")?))));
    }
    for n in ["Rc::clone", "Arc::clone"] {
        r.func(n, |_, a| {
            let v = raw(&a, 0, "clone")?;
            rc_slot(&v, "Rc::clone")?;
            Ok(v)
        });
    }
    for n in ["Rc::strong_count", "Arc::strong_count"] {
        r.func(n, |it, a| Ok(Value::usize(it.strong_count(&rc_slot(&raw(&a, 0, "strong_count")?, "strong_count")?))));
    }
    for n in ["Rc::weak_count", "Arc::weak_count"] {
        r.func(n, |it, a| Ok(Value::usize(it.weak_count(&rc_slot(&raw(&a, 0, "weak_count")?, "weak_count")?))));
    }
    for n in ["Rc::ptr_eq", "Arc::ptr_eq"] {
        r.func(n, |_, a| {
            let (x, y) = (rc_slot(&raw(&a, 0, "ptr_eq")?, "ptr_eq")?, rc_slot(&raw(&a, 1, "ptr_eq")?, "ptr_eq")?);
            Ok(Value::Bool(Rc::ptr_eq(&x, &y)))
        });
    }
    for n in ["Rc::downgrade", "Arc::downgrade"] {
        r.func(n, |_, a| Ok(Value::Weak(Rc::downgrade(&rc_slot(&raw(&a, 0, "downgrade")?, "downgrade")?))));
    }
    for n in ["Weak::new"] {
        r.func(n, |_, _| Ok(Value::Weak(std::rc::Weak::new())));
    }
    for n in ["Rc::try_unwrap", "Arc::try_unwrap"] {
        r.func(n, |it, a| {
            let v = raw(&a, 0, "try_unwrap")?;
            let slot = rc_slot(&v, "try_unwrap")?;
            if it.strong_count(&slot) <= 1 {
                let inner = slot.borrow().clone();
                Ok(Value::ok(inner))
            } else {
                Ok(Value::err(v))
            }
        });
    }
    for n in ["Rc::get_mut", "Arc::get_mut"] {
        r.func(n, |it, a| {
            let v = raw(&a, 0, "get_mut")?;
            let slot = rc_slot(&v, "get_mut")?;
            Ok(if it.strong_count(&slot) <= 1 && it.weak_count(&slot) == 0 {
                Value::some(Value::Ref(Place::of_slot(slot)))
            } else {
                Value::none()
            })
        });
    }
    for n in ["Rc::make_mut", "Arc::make_mut"] {
        r.func(n, |it, a| {
            // the argument is `&mut rc`: a place holding the Rc
            let place = match a.first() {
                Some(Value::Ref(p)) => p.clone(),
                _ => return Err(semantic("`make_mut` needs `&mut` of an Rc")),
            };
            let slot = rc_slot(&place.read()?, "make_mut")?;
            if it.strong_count(&slot) <= 1 {
                return Ok(Value::Ref(Place::of_slot(slot)));
            }
            // shared: clone the contents into a fresh allocation owned by this handle
            let inner = slot.borrow().clone();
            let fresh = new_slot(it.fresh_identities(&inner));
            place.write(Value::Rc(fresh.clone()))?;
            Ok(Value::Ref(Place::of_slot(fresh)))
        });
    }
    r.func("RefCell::new", |_, a| Ok(new_cell(CellKind::RefCell, raw(&a, 0, "new")?)));
    r.func("Cell::new", |_, a| Ok(new_cell(CellKind::Cell, raw(&a, 0, "new")?)));
    r.func("Mutex::new", |_, a| Ok(new_cell(CellKind::Mutex, raw(&a, 0, "new")?)));
    r.func("RwLock::new", |_, a| Ok(new_cell(CellKind::RwLock, raw(&a, 0, "new")?)));

    r.pure(Kind::Rc, "upgrade", |it, v, _| match v {
        Value::Weak(w) => {
            let _ = it;
            Ok(Value::option(w.upgrade().map(Value::Rc)))
        }
        _ => Err(semantic("`upgrade` needs a Weak")),
    });
    r.pure(Kind::Rc, "clone", |_, v, _| Ok(v.clone()));
    register_cells(r);
}

fn new_cell(k: CellKind, v: Value) -> Value {
    Value::Cell(k, new_slot(v), Rc::new(CellState::default()))
}

fn cell_parts(v: &Value) -> R<(CellKind, Slot, Rc<CellState>)> {
    match v {
        Value::Cell(k, s, st) => Ok((*k, s.clone(), st.clone())),
        other => Err(semantic(format!("expected a cell, found {}", other.type_name()))),
    }
}

fn cell_slot(v: &Value) -> R<(CellKind, Slot)> {
    cell_parts(v).map(|(k, s, _)| (k, s))
}

fn guard(slot: Slot, tok: Rc<GuardToken>) -> Value {
    Value::Ref(Place { slot, path: Vec::new(), guard: Some(tok) })
}

fn plain_ref(slot: Slot) -> Value {
    Value::Ref(Place::of_slot(slot))
}

fn borrow(v: &Value, exclusive: bool) -> Result<Value, &'static str> {
    let (_, slot, st) = cell_parts(v).map_err(|_| "not a cell")?;
    if exclusive && st.any_held() {
        return Err("already borrowed: BorrowMutError");
    }
    if !exclusive && st.exclusively_held() {
        return Err("already mutably borrowed: BorrowError");
    }
    Ok(guard(slot, st.acquire(exclusive)))
}

/// `Mutex::lock` / `RwLock::read|write`: wait (running other threads) until the lock is free.
fn lock(it: &mut Interpreter, v: &Value, exclusive: bool) -> R {
    let (_, slot, st) = cell_parts(v)?;
    it.preempt()?;
    it.block_on(Wait::Lock(st.clone(), exclusive))?;
    Ok(Value::ok(guard(slot, st.acquire(exclusive))))
}

fn register_cells(r: &mut Registry) {
    r.pure(Kind::Cell, "borrow", |_, v, _| borrow(v, false).map_err(panic_ctl));
    r.pure(Kind::Cell, "borrow_mut", |_, v, _| borrow(v, true).map_err(panic_ctl));
    r.pure(Kind::Cell, "try_borrow", |_, v, _| {
        Ok(match borrow(v, false) {
            Ok(g) => Value::ok(g),
            Err(_) => Value::err(Value::str("BorrowError")),
        })
    });
    r.pure(Kind::Cell, "try_borrow_mut", |_, v, _| {
        Ok(match borrow(v, true) {
            Ok(g) => Value::ok(g),
            Err(_) => Value::err(Value::str("BorrowMutError")),
        })
    });
    r.pure(Kind::Cell, "get_mut", |_, v, _| Ok(plain_ref(cell_slot(v)?.1)));
    r.pure(Kind::Cell, "lock", |it, v, _| lock(it, v, true));
    r.pure(Kind::Cell, "write", |it, v, _| lock(it, v, true));
    r.pure(Kind::Cell, "read", |it, v, _| lock(it, v, false));
    r.pure(Kind::Cell, "try_lock", |_, v, _| {
        let (_, slot, st) = cell_parts(v)?;
        Ok(if st.can_acquire(true) {
            Value::ok(guard(slot, st.acquire(true)))
        } else {
            Value::err(Value::enum_val("TryLockError", "WouldBlock", 0, Payload::Unit))
        })
    });
    r.pure(Kind::Cell, "get", |_, v, _| Ok(cell_slot(v)?.1.borrow().clone()));
    r.pure(Kind::Cell, "set", |_, v, a| {
        *cell_slot(v)?.1.borrow_mut() = arg(&a, 0, "set")?;
        Ok(Value::Unit)
    });
    r.pure(Kind::Cell, "replace", |_, v, a| {
        Ok(std::mem::replace(&mut *cell_slot(v)?.1.borrow_mut(), arg(&a, 0, "replace")?))
    });
    r.pure(Kind::Cell, "swap", |it, v, a| {
        let (k, slot) = cell_slot(v)?;
        if k == CellKind::Atomic {
            it.preempt()?;
            return Ok(std::mem::replace(&mut *slot.borrow_mut(), arg(&a, 0, "swap")?));
        }
        let (_, other) = cell_slot(&raw(&a, 0, "swap")?)?;
        std::mem::swap(&mut *slot.borrow_mut(), &mut *other.borrow_mut());
        Ok(Value::Unit)
    });
    r.pure(Kind::Cell, "take", |it, v, _| {
        let slot = cell_slot(v)?.1;
        let old = slot.borrow().clone();
        let fresh = it.default_like(&old)?;
        *slot.borrow_mut() = fresh;
        Ok(old)
    });
    r.pure(Kind::Cell, "into_inner", |_, v, _| {
        let (k, slot) = cell_slot(v)?;
        let inner = slot.borrow().clone();
        Ok(if matches!(k, CellKind::Mutex | CellKind::RwLock) { Value::ok(inner) } else { inner })
    });
    // cloning a cell copies its contents into a new, unshared cell
    r.pure(Kind::Cell, "clone", |it, v, _| {
        let (k, slot) = cell_slot(v)?;
        let inner = slot.borrow().clone();
        Ok(new_cell(k, it.fresh_identities(&inner)))
    });
}

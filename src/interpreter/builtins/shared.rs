//! `Rc`/`Arc`/`Weak` and the interior-mutability cells (`RefCell`, `Cell`, `Mutex`, `RwLock`).
//!
//! `Rc` is a shared slot; strong/weak counts are answered by reachability (`heap.rs`), so they
//! follow variables going out of scope and `drop(x)`. Borrow-state panics (`already borrowed`) and
//! mutex poisoning are not modelled: `borrow_mut()`/`lock()` always succeed.

use super::*;

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
        r.func(n, |_, a| Ok(Value::Weak(rc_slot(&raw(&a, 0, "downgrade")?, "downgrade")?)));
    }
    for n in ["Weak::new"] {
        r.func(n, |_, _| Ok(Value::Weak(new_slot(Value::Unit))));
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
    r.func("RefCell::new", |_, a| Ok(Value::Cell(CellKind::RefCell, new_slot(raw(&a, 0, "new")?))));
    r.func("Cell::new", |_, a| Ok(Value::Cell(CellKind::Cell, new_slot(raw(&a, 0, "new")?))));
    r.func("Mutex::new", |_, a| Ok(Value::Cell(CellKind::Mutex, new_slot(raw(&a, 0, "new")?))));
    r.func("RwLock::new", |_, a| Ok(Value::Cell(CellKind::RwLock, new_slot(raw(&a, 0, "new")?))));

    r.pure(Kind::Rc, "upgrade", |it, v, _| match v {
        Value::Weak(s) if it.strong_count(s) > 0 => Ok(Value::some(Value::Rc(s.clone()))),
        Value::Weak(_) => Ok(Value::none()),
        _ => Err(semantic("`upgrade` needs a Weak")),
    });
    r.pure(Kind::Rc, "clone", |_, v, _| Ok(v.clone()));
    register_cells(r);
}

fn cell_slot(v: &Value) -> R<(CellKind, Slot)> {
    match v {
        Value::Cell(k, s) => Ok((*k, s.clone())),
        other => Err(semantic(format!("expected a cell, found {}", other.type_name()))),
    }
}

fn guard(slot: Slot) -> Value {
    Value::Ref(Place::of_slot(slot))
}

fn register_cells(r: &mut Registry) {
    for n in ["borrow", "borrow_mut", "get_mut"] {
        r.pure(Kind::Cell, n, |_, v, _| Ok(guard(cell_slot(v)?.1)));
    }
    for n in ["lock", "write", "read", "try_lock", "try_borrow", "try_borrow_mut"] {
        r.pure(Kind::Cell, n, |_, v, _| Ok(Value::ok(guard(cell_slot(v)?.1))));
    }
    r.pure(Kind::Cell, "get", |_, v, _| Ok(cell_slot(v)?.1.borrow().clone()));
    r.pure(Kind::Cell, "set", |_, v, a| {
        *cell_slot(v)?.1.borrow_mut() = arg(&a, 0, "set")?;
        Ok(Value::Unit)
    });
    r.pure(Kind::Cell, "replace", |_, v, a| {
        Ok(std::mem::replace(&mut *cell_slot(v)?.1.borrow_mut(), arg(&a, 0, "replace")?))
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
        Ok(Value::Cell(k, new_slot(it.fresh_identities(&inner))))
    });
}

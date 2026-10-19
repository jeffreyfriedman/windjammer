//! `OrderedMap` / `OrderedSet`: insertion-ordered collections (ADR-0021 D6).
//!
//! Iteration is in insertion order; re-inserting an existing key keeps its position and replaces the
//! value. `remove` / `shift_remove` preserve the order of the rest (O(n)); `swap_remove` moves the last
//! entry into the hole (as `indexmap` does). They are host objects with interior mutability, like
//! channels: a `let b = a` moves the handle, `clone()` copies the collection.

use super::*;
use crate::interpreter::native::{downcast, Native};
use std::any::Any;
use std::cell::RefCell;

#[derive(Default)]
struct Inner {
    entries: Vec<(Value, Value)>,
    index: std::collections::BTreeMap<Key, usize>,
}

impl Inner {
    fn find(&self, k: &Value) -> Option<usize> {
        self.index.get(&Key(k.clone())).copied()
    }
    fn insert(&mut self, k: Value, v: Value) -> Option<Value> {
        match self.find(&k) {
            Some(i) => Some(std::mem::replace(&mut self.entries[i].1, v)),
            None => {
                self.index.insert(Key(k.clone()), self.entries.len());
                self.entries.push((k, v));
                None
            }
        }
    }
    fn shift_remove(&mut self, k: &Value) -> Option<(Value, Value)> {
        let i = self.find(k)?;
        let e = self.entries.remove(i);
        self.index.remove(&Key(e.0.clone()));
        for (j, (key, _)) in self.entries.iter().enumerate().skip(i) {
            self.index.insert(Key(key.clone()), j);
        }
        Some(e)
    }
    fn swap_remove(&mut self, k: &Value) -> Option<(Value, Value)> {
        let i = self.find(k)?;
        let e = self.entries.swap_remove(i);
        self.index.remove(&Key(e.0.clone()));
        if let Some((moved, _)) = self.entries.get(i) {
            self.index.insert(Key(moved.clone()), i);
        }
        Some(e)
    }
    fn rebuild(&mut self) {
        self.index = self.entries.iter().enumerate().map(|(i, (k, _))| (Key(k.clone()), i)).collect();
    }
}

struct Ordered {
    is_map: bool,
    inner: RefCell<Inner>,
}

impl Native for Ordered {
    fn type_name(&self) -> &'static str {
        if self.is_map {
            "OrderedMap"
        } else {
            "OrderedSet"
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn for_each_value(&self, f: &mut dyn FnMut(&Value)) {
        for (k, v) in &self.inner.borrow().entries {
            f(k);
            f(v);
        }
    }
    fn iter_items(&self) -> Option<Vec<Value>> {
        let i = self.inner.borrow();
        Some(i.entries.iter().map(|(k, v)| if self.is_map { Value::tuple(vec![k.clone(), v.clone()]) } else { k.clone() }).collect())
    }
    fn debug_entries(&self) -> Option<(bool, Vec<(Value, Option<Value>)>)> {
        let i = self.inner.borrow();
        Some((self.is_map, i.entries.iter().map(|(k, v)| (k.clone(), self.is_map.then(|| v.clone()))).collect()))
    }
}

fn make(is_map: bool) -> Value {
    Value::Native(Rc::new(Ordered { is_map, inner: RefCell::new(Inner::default()) }))
}

fn me(v: &Value) -> R<&Ordered> {
    downcast::<Ordered>(v).ok_or_else(|| semantic("expected an OrderedMap / OrderedSet"))
}

fn pair(k: &Value, v: &Value) -> Value {
    Value::tuple(vec![k.clone(), v.clone()])
}

pub fn register(r: &mut Registry) {
    r.type_name("OrderedMap");
    r.type_name("OrderedSet");
    for n in ["OrderedMap::new", "OrderedMap::with_capacity", "OrderedMap::default"] {
        r.func(n, |_, _| Ok(make(true)));
    }
    for n in ["OrderedSet::new", "OrderedSet::with_capacity", "OrderedSet::default"] {
        r.func(n, |_, _| Ok(make(false)));
    }
    r.func("OrderedMap::from", |it, a| {
        let m = make(true);
        fill(it, &m, arg(&a, 0, "from")?)?;
        Ok(m)
    });
    r.func("OrderedSet::from", |it, a| {
        let m = make(false);
        fill(it, &m, arg(&a, 0, "from")?)?;
        Ok(m)
    });
    for ty in ["OrderedMap", "OrderedSet"] {
        r.native(ty, "len", |_, v, _| Ok(Value::usize(me(v)?.inner.borrow().entries.len())));
        r.native(ty, "is_empty", |_, v, _| Ok(Value::Bool(me(v)?.inner.borrow().entries.is_empty())));
        r.native(ty, "clear", |_, v, _| {
            let mut i = me(v)?.inner.borrow_mut();
            i.entries.clear();
            i.index.clear();
            Ok(Value::Unit)
        });
        r.native(ty, "clone", |_, v, _| {
            let o = me(v)?;
            let c = Ordered { is_map: o.is_map, inner: RefCell::new(Inner { entries: o.inner.borrow().entries.clone(), index: o.inner.borrow().index.clone() }) };
            Ok(Value::Native(Rc::new(c)))
        });
        r.native(ty, "iter", |_, v, _| Ok(new_iter(me(v)?.iter_items().unwrap_or_default())));
        r.native(ty, "into_iter", |_, v, _| Ok(new_iter(me(v)?.iter_items().unwrap_or_default())));
        r.native(ty, "get_index", |_, v, a| {
            let o = me(v)?;
            let i = arg_usize(&a, 0, "get_index")?;
            Ok(Value::option(o.inner.borrow().entries.get(i).map(|(k, x)| if o.is_map { pair(k, x) } else { k.clone() })))
        });
        r.native(ty, "first", |_, v, _| {
            let o = me(v)?;
            Ok(Value::option(o.inner.borrow().entries.first().map(|(k, x)| if o.is_map { pair(k, x) } else { k.clone() })))
        });
        r.native(ty, "last", |_, v, _| {
            let o = me(v)?;
            Ok(Value::option(o.inner.borrow().entries.last().map(|(k, x)| if o.is_map { pair(k, x) } else { k.clone() })))
        });
        r.native(ty, "pop", |_, v, _| {
            let o = me(v)?;
            let mut i = o.inner.borrow_mut();
            let e = i.entries.pop();
            if let Some((k, _)) = &e {
                i.index.remove(&Key(k.clone()));
            }
            Ok(Value::option(e.map(|(k, x)| if o.is_map { pair(&k, &x) } else { k })))
        });
        r.native(ty, "extend", |it, v, a| {
            fill(it, v, arg(&a, 0, "extend")?)?;
            Ok(Value::Unit)
        });
        r.native(ty, "retain", |it, v, a| {
            let o = me(v)?;
            let f = arg(&a, 0, "retain")?;
            let snapshot = o.inner.borrow().entries.clone();
            let mut kept = Vec::new();
            for (k, x) in snapshot {
                let args = if o.is_map { vec![k.clone(), x.clone()] } else { vec![k.clone()] };
                if matches!(it.call_value(&f, args)?, Value::Bool(true)) {
                    kept.push((k, x));
                }
            }
            let mut i = o.inner.borrow_mut();
            i.entries = kept;
            i.rebuild();
            Ok(Value::Unit)
        });
    }
    // ---- map ----
    r.native("OrderedMap", "insert", |_, v, a| Ok(Value::option(me(v)?.inner.borrow_mut().insert(arg(&a, 0, "insert")?, arg(&a, 1, "insert")?))));
    r.native("OrderedMap", "get", |_, v, a| {
        let i = me(v)?.inner.borrow();
        Ok(Value::option(i.find(&arg(&a, 0, "get")?).map(|j| i.entries[j].1.clone())))
    });
    r.native("OrderedMap", "get_key_value", |_, v, a| {
        let i = me(v)?.inner.borrow();
        Ok(Value::option(i.find(&arg(&a, 0, "get_key_value")?).map(|j| pair(&i.entries[j].0, &i.entries[j].1))))
    });
    r.native("OrderedMap", "get_index_of", |_, v, a| Ok(Value::option(me(v)?.inner.borrow().find(&arg(&a, 0, "get_index_of")?).map(Value::usize))));
    r.native("OrderedMap", "contains_key", |_, v, a| Ok(Value::Bool(me(v)?.inner.borrow().find(&arg(&a, 0, "contains_key")?).is_some())));
    for n in ["remove", "shift_remove"] {
        r.native("OrderedMap", n, |_, v, a| Ok(Value::option(me(v)?.inner.borrow_mut().shift_remove(&arg(&a, 0, "remove")?).map(|(_, x)| x))));
    }
    r.native("OrderedMap", "swap_remove", |_, v, a| Ok(Value::option(me(v)?.inner.borrow_mut().swap_remove(&arg(&a, 0, "swap_remove")?).map(|(_, x)| x))));
    r.native("OrderedMap", "keys", |_, v, _| Ok(new_iter(me(v)?.inner.borrow().entries.iter().map(|(k, _)| k.clone()).collect())));
    r.native("OrderedMap", "values", |_, v, _| Ok(new_iter(me(v)?.inner.borrow().entries.iter().map(|(_, x)| x.clone()).collect())));
    // ---- set ----
    r.native("OrderedSet", "insert", |_, v, a| Ok(Value::Bool(me(v)?.inner.borrow_mut().insert(arg(&a, 0, "insert")?, Value::Unit).is_none())));
    r.native("OrderedSet", "contains", |_, v, a| Ok(Value::Bool(me(v)?.inner.borrow().find(&arg(&a, 0, "contains")?).is_some())));
    r.native("OrderedSet", "get_index_of", |_, v, a| Ok(Value::option(me(v)?.inner.borrow().find(&arg(&a, 0, "get_index_of")?).map(Value::usize))));
    for n in ["remove", "shift_remove"] {
        r.native("OrderedSet", n, |_, v, a| Ok(Value::Bool(me(v)?.inner.borrow_mut().shift_remove(&arg(&a, 0, "remove")?).is_some())));
    }
    r.native("OrderedSet", "swap_remove", |_, v, a| Ok(Value::Bool(me(v)?.inner.borrow_mut().swap_remove(&arg(&a, 0, "swap_remove")?).is_some())));
    fn other_keys(it: &mut Interpreter, a: &[Value]) -> R<Vec<Value>> {
        let o = arg(a, 0, "set operation")?;
        match downcast::<Ordered>(&o) {
            Some(x) => Ok(x.inner.borrow().entries.iter().map(|(k, _)| k.clone()).collect()),
            None => it.to_items(o),
        }
    }
    r.native("OrderedSet", "union", |it, v, a| {
        let mine: Vec<Value> = me(v)?.inner.borrow().entries.iter().map(|(k, _)| k.clone()).collect();
        let mut seen: std::collections::BTreeSet<Key> = mine.iter().map(|k| Key(k.clone())).collect();
        let mut out = mine;
        for k in other_keys(it, &a)? {
            if seen.insert(Key(k.clone())) {
                out.push(k);
            }
        }
        Ok(new_iter(out))
    });
    r.native("OrderedSet", "intersection", |it, v, a| {
        let theirs: std::collections::BTreeSet<Key> = other_keys(it, &a)?.into_iter().map(Key).collect();
        Ok(new_iter(me(v)?.inner.borrow().entries.iter().filter(|(k, _)| theirs.contains(&Key(k.clone()))).map(|(k, _)| k.clone()).collect()))
    });
    r.native("OrderedSet", "difference", |it, v, a| {
        let theirs: std::collections::BTreeSet<Key> = other_keys(it, &a)?.into_iter().map(Key).collect();
        Ok(new_iter(me(v)?.inner.borrow().entries.iter().filter(|(k, _)| !theirs.contains(&Key(k.clone()))).map(|(k, _)| k.clone()).collect()))
    });
}

/// `extend` / `from`: pairs for a map, elements for a set, in the source's iteration order.
fn fill(it: &mut Interpreter, target: &Value, src: Value) -> R<()> {
    let o = me(target)?;
    for x in it.to_items(src)? {
        if o.is_map {
            match x {
                Value::Tuple(t) if t.len() == 2 => {
                    o.inner.borrow_mut().insert(t[0].clone(), t[1].clone());
                }
                other => return Err(semantic(format!("OrderedMap expects (key, value) pairs, got {}", other.type_name()))),
            }
        } else {
            o.inner.borrow_mut().insert(x, Value::Unit);
        }
    }
    Ok(())
}

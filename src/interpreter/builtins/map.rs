//! `HashMap`/`BTreeMap`, `HashSet`/`BTreeSet` and the `entry` API.
//!
//! Maps are ordered by key (a valid HashMap iteration order; programs must not depend on it).

use super::*;
use std::collections::BTreeMap;

type M = BTreeMap<Key, Value>;

fn map_of(v: &Value) -> Rc<KMap> {
    match v {
        Value::Map(m) | Value::Set(m) => m.clone(),
        _ => Rc::new(KMap::default()),
    }
}

fn map_mut(v: &mut Value) -> &mut M {
    match v {
        Value::Map(m) | Value::Set(m) => &mut Rc::make_mut(m).m,
        _ => unreachable!("map method on non-map"),
    }
}

fn entry_parts(v: &Value) -> R<(Place, Value)> {
    if let Value::Struct(s) = v {
        if &*s.name == "__Entry" {
            if let (Value::Ref(p), k) = (&s.fields[0].1, &s.fields[1].1) {
                return Ok((p.clone(), k.clone()));
            }
        }
    }
    Err(semantic("not a map entry"))
}

fn entry_value(map: Place, key: Value) -> Value {
    Value::Struct(Rc::new(StructVal {
        name: Rc::from("__Entry"),
        fields: vec![(Rc::from("map"), Value::Ref(map)), (Rc::from("key"), key)],
        copy: false,
        uid: 0,
    }))
}

fn tup(k: &Key, v: &Value) -> Value {
    Value::tuple(vec![k.0.clone(), v.clone()])
}

pub fn register(r: &mut Registry) {
    // ---------------- Map ----------------
    r.pure(Kind::Map, "len", |_, v, _| Ok(Value::usize(map_of(v).len())));
    r.pure(Kind::Map, "is_empty", |_, v, _| Ok(Value::Bool(map_of(v).is_empty())));
    r.pure(Kind::Map, "get", |_, v, a| Ok(Value::option(map_of(v).get(&Key(arg(&a, 0, "get")?)).cloned())));
    r.pure(Kind::Map, "contains_key", |_, v, a| Ok(Value::Bool(map_of(v).contains_key(&Key(arg(&a, 0, "contains_key")?)))));
    r.pure(Kind::Map, "get_or_insert", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Map, "keys", |_, v, _| Ok(new_iter(map_of(v).ordered_keys())));
    r.pure(Kind::Map, "into_keys", |_, v, _| Ok(new_iter(map_of(v).ordered_keys())));
    r.pure(Kind::Map, "values", |_, v, _| Ok(new_iter(map_of(v).ordered_values())));
    r.pure(Kind::Map, "into_values", |_, v, _| Ok(new_iter(map_of(v).ordered_values())));
    r.pure(Kind::Map, "iter", |_, v, _| Ok(new_iter(map_of(v).ordered().into_iter().map(|(k, x)| tup(k, x)).collect())));
    r.pure(Kind::Map, "into_iter", |_, v, _| Ok(new_iter(map_of(v).ordered().into_iter().map(|(k, x)| tup(k, x)).collect())));
    r.pure(Kind::Map, "first_key_value", |_, v, _| Ok(Value::option(map_of(v).iter().next().map(|(k, x)| tup(k, x)))));
    r.pure(Kind::Map, "last_key_value", |_, v, _| Ok(Value::option(map_of(v).iter().next_back().map(|(k, x)| tup(k, x)))));
    r.pure(Kind::Map, "range", |_, v, a| {
        let (lo, hi) = match arg(&a, 0, "range")? {
            Value::Range(x, y, t) => (Value::Int(x, t), Value::Int(y, t)),
            _ => return Err(semantic("`range` expects a range")),
        };
        Ok(new_iter(map_of(v).iter().filter(|(k, _)| k.0.structural_cmp(&lo) != std::cmp::Ordering::Less && k.0.structural_cmp(&hi) == std::cmp::Ordering::Less).map(|(k, x)| tup(k, x)).collect()))
    });
    r.mutating(Kind::Map, "insert", |_, v, a| {
        let k = arg(&a, 0, "insert")?;
        let x = arg(&a, 1, "insert")?;
        Ok(Value::option(map_mut(v).insert(Key(k), x)))
    });
    r.mutating(Kind::Map, "remove", |_, v, a| Ok(Value::option(map_mut(v).remove(&Key(arg(&a, 0, "remove")?)))));
    r.mutating(Kind::Map, "remove_entry", |_, v, a| {
        let k = Key(arg(&a, 0, "remove_entry")?);
        Ok(Value::option(map_mut(v).remove_entry(&k).map(|(k, x)| tup(&k, &x))))
    });
    r.mutating(Kind::Map, "clear", |_, v, _| {
        map_mut(v).clear();
        Ok(Value::Unit)
    });
    r.mutating(Kind::Map, "extend", |it, v, a| {
        let src = arg(&a, 0, "extend")?;
        for x in it.to_items(src)? {
            if let Value::Tuple(t) = x {
                map_mut(v).insert(Key(t[0].clone()), t[1].clone());
            }
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Map, "retain", |it, v, a| {
        let f = arg(&a, 0, "retain")?;
        let src: Vec<(Key, Value)> = map_of(v).iter().map(|(k, x)| (k.clone(), x.clone())).collect();
        let mut out = M::new();
        for (k, x) in src {
            let keep = it.call_value(&f, vec![k.0.clone(), x.clone()])?;
            if matches!(keep, Value::Bool(true)) {
                out.insert(k, x);
            }
        }
        *map_mut(v) = out;
        Ok(Value::Unit)
    });
    r.mutating(Kind::Map, "pop_first", |_, v, _| {
        let m = map_mut(v);
        let k = m.keys().next().cloned();
        Ok(Value::option(k.and_then(|k| m.remove_entry(&k)).map(|(k, x)| tup(&k, &x))))
    });
    r.mutating(Kind::Map, "pop_last", |_, v, _| {
        let m = map_mut(v);
        let k = m.keys().next_back().cloned();
        Ok(Value::option(k.and_then(|k| m.remove_entry(&k)).map(|(k, x)| tup(&k, &x))))
    });
    r.place(Kind::Map, "get_mut", |_, p, a| {
        let k = arg(&a, 0, "get_mut")?;
        let present = map_of(&p.read()?).contains_key(&Key(k.clone()));
        Ok(if present { Value::some(Value::Ref(p.proj(Proj::Key(k)))) } else { Value::none() })
    });
    r.place(Kind::Map, "entry", |_, p, a| Ok(entry_value(p, arg(&a, 0, "entry")?)));
    r.place(Kind::Map, "iter_mut", |_, p, _| {
        let keys: Vec<Value> = map_of(&p.read()?).ordered_keys();
        Ok(new_iter(keys.into_iter().map(|k| Value::tuple(vec![k.clone(), Value::Ref(p.proj(Proj::Key(k)))])).collect()))
    });
    r.place(Kind::Map, "values_mut", |_, p, _| {
        let keys: Vec<Value> = map_of(&p.read()?).ordered_keys();
        Ok(new_iter(keys.into_iter().map(|k| Value::Ref(p.proj(Proj::Key(k)))).collect()))
    });

    // ---------------- Entry ----------------
    r.pure(Kind::Entry, "or_insert", |_, v, a| {
        let (map, key) = entry_parts(v)?;
        let present = map_of(&map.read()?).contains_key(&Key(key.clone()));
        if !present {
            let x = arg(&a, 0, "or_insert")?;
            map.with_mut(|m| {
                map_mut(m).insert(Key(key.clone()), x);
                Ok(())
            })?;
        }
        Ok(Value::Ref(map.proj(Proj::Key(key))))
    });
    r.pure(Kind::Entry, "or_insert_with", |it, v, a| {
        let (map, key) = entry_parts(v)?;
        let present = map_of(&map.read()?).contains_key(&Key(key.clone()));
        if !present {
            let f = arg(&a, 0, "or_insert_with")?;
            let x = it.call_value(&f, vec![])?;
            map.with_mut(|m| {
                map_mut(m).insert(Key(key.clone()), x);
                Ok(())
            })?;
        }
        Ok(Value::Ref(map.proj(Proj::Key(key))))
    });
    r.pure(Kind::Entry, "or_default", |it, v, _| {
        let (map, key) = entry_parts(v)?;
        let present = map_of(&map.read()?).contains_key(&Key(key.clone()));
        if !present {
            let x = match it.type_hint.take() {
                Some(t) => it.default_for(&t)?,
                None => Value::int(0),
            };
            map.with_mut(|m| {
                map_mut(m).insert(Key(key.clone()), x);
                Ok(())
            })?;
        }
        Ok(Value::Ref(map.proj(Proj::Key(key))))
    });
    r.pure(Kind::Entry, "and_modify", |it, v, a| {
        let (map, key) = entry_parts(v)?;
        let present = map_of(&map.read()?).contains_key(&Key(key.clone()));
        if present {
            let f = arg(&a, 0, "and_modify")?;
            it.call_value(&f, vec![Value::Ref(map.proj(Proj::Key(key.clone())))])?;
        }
        Ok(v.clone())
    });
    r.pure(Kind::Entry, "key", |_, v, _| Ok(entry_parts(v)?.1));

    // ---------------- Set ----------------
    r.pure(Kind::Set, "len", |_, v, _| Ok(Value::usize(map_of(v).len())));
    r.pure(Kind::Set, "is_empty", |_, v, _| Ok(Value::Bool(map_of(v).is_empty())));
    r.pure(Kind::Set, "contains", |_, v, a| Ok(Value::Bool(map_of(v).contains_key(&Key(arg(&a, 0, "contains")?)))));
    r.pure(Kind::Set, "iter", |_, v, _| Ok(new_iter(map_of(v).ordered_keys())));
    r.pure(Kind::Set, "into_iter", |_, v, _| Ok(new_iter(map_of(v).ordered_keys())));
    r.pure(Kind::Set, "first", |_, v, _| Ok(Value::option(map_of(v).keys().next().map(|k| k.0.clone()))));
    r.pure(Kind::Set, "last", |_, v, _| Ok(Value::option(map_of(v).keys().next_back().map(|k| k.0.clone()))));
    fn setop(v: &Value, a: &[Value], f: fn(&M, &M) -> Vec<Key>) -> R {
        let other = map_of(&arg(a, 0, "set op")?);
        Ok(new_iter(f(&map_of(v), &other).into_iter().map(|k| k.0).collect()))
    }
    r.pure(Kind::Set, "union", |_, v, a| {
        setop(v, &a, |x, y| {
            let mut s: Vec<Key> = x.keys().cloned().collect();
            for k in y.keys() {
                if !x.contains_key(k) {
                    s.push(k.clone());
                }
            }
            s.sort();
            s
        })
    });
    r.pure(Kind::Set, "intersection", |_, v, a| setop(v, &a, |x, y| x.keys().filter(|k| y.contains_key(*k)).cloned().collect()));
    r.pure(Kind::Set, "difference", |_, v, a| setop(v, &a, |x, y| x.keys().filter(|k| !y.contains_key(*k)).cloned().collect()));
    r.pure(Kind::Set, "symmetric_difference", |_, v, a| {
        setop(v, &a, |x, y| {
            let mut s: Vec<Key> = x.keys().filter(|k| !y.contains_key(*k)).cloned().collect();
            s.extend(y.keys().filter(|k| !x.contains_key(*k)).cloned());
            s.sort();
            s
        })
    });
    r.pure(Kind::Set, "is_subset", |_, v, a| {
        let o = map_of(&arg(&a, 0, "is_subset")?);
        Ok(Value::Bool(map_of(v).keys().all(|k| o.contains_key(k))))
    });
    r.pure(Kind::Set, "is_superset", |_, v, a| {
        let o = map_of(&arg(&a, 0, "is_superset")?);
        let m = map_of(v);
        Ok(Value::Bool(o.keys().all(|k| m.contains_key(k))))
    });
    r.pure(Kind::Set, "is_disjoint", |_, v, a| {
        let o = map_of(&arg(&a, 0, "is_disjoint")?);
        Ok(Value::Bool(map_of(v).keys().all(|k| !o.contains_key(k))))
    });
    r.mutating(Kind::Set, "insert", |_, v, a| Ok(Value::Bool(map_mut(v).insert(Key(arg(&a, 0, "insert")?), Value::Unit).is_none())));
    r.mutating(Kind::Set, "remove", |_, v, a| Ok(Value::Bool(map_mut(v).remove(&Key(arg(&a, 0, "remove")?)).is_some())));
    r.mutating(Kind::Set, "take", |_, v, a| {
        let k = Key(arg(&a, 0, "take")?);
        Ok(Value::option(map_mut(v).remove_entry(&k).map(|(k, _)| k.0)))
    });
    r.mutating(Kind::Set, "clear", |_, v, _| {
        map_mut(v).clear();
        Ok(Value::Unit)
    });
    r.mutating(Kind::Set, "extend", |it, v, a| {
        let src = arg(&a, 0, "extend")?;
        for x in it.to_items(src)? {
            map_mut(v).insert(Key(x), Value::Unit);
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Set, "retain", |it, v, a| {
        let f = arg(&a, 0, "retain")?;
        let src: Vec<Key> = map_of(v).keys().cloned().collect();
        let mut out = M::new();
        for k in src {
            if matches!(it.call_value(&f, vec![k.0.clone()])?, Value::Bool(true)) {
                out.insert(k, Value::Unit);
            }
        }
        *map_mut(v) = out;
        Ok(Value::Unit)
    });
}

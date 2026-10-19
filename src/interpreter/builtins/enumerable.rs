//! Enumerable collection methods (ADR-0018 feature 1): the interpreter's implementation of the surface
//! declared in `crate::stdlib_surface::enumerable`. Registered on `Kind::Iter`, so lists, sets, maps and
//! ranges reach them too. Semantics mirror `crates/windjammer-runtime/src/enumerable.rs`.

use super::seq::{add_ints, call, truthy};
use super::*;
use std::collections::BTreeMap;

fn items(it: &mut Interpreter, v: &Value) -> R<Vec<Value>> {
    it.to_items(v.clone())
}

fn count(n: i128) -> Value {
    Value::typed(n, IntTy::I64)
}

fn map_of(m: BTreeMap<Key, Value>) -> Value {
    Value::Map(Rc::new(m))
}

/// Calls the key closure on every item, in order.
fn keyed(it: &mut Interpreter, v: &Value, f: &Value) -> R<Vec<(Value, Value)>> {
    let xs = items(it, v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        out.push((call(it, f, vec![x.clone()])?, x));
    }
    Ok(out)
}

fn group_by(it: &mut Interpreter, v: &Value, a: Vec<Value>) -> R {
    let f = arg(&a, 0, "group_by")?;
    let mut m: BTreeMap<Key, Value> = BTreeMap::new();
    for (k, x) in keyed(it, v, &f)? {
        match m.entry(Key(k)).or_insert_with(|| Value::vec(vec![])) {
            Value::Vec(run) => Rc::make_mut(run).push(x),
            _ => unreachable!("group_by buckets are lists"),
        }
    }
    Ok(map_of(m))
}

fn count_keys(keys: impl IntoIterator<Item = Value>) -> Value {
    let mut m: BTreeMap<Key, i128> = BTreeMap::new();
    for k in keys {
        *m.entry(Key(k)).or_insert(0) += 1;
    }
    map_of(m.into_iter().map(|(k, n)| (k, count(n))).collect())
}

fn unique_by_key(pairs: Vec<(Value, Value)>) -> Value {
    let mut seen: Vec<Value> = Vec::new();
    let mut out = Vec::new();
    for (k, x) in pairs {
        if !seen
            .iter()
            .any(|s| s.structural_cmp(&k) == std::cmp::Ordering::Equal)
        {
            seen.push(k);
            out.push(x);
        }
    }
    Value::vec(out)
}

fn chunk_by(it: &mut Interpreter, v: &Value, a: Vec<Value>) -> R {
    let f = arg(&a, 0, "chunk_by")?;
    let mut runs: Vec<Vec<Value>> = Vec::new();
    let mut last: Option<Value> = None;
    for (k, x) in keyed(it, v, &f)? {
        let same = last
            .as_ref()
            .is_some_and(|p| p.structural_cmp(&k) == std::cmp::Ordering::Equal);
        match runs.last_mut() {
            Some(run) if same => run.push(x),
            _ => runs.push(vec![x]),
        }
        last = Some(k);
    }
    Ok(Value::vec(runs.into_iter().map(Value::vec).collect()))
}

fn each_slice(it: &mut Interpreter, v: &Value, a: Vec<Value>) -> R {
    let n = arg_usize(&a, 0, "each_slice")?;
    if n == 0 {
        return Err(panic_ctl("chunk size must be non-zero"));
    }
    let xs = items(it, v)?;
    Ok(Value::vec(
        xs.chunks(n).map(|c| Value::vec(c.to_vec())).collect(),
    ))
}

fn sorted_by_key(it: &mut Interpreter, v: &Value, f: &Value, desc: bool) -> R<Vec<Value>> {
    let mut pairs = keyed(it, v, f)?;
    // `sort_by` is stable: equal keys keep their order in both directions.
    pairs.sort_by(|a, b| {
        if desc {
            b.0.structural_cmp(&a.0)
        } else {
            a.0.structural_cmp(&b.0)
        }
    });
    Ok(pairs.into_iter().map(|(_, x)| x).collect())
}

pub fn register(r: &mut Registry) {
    r.pure(Kind::Iter, "group_by", group_by);
    r.pure(Kind::Iter, "count_by", |it, v, a| {
        let f = arg(&a, 0, "count_by")?;
        Ok(count_keys(keyed(it, v, &f)?.into_iter().map(|(k, _)| k)))
    });
    for name in ["tally", "frequencies"] {
        r.pure(Kind::Iter, name, |it, v, _| Ok(count_keys(items(it, v)?)));
    }
    r.pure(Kind::Iter, "sum_by", |it, v, a| {
        let f = arg(&a, 0, "sum_by")?;
        let mut acc: Option<Value> = None;
        for (k, _) in keyed(it, v, &f)? {
            acc = Some(match acc {
                None => k,
                Some(s) => add_ints(&s, &k)?,
            });
        }
        Ok(acc.unwrap_or_else(|| Value::int(0)))
    });
    r.pure(Kind::Iter, "unique", |it, v, _| {
        let xs = items(it, v)?;
        Ok(unique_by_key(
            xs.into_iter().map(|x| (x.clone(), x)).collect(),
        ))
    });
    r.pure(Kind::Iter, "unique_by", |it, v, a| {
        let f = arg(&a, 0, "unique_by")?;
        Ok(unique_by_key(keyed(it, v, &f)?))
    });
    r.pure(Kind::Iter, "chunk_by", chunk_by);
    r.pure(Kind::Iter, "each_slice", each_slice);
    r.pure(Kind::Iter, "partition_by", |it, v, a| {
        let f = arg(&a, 0, "partition_by")?;
        let (mut yes, mut no) = (Vec::new(), Vec::new());
        for x in items(it, v)? {
            if truthy(&call(it, &f, vec![x.clone()])?) {
                yes.push(x)
            } else {
                no.push(x)
            }
        }
        Ok(Value::tuple(vec![Value::vec(yes), Value::vec(no)]))
    });
    r.pure(Kind::Iter, "intersperse", |it, v, a| {
        let sep = arg(&a, 0, "intersperse")?;
        let mut out = Vec::new();
        for (i, x) in items(it, v)?.into_iter().enumerate() {
            if i > 0 {
                out.push(sep.clone());
            }
            out.push(x);
        }
        Ok(Value::vec(out))
    });
    r.pure(Kind::Iter, "zip_with", |it, v, a| {
        let other = arg(&a, 0, "zip_with")?;
        let f = arg(&a, 1, "zip_with")?;
        let (xs, ys) = (items(it, v)?, items(it, &other)?);
        let mut out = Vec::new();
        for (x, y) in xs.into_iter().zip(ys) {
            out.push(call(it, &f, vec![x, y])?);
        }
        Ok(Value::vec(out))
    });
    r.pure(Kind::Iter, "index_of", |it, v, a| {
        let x = arg(&a, 0, "index_of")?;
        let pos = items(it, v)?
            .iter()
            .position(|y| y.structural_cmp(&x) == std::cmp::Ordering::Equal);
        Ok(Value::option(pos.map(Value::usize)))
    });
    r.pure(Kind::Iter, "sorted", |it, v, _| {
        let mut xs = items(it, v)?;
        xs.sort_by(|a, b| a.structural_cmp(b));
        Ok(Value::vec(xs))
    });
    r.pure(Kind::Iter, "sorted_by_key", |it, v, a| {
        let f = arg(&a, 0, "sorted_by_key")?;
        Ok(Value::vec(sorted_by_key(it, v, &f, false)?))
    });
    r.pure(Kind::Iter, "sorted_by_key_desc", |it, v, a| {
        let f = arg(&a, 0, "sorted_by_key_desc")?;
        Ok(Value::vec(sorted_by_key(it, v, &f, true)?))
    });
    r.pure(Kind::Iter, "to_list", |it, v, _| {
        Ok(Value::vec(items(it, v)?))
    });
    r.mutating(Kind::Vec, "sort_by_key_desc", |it, v, a| {
        let f = arg(&a, 0, "sort_by_key_desc")?;
        let sorted = sorted_by_key(it, &v.clone(), &f, true)?;
        *v = Value::vec(sorted);
        Ok(Value::Unit)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The interpreter implements every method of the shared surface table.
    #[test]
    fn implements_the_whole_surface() {
        let r = Registry::standard();
        for m in crate::stdlib_surface::enumerable::METHODS {
            let k = if m.mutating { Kind::Vec } else { Kind::Iter };
            assert!(r.has_method(k, m.name), "interpreter lacks `{}`", m.name);
        }
    }
}

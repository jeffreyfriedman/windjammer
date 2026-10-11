//! `Vec` / slice / `VecDeque` methods and the iterator adaptor protocol.
//!
//! Iterators are eager: adaptors materialise their items. That keeps closures free to call
//! back into the interpreter while preserving Rust's observable results for finite sources.

use super::*;
use crate::parser::Type;
use std::cmp::Ordering;

fn vec_of(v: &Value) -> Rc<Vec<Value>> {
    match v {
        Value::Vec(x) => x.clone(),
        _ => Rc::new(Vec::new()),
    }
}

fn vec_mut(v: &mut Value) -> &mut Vec<Value> {
    if !matches!(v, Value::Vec(_)) {
        *v = Value::vec(Vec::new());
    }
    match v {
        Value::Vec(x) => Rc::make_mut(x),
        _ => unreachable!(),
    }
}

fn truthy(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
}

fn call(it: &mut Interpreter, f: &Value, args: Vec<Value>) -> R {
    it.call_value(f, args)
}

fn items(it: &mut Interpreter, v: &Value) -> R<Vec<Value>> {
    it.to_items(v.clone())
}

/// Sort `items` with a fallible comparator.
fn sort_items(items: &mut [Value], mut cmp: impl FnMut(&Value, &Value) -> R<Ordering>) -> R<()> {
    let mut err = None;
    items.sort_by(|a, b| match cmp(a, b) {
        Ok(o) => o,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            Ordering::Equal
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn ord_from_closure(it: &mut Interpreter, f: &Value, a: &Value, b: &Value) -> R<Ordering> {
    let r = it.call_value(f, vec![a.clone(), b.clone()])?;
    if let Some(o) = ordering_of(&r) {
        return Ok(o);
    }
    if let Value::Enum(e) = &r {
        if &*e.ty == "Option" {
            if let Payload::Tuple(p) = &e.payload {
                if let Some(o) = p.first().and_then(ordering_of) {
                    return Ok(o);
                }
            }
        }
    }
    Err(semantic("comparator must return an Ordering"))
}

fn add_ints(a: &Value, b: &Value) -> R {
    match (a, b) {
        (Value::Int(x, xt), Value::Int(y, yt)) => crate::interpreter::ops::int_binary(crate::parser::BinaryOp::Add, *x, *xt, *y, *yt),
        (Value::Float(x, xt), Value::Float(y, _)) => Ok(Value::Float(x + y, *xt)),
        (Value::Float(x, xt), Value::Int(y, _)) => Ok(Value::Float(x + *y as f64, *xt)),
        (Value::Int(x, _), Value::Float(y, yt)) => Ok(Value::Float(*x as f64 + y, *yt)),
        _ => Err(semantic("cannot sum these values")),
    }
}

fn mul_ints(a: &Value, b: &Value) -> R {
    match (a, b) {
        (Value::Int(x, xt), Value::Int(y, yt)) => crate::interpreter::ops::int_binary(crate::parser::BinaryOp::Mul, *x, *xt, *y, *yt),
        (Value::Float(x, xt), Value::Float(y, _)) => Ok(Value::Float(x * y, *xt)),
        _ => Err(semantic("cannot multiply these values")),
    }
}

/// Build the collection named by `hint` (default `Vec`) from items.
pub fn collect_into(items: Vec<Value>, hint: Option<&Type>) -> R {
    let name = match hint {
        Some(Type::String) => "String".to_string(),
        Some(Type::Vec(_)) | Some(Type::Array(..)) | None | Some(Type::Infer) => "Vec".to_string(),
        Some(Type::Parameterized(n, _)) => n.rsplit("::").next().unwrap_or(n).to_string(),
        Some(Type::Custom(n)) => n.rsplit("::").next().unwrap_or(n).to_string(),
        Some(Type::Result(..)) => "Result".to_string(),
        Some(Type::Option(_)) => "Option".to_string(),
        Some(Type::Reference(i)) | Some(Type::MutableReference(i)) => return collect_into(items, Some(i)),
        Some(_) => "Vec".to_string(),
    };
    match name.as_str() {
        "String" | "str" => {
            let mut s = String::new();
            for x in &items {
                match x {
                    Value::Char(c) => s.push(*c),
                    Value::Str(t) => s.push_str(t),
                    other => return Err(semantic(format!("cannot collect {} into String", other.type_name()))),
                }
            }
            Ok(Value::str(s))
        }
        "HashMap" | "BTreeMap" => {
            let mut m = std::collections::BTreeMap::new();
            for x in items {
                match x {
                    Value::Tuple(t) if t.len() == 2 => {
                        m.insert(Key(t[0].clone()), t[1].clone());
                    }
                    other => return Err(semantic(format!("cannot collect {} into a map", other.type_name()))),
                }
            }
            Ok(Value::Map(Rc::new(m)))
        }
        "HashSet" | "BTreeSet" => {
            let mut m = std::collections::BTreeMap::new();
            for x in items {
                m.insert(Key(x), Value::Unit);
            }
            Ok(Value::Set(Rc::new(m)))
        }
        "Result" => {
            let inner = match hint {
                Some(Type::Result(ok, _)) => Some(&**ok),
                _ => None,
            };
            let mut out = Vec::new();
            for x in items {
                match &x {
                    Value::Enum(e) if &*e.ty == "Result" => match (&e.payload, e.idx) {
                        (Payload::Tuple(p), 0) => out.push(p[0].clone()),
                        _ => return Ok(x),
                    },
                    _ => out.push(x),
                }
            }
            Ok(Value::ok(collect_into(out, inner)?))
        }
        "Option" => {
            let inner = match hint {
                Some(Type::Option(i)) => Some(&**i),
                _ => None,
            };
            let mut out = Vec::new();
            for x in items {
                match &x {
                    Value::Enum(e) if &*e.ty == "Option" => match (&e.payload, e.idx) {
                        (Payload::Tuple(p), 1) => out.push(p[0].clone()),
                        _ => return Ok(Value::none()),
                    },
                    _ => out.push(x),
                }
            }
            Ok(Value::some(collect_into(out, inner)?))
        }
        _ => Ok(Value::vec(items)),
    }
}

pub fn register(r: &mut Registry) {
    register_vec(r);
    register_iter(r);
}

fn register_vec(r: &mut Registry) {
    r.pure(Kind::Vec, "len", |_, v, _| Ok(Value::usize(vec_of(v).len())));
    r.pure(Kind::Vec, "is_empty", |_, v, _| Ok(Value::Bool(vec_of(v).is_empty())));
    r.pure(Kind::Vec, "capacity", |_, v, _| Ok(Value::usize(vec_of(v).capacity())));
    r.pure(Kind::Vec, "slice", |it, v, a| {
        let (x, y) = (arg_int(&a, 0, "slice")?, arg_int(&a, 1, "slice")?);
        it.index_value(v, &Value::Range(x, y, IntTy::Usize))
    });
    r.pure(Kind::Vec, "iter", |_, v, _| Ok(new_iter(vec_of(v).to_vec())));
    r.pure(Kind::Vec, "into_iter", |_, v, _| Ok(new_iter(vec_of(v).to_vec())));
    r.pure(Kind::Vec, "to_vec", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Vec, "as_slice", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Vec, "into_boxed_slice", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Vec, "first", |_, v, _| Ok(Value::option(vec_of(v).first().cloned())));
    r.pure(Kind::Vec, "last", |_, v, _| Ok(Value::option(vec_of(v).last().cloned())));
    r.pure(Kind::Vec, "front", |_, v, _| Ok(Value::option(vec_of(v).first().cloned())));
    r.pure(Kind::Vec, "back", |_, v, _| Ok(Value::option(vec_of(v).last().cloned())));
    r.pure(Kind::Vec, "get", |_, v, a| match arg(&a, 0, "get")? {
        Value::Int(i, _) => Ok(Value::option(usize::try_from(i).ok().and_then(|i| vec_of(v).get(i).cloned()))),
        Value::Range(x, y, _) => {
            let vv = vec_of(v);
            let (x, y) = (x as usize, y as usize);
            Ok(if x <= y && y <= vv.len() { Value::some(Value::vec(vv[x..y].to_vec())) } else { Value::none() })
        }
        _ => Err(semantic("`get` expects an index")),
    });
    r.pure(Kind::Vec, "contains", |_, v, a| {
        let x = arg(&a, 0, "contains")?;
        Ok(Value::Bool(vec_of(v).iter().any(|y| y.structural_eq(&x))))
    });
    r.pure(Kind::Vec, "starts_with", |_, v, a| {
        let p = vec_of(&arg(&a, 0, "starts_with")?);
        let vv = vec_of(v);
        Ok(Value::Bool(vv.len() >= p.len() && vv.iter().zip(p.iter()).all(|(x, y)| x.structural_eq(y))))
    });
    r.pure(Kind::Vec, "ends_with", |_, v, a| {
        let p = vec_of(&arg(&a, 0, "ends_with")?);
        let vv = vec_of(v);
        Ok(Value::Bool(vv.len() >= p.len() && vv[vv.len() - p.len()..].iter().zip(p.iter()).all(|(x, y)| x.structural_eq(y))))
    });
    r.pure(Kind::Vec, "join", |it, v, a| {
        let sep = arg_str(&a, 0, "join")?;
        let mut parts = Vec::new();
        for x in vec_of(v).iter() {
            parts.push(it.display_string(x)?);
        }
        Ok(Value::str(parts.join(&sep)))
    });
    r.pure(Kind::Vec, "concat", |it, v, _| {
        let vv = vec_of(v);
        if vv.first().map(|x| matches!(x, Value::Str(_))).unwrap_or(false) {
            let mut s = String::new();
            for x in vv.iter() {
                s.push_str(&it.display_string(x)?);
            }
            return Ok(Value::str(s));
        }
        let mut out = Vec::new();
        for x in vv.iter() {
            out.extend(vec_of(x).iter().cloned());
        }
        Ok(Value::vec(out))
    });
    r.pure(Kind::Vec, "windows", |_, v, a| {
        let n = arg_usize(&a, 0, "windows")?;
        if n == 0 {
            return Err(panic_ctl("window size must be non-zero"));
        }
        Ok(new_iter(vec_of(v).windows(n).map(|w| Value::vec(w.to_vec())).collect()))
    });
    r.pure(Kind::Vec, "chunks", |_, v, a| {
        let n = arg_usize(&a, 0, "chunks")?;
        if n == 0 {
            return Err(panic_ctl("chunk size must be non-zero"));
        }
        Ok(new_iter(vec_of(v).chunks(n).map(|w| Value::vec(w.to_vec())).collect()))
    });
    r.pure(Kind::Vec, "split_at", |_, v, a| {
        let n = arg_usize(&a, 0, "split_at")?;
        let vv = vec_of(v);
        if n > vv.len() {
            return Err(panic_ctl("mid > len"));
        }
        Ok(Value::tuple(vec![Value::vec(vv[..n].to_vec()), Value::vec(vv[n..].to_vec())]))
    });
    r.pure(Kind::Vec, "split_first", |_, v, _| {
        let vv = vec_of(v);
        Ok(match vv.split_first() {
            Some((h, t)) => Value::some(Value::tuple(vec![h.clone(), Value::vec(t.to_vec())])),
            None => Value::none(),
        })
    });
    r.pure(Kind::Vec, "split_last", |_, v, _| {
        let vv = vec_of(v);
        Ok(match vv.split_last() {
            Some((l, init)) => Value::some(Value::tuple(vec![l.clone(), Value::vec(init.to_vec())])),
            None => Value::none(),
        })
    });
    r.pure(Kind::Vec, "binary_search", |_, v, a| {
        let x = arg(&a, 0, "binary_search")?;
        let vv = vec_of(v);
        let (mut lo, mut hi) = (0usize, vv.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match vv[mid].structural_cmp(&x) {
                Ordering::Equal => return Ok(Value::ok(Value::usize(mid))),
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
            }
        }
        Ok(Value::err(Value::usize(lo)))
    });
    r.pure(Kind::Vec, "is_sorted", |_, v, _| {
        let vv = vec_of(v);
        Ok(Value::Bool(vv.windows(2).all(|w| w[0].structural_cmp(&w[1]) != Ordering::Greater)))
    });
    r.pure(Kind::Vec, "repeat", |_, v, a| {
        let n = arg_usize(&a, 0, "repeat")?;
        let vv = vec_of(v);
        let mut out = Vec::with_capacity(vv.len() * n);
        for _ in 0..n {
            out.extend(vv.iter().cloned());
        }
        Ok(Value::vec(out))
    });

    r.mutating(Kind::Vec, "push", |_, v, a| {
        vec_mut(v).push(arg(&a, 0, "push")?);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "push_back", |_, v, a| {
        vec_mut(v).push(arg(&a, 0, "push_back")?);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "push_front", |_, v, a| {
        vec_mut(v).insert(0, arg(&a, 0, "push_front")?);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "pop", |_, v, _| Ok(Value::option(vec_mut(v).pop())));
    r.mutating(Kind::Vec, "pop_back", |_, v, _| Ok(Value::option(vec_mut(v).pop())));
    r.mutating(Kind::Vec, "pop_front", |_, v, _| {
        let vv = vec_mut(v);
        Ok(if vv.is_empty() { Value::none() } else { Value::some(vv.remove(0)) })
    });
    r.mutating(Kind::Vec, "clear", |_, v, _| {
        vec_mut(v).clear();
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "truncate", |_, v, a| {
        vec_mut(v).truncate(arg_usize(&a, 0, "truncate")?);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "insert", |_, v, a| {
        let i = arg_usize(&a, 0, "insert")?;
        let x = arg(&a, 1, "insert")?;
        let vv = vec_mut(v);
        if i > vv.len() {
            return Err(panic_ctl(format!("insertion index (is {}) should be <= len (is {})", i, vv.len())));
        }
        vv.insert(i, x);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "remove", |_, v, a| {
        let i = arg_usize(&a, 0, "remove")?;
        let vv = vec_mut(v);
        if i >= vv.len() {
            return Err(panic_ctl(format!("removal index (is {}) should be < len (is {})", i, vv.len())));
        }
        Ok(vv.remove(i))
    });
    r.mutating(Kind::Vec, "swap_remove", |_, v, a| {
        let i = arg_usize(&a, 0, "swap_remove")?;
        let vv = vec_mut(v);
        if i >= vv.len() {
            return Err(panic_ctl(format!("swap_remove index (is {}) should be < len (is {})", i, vv.len())));
        }
        Ok(vv.swap_remove(i))
    });
    r.mutating(Kind::Vec, "swap", |_, v, a| {
        let (i, j) = (arg_usize(&a, 0, "swap")?, arg_usize(&a, 1, "swap")?);
        let vv = vec_mut(v);
        if i >= vv.len() || j >= vv.len() {
            return Err(panic_ctl(format!("index out of bounds: the len is {} but the index is {}", vv.len(), i.max(j))));
        }
        vv.swap(i, j);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "reverse", |_, v, _| {
        vec_mut(v).reverse();
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "resize", |_, v, a| {
        let n = arg_usize(&a, 0, "resize")?;
        let x = arg(&a, 1, "resize")?;
        vec_mut(v).resize(n, x);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "fill", |_, v, a| {
        let x = arg(&a, 0, "fill")?;
        for e in vec_mut(v).iter_mut() {
            *e = x.clone();
        }
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "extend", |it, v, a| {
        let src = arg(&a, 0, "extend")?;
        let new = items(it, &src)?;
        vec_mut(v).extend(new);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "extend_from_slice", |it, v, a| {
        let src = arg(&a, 0, "extend_from_slice")?;
        let new = items(it, &src)?;
        vec_mut(v).extend(new);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "append", |_, v, a| {
        // arg is `&mut other`
        let other = a.first().cloned().ok_or_else(|| semantic("append needs an argument"))?;
        let moved = match other {
            Value::Ref(p) => {
                let cur = p.read()?;
                p.write(Value::vec(Vec::new()))?;
                cur
            }
            o => o,
        };
        vec_mut(v).extend(vec_of(&moved).iter().cloned());
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "split_off", |_, v, a| {
        let n = arg_usize(&a, 0, "split_off")?;
        let vv = vec_mut(v);
        if n > vv.len() {
            return Err(panic_ctl("`at` split index out of bounds"));
        }
        Ok(Value::vec(vv.split_off(n)))
    });
    r.mutating(Kind::Vec, "rotate_left", |_, v, a| {
        let n = arg_usize(&a, 0, "rotate_left")?;
        let vv = vec_mut(v);
        if n > vv.len() {
            return Err(panic_ctl("assertion failed: mid <= self.len()"));
        }
        vv.rotate_left(n);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "rotate_right", |_, v, a| {
        let n = arg_usize(&a, 0, "rotate_right")?;
        let vv = vec_mut(v);
        if n > vv.len() {
            return Err(panic_ctl("assertion failed: k <= self.len()"));
        }
        vv.rotate_right(n);
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "dedup", |_, v, _| {
        vec_mut(v).dedup_by(|a, b| a.structural_eq(b));
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "dedup_by_key", |it, v, a| {
        let f = arg(&a, 0, "dedup_by_key")?;
        let src = vec_mut(v).clone();
        let mut out: Vec<Value> = Vec::new();
        let mut last: Option<Value> = None;
        for x in src {
            let k = call(it, &f, vec![x.clone()])?;
            if last.as_ref().map(|l| l.structural_eq(&k)).unwrap_or(false) {
                continue;
            }
            last = Some(k);
            out.push(x);
        }
        *vec_mut(v) = out;
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "retain", |it, v, a| {
        let f = arg(&a, 0, "retain")?;
        let src = vec_mut(v).clone();
        let mut out = Vec::new();
        for x in src {
            if truthy(&call(it, &f, vec![x.clone()])?) {
                out.push(x);
            }
        }
        *vec_mut(v) = out;
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "drain", |_, v, a| {
        let vv = vec_mut(v);
        let (x, y) = match a.first() {
            Some(Value::Range(x, y, _)) => (*x as usize, *y as usize),
            None => (0, vv.len()),
            _ => return Err(semantic("`drain` expects a range")),
        };
        if x > y || y > vv.len() {
            return Err(panic_ctl("drain range out of bounds"));
        }
        let drained: Vec<Value> = vv.drain(x..y).collect();
        Ok(new_iter(drained))
    });
    r.mutating(Kind::Vec, "sort", |_, v, _| {
        vec_mut(v).sort_by(|a, b| a.structural_cmp(b));
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "sort_unstable", |_, v, _| {
        vec_mut(v).sort_by(|a, b| a.structural_cmp(b));
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "sort_by", |it, v, a| {
        let f = arg(&a, 0, "sort_by")?;
        sort_items(vec_mut(v), |x, y| ord_from_closure(it, &f, x, y))?;
        Ok(Value::Unit)
    });
    r.mutating(Kind::Vec, "sort_unstable_by", |it, v, a| {
        let f = arg(&a, 0, "sort_unstable_by")?;
        sort_items(vec_mut(v), |x, y| ord_from_closure(it, &f, x, y))?;
        Ok(Value::Unit)
    });
    for name in ["sort_by_key", "sort_unstable_by_key", "sort_by_cached_key"] {
        r.mutating(Kind::Vec, name, |it, v, a| {
            let f = arg(&a, 0, "sort_by_key")?;
            let src = vec_mut(v).clone();
            let mut keyed: Vec<(Value, Value)> = Vec::new();
            for x in src {
                let k = call(it, &f, vec![x.clone()])?;
                keyed.push((k, x));
            }
            keyed.sort_by(|a, b| a.0.structural_cmp(&b.0));
            *vec_mut(v) = keyed.into_iter().map(|(_, x)| x).collect();
            Ok(Value::Unit)
        });
    }
    r.place(Kind::Vec, "iter_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(new_iter((0..n).map(|i| Value::Ref(p.proj(Proj::Index(i)))).collect()))
    });
    r.place(Kind::Vec, "get_mut", |_, p, a| {
        let i = arg_int(&a, 0, "get_mut")?;
        let n = vec_of(&p.read()?).len() as i128;
        Ok(if i >= 0 && i < n { Value::some(Value::Ref(p.proj(Proj::Index(i as usize)))) } else { Value::none() })
    });
    r.place(Kind::Vec, "first_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(p.proj(Proj::Index(0)))) } else { Value::none() })
    });
    r.place(Kind::Vec, "last_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(p.proj(Proj::Index(n - 1)))) } else { Value::none() })
    });
    r.place(Kind::Vec, "front_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(p.proj(Proj::Index(0)))) } else { Value::none() })
    });
    r.place(Kind::Vec, "back_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(p.proj(Proj::Index(n - 1)))) } else { Value::none() })
    });
}

fn register_iter(r: &mut Registry) {
    r.pure(Kind::Iter, "iter", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Iter, "into_iter", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Iter, "peekable", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Iter, "by_ref", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Iter, "cloned", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Iter, "copied", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Iter, "len", |it, v, _| match v {
        Value::Range(a, b, _) => Ok(Value::usize((*b - *a).max(0) as usize)),
        Value::Iter(s) => {
            let st = s.borrow();
            Ok(Value::usize(st.items.len() - st.pos))
        }
        other => Ok(Value::usize(items(it, other)?.len())),
    });
    r.pure(Kind::Iter, "is_empty", |it, v, _| match v {
        Value::Range(a, b, _) => Ok(Value::Bool(*b <= *a)),
        Value::Iter(s) => {
            let st = s.borrow();
            Ok(Value::Bool(st.items.len() == st.pos))
        }
        other => Ok(Value::Bool(items(it, other)?.is_empty())),
    });
    r.pure(Kind::Iter, "next", |_, v, _| match v {
        Value::Iter(s) => {
            let mut st = s.borrow_mut();
            if st.pos < st.items.len() {
                let x = st.items[st.pos].clone();
                st.pos += 1;
                Ok(Value::some(x))
            } else {
                Ok(Value::none())
            }
        }
        _ => Err(semantic("`next` needs an iterator; call `.iter()` first")),
    });
    r.pure(Kind::Iter, "next_back", |_, v, _| match v {
        Value::Iter(s) => {
            let mut st = s.borrow_mut();
            if st.pos < st.items.len() {
                Ok(Value::some(st.items.pop().unwrap()))
            } else {
                Ok(Value::none())
            }
        }
        _ => Err(semantic("`next_back` needs an iterator")),
    });
    r.pure(Kind::Iter, "peek", |_, v, _| match v {
        Value::Iter(s) => {
            let st = s.borrow();
            Ok(Value::option(st.items.get(st.pos).cloned()))
        }
        _ => Err(semantic("`peek` needs an iterator")),
    });
    r.pure(Kind::Iter, "next_if", |it, v, a| match v {
        Value::Iter(s) => {
            let f = arg(&a, 0, "next_if")?;
            let cand = {
                let st = s.borrow();
                st.items.get(st.pos).cloned()
            };
            match cand {
                Some(x) if truthy(&call(it, &f, vec![x.clone()])?) => {
                    s.borrow_mut().pos += 1;
                    Ok(Value::some(x))
                }
                _ => Ok(Value::none()),
            }
        }
        _ => Err(semantic("`next_if` needs an iterator")),
    });
    r.pure(Kind::Iter, "map", |it, v, a| {
        let f = arg(&a, 0, "map")?;
        let mut out = Vec::new();
        for x in items(it, v)? {
            out.push(call(it, &f, vec![x])?);
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "filter", |it, v, a| {
        let f = arg(&a, 0, "filter")?;
        let mut out = Vec::new();
        for x in items(it, v)? {
            if truthy(&call(it, &f, vec![x.clone()])?) {
                out.push(x);
            }
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "filter_map", |it, v, a| {
        let f = arg(&a, 0, "filter_map")?;
        let mut out = Vec::new();
        for x in items(it, v)? {
            let r = call(it, &f, vec![x])?;
            if let Some(Some(y)) = r.as_option() {
                out.push(y.clone());
            }
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "flat_map", |it, v, a| {
        let f = arg(&a, 0, "flat_map")?;
        let mut out = Vec::new();
        for x in items(it, v)? {
            let r = call(it, &f, vec![x])?;
            out.extend(items(it, &r)?);
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "flatten", |it, v, _| {
        let mut out = Vec::new();
        for x in items(it, v)? {
            out.extend(items(it, &x)?);
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "enumerate", |it, v, _| {
        Ok(new_iter(items(it, v)?.into_iter().enumerate().map(|(i, x)| Value::tuple(vec![Value::usize(i), x])).collect()))
    });
    r.pure(Kind::Iter, "zip", |it, v, a| {
        let other = arg(&a, 0, "zip")?;
        let o = items(it, &other)?;
        Ok(new_iter(items(it, v)?.into_iter().zip(o).map(|(x, y)| Value::tuple(vec![x, y])).collect()))
    });
    r.pure(Kind::Iter, "chain", |it, v, a| {
        let other = arg(&a, 0, "chain")?;
        let mut x = items(it, v)?;
        x.extend(items(it, &other)?);
        Ok(new_iter(x))
    });
    r.pure(Kind::Iter, "rev", |it, v, _| {
        let mut x = items(it, v)?;
        x.reverse();
        Ok(new_iter(x))
    });
    r.pure(Kind::Iter, "take", |it, v, a| {
        let n = arg_usize(&a, 0, "take")?;
        Ok(new_iter(items(it, v)?.into_iter().take(n).collect()))
    });
    r.pure(Kind::Iter, "skip", |it, v, a| {
        let n = arg_usize(&a, 0, "skip")?;
        Ok(new_iter(items(it, v)?.into_iter().skip(n).collect()))
    });
    r.pure(Kind::Iter, "step_by", |it, v, a| {
        let n = arg_usize(&a, 0, "step_by")?;
        if n == 0 {
            return Err(panic_ctl("assertion failed: step != 0"));
        }
        Ok(new_iter(items(it, v)?.into_iter().step_by(n).collect()))
    });
    r.pure(Kind::Iter, "take_while", |it, v, a| {
        let f = arg(&a, 0, "take_while")?;
        let mut out = Vec::new();
        for x in items(it, v)? {
            if !truthy(&call(it, &f, vec![x.clone()])?) {
                break;
            }
            out.push(x);
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "skip_while", |it, v, a| {
        let f = arg(&a, 0, "skip_while")?;
        let mut out = Vec::new();
        let mut skipping = true;
        for x in items(it, v)? {
            if skipping && truthy(&call(it, &f, vec![x.clone()])?) {
                continue;
            }
            skipping = false;
            out.push(x);
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "inspect", |it, v, a| {
        let f = arg(&a, 0, "inspect")?;
        let xs = items(it, v)?;
        for x in &xs {
            call(it, &f, vec![x.clone()])?;
        }
        Ok(new_iter(xs))
    });
    r.pure(Kind::Iter, "scan", |it, v, a| {
        let mut state = arg(&a, 0, "scan")?;
        let f = arg(&a, 1, "scan")?;
        let mut out = Vec::new();
        for x in items(it, v)? {
            let cell = Place::temp(state.clone());
            let res = call(it, &f, vec![Value::Ref(cell.clone()), x])?;
            state = cell.read()?;
            match res.as_option() {
                Some(Some(y)) => out.push(y.clone()),
                _ => break,
            }
        }
        Ok(new_iter(out))
    });
    r.pure(Kind::Iter, "for_each", |it, v, a| {
        let f = arg(&a, 0, "for_each")?;
        for x in items(it, v)? {
            call(it, &f, vec![x])?;
        }
        Ok(Value::Unit)
    });
    r.pure(Kind::Iter, "fold", |it, v, a| {
        let mut acc = arg(&a, 0, "fold")?;
        let f = arg(&a, 1, "fold")?;
        for x in items(it, v)? {
            acc = call(it, &f, vec![acc, x])?;
        }
        Ok(acc)
    });
    r.pure(Kind::Iter, "reduce", |it, v, a| {
        let f = arg(&a, 0, "reduce")?;
        let mut xs = items(it, v)?.into_iter();
        let Some(mut acc) = xs.next() else { return Ok(Value::none()) };
        for x in xs {
            acc = call(it, &f, vec![acc, x])?;
        }
        Ok(Value::some(acc))
    });
    r.pure(Kind::Iter, "sum", |it, v, _| {
        let hint = it.type_hint.take();
        let xs = items(it, v)?;
        let mut acc = match (&hint, xs.first()) {
            (Some(h), _) if super::super::value::type_is_float(h).is_some() => {
                Value::Float(0.0, super::super::value::type_is_float(h).unwrap())
            }
            (Some(h), _) if IntTy::from_type(h).is_some() => Value::Int(0, IntTy::from_type(h).unwrap()),
            (_, Some(Value::Float(_, t))) => Value::Float(0.0, *t),
            (_, Some(Value::Int(_, t))) => Value::Int(0, *t),
            _ => Value::int(0),
        };
        for x in xs {
            acc = add_ints(&acc, &x)?;
        }
        Ok(acc)
    });
    r.pure(Kind::Iter, "product", |it, v, _| {
        let hint = it.type_hint.take();
        let xs = items(it, v)?;
        let mut acc = match (&hint, xs.first()) {
            (Some(h), _) if IntTy::from_type(h).is_some() => Value::Int(1, IntTy::from_type(h).unwrap()),
            (_, Some(Value::Float(_, t))) => Value::Float(1.0, *t),
            (_, Some(Value::Int(_, t))) => Value::Int(1, *t),
            _ => Value::int(1),
        };
        for x in xs {
            acc = mul_ints(&acc, &x)?;
        }
        Ok(acc)
    });
    r.pure(Kind::Iter, "count", |it, v, _| Ok(Value::usize(items(it, v)?.len())));
    r.pure(Kind::Iter, "last", |it, v, _| Ok(Value::option(items(it, v)?.pop())));
    r.pure(Kind::Iter, "nth", |it, v, a| {
        let n = arg_usize(&a, 0, "nth")?;
        match v {
            Value::Iter(s) => {
                let mut st = s.borrow_mut();
                st.pos = (st.pos + n).min(st.items.len());
                if st.pos < st.items.len() {
                    let x = st.items[st.pos].clone();
                    st.pos += 1;
                    Ok(Value::some(x))
                } else {
                    Ok(Value::none())
                }
            }
            other => Ok(Value::option(items(it, other)?.into_iter().nth(n))),
        }
    });
    r.pure(Kind::Iter, "min", |it, v, _| {
        let xs = items(it, v)?;
        let mut best: Option<Value> = None;
        for x in xs {
            if best.as_ref().map(|b| x.structural_cmp(b) == Ordering::Less).unwrap_or(true) {
                best = Some(x);
            }
        }
        Ok(Value::option(best))
    });
    r.pure(Kind::Iter, "max", |it, v, _| {
        let xs = items(it, v)?;
        let mut best: Option<Value> = None;
        for x in xs {
            if best.as_ref().map(|b| x.structural_cmp(b) != Ordering::Less).unwrap_or(true) {
                best = Some(x);
            }
        }
        Ok(Value::option(best))
    });
    r.pure(Kind::Iter, "min_by_key", |it, v, a| {
        let f = arg(&a, 0, "min_by_key")?;
        let mut best: Option<(Value, Value)> = None;
        for x in items(it, v)? {
            let k = call(it, &f, vec![x.clone()])?;
            if best.as_ref().map(|(bk, _)| k.structural_cmp(bk) == Ordering::Less).unwrap_or(true) {
                best = Some((k, x));
            }
        }
        Ok(Value::option(best.map(|(_, x)| x)))
    });
    r.pure(Kind::Iter, "max_by_key", |it, v, a| {
        let f = arg(&a, 0, "max_by_key")?;
        let mut best: Option<(Value, Value)> = None;
        for x in items(it, v)? {
            let k = call(it, &f, vec![x.clone()])?;
            if best.as_ref().map(|(bk, _)| k.structural_cmp(bk) != Ordering::Less).unwrap_or(true) {
                best = Some((k, x));
            }
        }
        Ok(Value::option(best.map(|(_, x)| x)))
    });
    r.pure(Kind::Iter, "min_by", |it, v, a| {
        let f = arg(&a, 0, "min_by")?;
        let mut best: Option<Value> = None;
        for x in items(it, v)? {
            let better = match &best {
                None => true,
                Some(b) => ord_from_closure(it, &f, &x, b)? == Ordering::Less,
            };
            if better {
                best = Some(x);
            }
        }
        Ok(Value::option(best))
    });
    r.pure(Kind::Iter, "max_by", |it, v, a| {
        let f = arg(&a, 0, "max_by")?;
        let mut best: Option<Value> = None;
        for x in items(it, v)? {
            let better = match &best {
                None => true,
                Some(b) => ord_from_closure(it, &f, &x, b)? != Ordering::Less,
            };
            if better {
                best = Some(x);
            }
        }
        Ok(Value::option(best))
    });
    r.pure(Kind::Iter, "any", |it, v, a| {
        let f = arg(&a, 0, "any")?;
        for x in items(it, v)? {
            if truthy(&call(it, &f, vec![x])?) {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    });
    r.pure(Kind::Iter, "all", |it, v, a| {
        let f = arg(&a, 0, "all")?;
        for x in items(it, v)? {
            if !truthy(&call(it, &f, vec![x])?) {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    });
    r.pure(Kind::Iter, "find", |it, v, a| {
        let f = arg(&a, 0, "find")?;
        for x in items(it, v)? {
            if truthy(&call(it, &f, vec![x.clone()])?) {
                return Ok(Value::some(x));
            }
        }
        Ok(Value::none())
    });
    r.pure(Kind::Iter, "find_map", |it, v, a| {
        let f = arg(&a, 0, "find_map")?;
        for x in items(it, v)? {
            let r = call(it, &f, vec![x])?;
            if let Some(Some(_)) = r.as_option() {
                return Ok(r);
            }
        }
        Ok(Value::none())
    });
    r.pure(Kind::Iter, "position", |it, v, a| {
        let f = arg(&a, 0, "position")?;
        for (i, x) in items(it, v)?.into_iter().enumerate() {
            if truthy(&call(it, &f, vec![x])?) {
                return Ok(Value::some(Value::usize(i)));
            }
        }
        Ok(Value::none())
    });
    r.pure(Kind::Iter, "rposition", |it, v, a| {
        let f = arg(&a, 0, "rposition")?;
        let xs = items(it, v)?;
        for (i, x) in xs.into_iter().enumerate().rev() {
            if truthy(&call(it, &f, vec![x])?) {
                return Ok(Value::some(Value::usize(i)));
            }
        }
        Ok(Value::none())
    });
    r.pure(Kind::Iter, "unzip", |it, v, _| {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for x in items(it, v)? {
            if let Value::Tuple(t) = x {
                a.push(t[0].clone());
                b.push(t[1].clone());
            }
        }
        Ok(Value::tuple(vec![Value::vec(a), Value::vec(b)]))
    });
    r.pure(Kind::Iter, "partition", |it, v, a| {
        let f = arg(&a, 0, "partition")?;
        let (mut yes, mut no) = (Vec::new(), Vec::new());
        for x in items(it, v)? {
            if truthy(&call(it, &f, vec![x.clone()])?) {
                yes.push(x);
            } else {
                no.push(x);
            }
        }
        Ok(Value::tuple(vec![Value::vec(yes), Value::vec(no)]))
    });
    r.pure(Kind::Iter, "collect", |it, v, _| {
        let hint = it.type_hint.take();
        let xs = items(it, v)?;
        collect_into(xs, hint.as_ref())
    });
    r.pure(Kind::Iter, "cmp", |it, v, a| {
        let other = arg(&a, 0, "cmp")?;
        let (x, y) = (items(it, v)?, items(it, &other)?);
        Ok(ordering(Value::vec(x).structural_cmp(&Value::vec(y))))
    });
    r.pure(Kind::Iter, "eq", |it, v, a| {
        let other = arg(&a, 0, "eq")?;
        let (x, y) = (items(it, v)?, items(it, &other)?);
        Ok(Value::Bool(Value::vec(x).structural_eq(&Value::vec(y))))
    });
    r.pure(Kind::Iter, "contains", |_, v, a| {
        let x = arg(&a, 0, "contains")?;
        match v {
            Value::Range(lo, hi, _) => Ok(Value::Bool(matches!(x, Value::Int(n, _) if n >= *lo && n < *hi))),
            _ => Err(semantic("`contains` on an iterator")),
        }
    });
    r.pure(Kind::Iter, "cycle", |_, _, _| Err(unsupported("`cycle()` (infinite iterators)")));
}

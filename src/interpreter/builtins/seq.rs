//! `Vec` / slice / `VecDeque` methods and the iterator adaptor protocol.
//!
//! The iterator protocol itself (lazy adaptors) lives in `builtins/iter.rs` and `interpreter/iter.rs`.

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

pub(super) fn truthy(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
}

pub(super) fn call(it: &mut Interpreter, f: &Value, args: Vec<Value>) -> R {
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

pub(super) fn ord_from_closure(it: &mut Interpreter, f: &Value, a: &Value, b: &Value) -> R<Ordering> {
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

pub(super) fn add_ints(a: &Value, b: &Value) -> R {
    match (a, b) {
        (Value::Int(x, xt), Value::Int(y, yt)) => crate::interpreter::ops::int_binary(crate::parser::BinaryOp::Add, *x, *xt, *y, *yt),
        (Value::Float(x, xt), Value::Float(y, _)) => Ok(Value::Float(x + y, *xt)),
        (Value::Float(x, xt), Value::Int(y, _)) => Ok(Value::Float(x + *y as f64, *xt)),
        (Value::Int(x, _), Value::Float(y, yt)) => Ok(Value::Float(*x as f64 + y, *yt)),
        _ => Err(semantic("cannot sum these values")),
    }
}

pub(super) fn mul_ints(a: &Value, b: &Value) -> R {
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
            let hashed = name == "HashMap";
            let mut m = std::collections::BTreeMap::new();
            for x in items {
                match x {
                    Value::Tuple(t) if t.len() == 2 => {
                        m.insert(Key(t[0].clone()), t[1].clone());
                    }
                    other => return Err(semantic(format!("cannot collect {} into a map", other.type_name()))),
                }
            }
            Ok(if hashed { Value::hash_map(m) } else { Value::btree_map(m) })
        }
        "HashSet" | "BTreeSet" => {
            let hashed = name == "HashSet";
            let mut m = std::collections::BTreeMap::new();
            for x in items {
                m.insert(Key(x), Value::Unit);
            }
            Ok(if hashed { Value::hash_set(m) } else { Value::btree_set(m) })
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
    super::iter::register(r);
}

fn register_vec(r: &mut Registry) {
    r.pure(Kind::Vec, "len", |_, v, _| Ok(Value::usize(vec_of(v).len())));
    r.pure(Kind::Vec, "is_empty", |_, v, _| Ok(Value::Bool(vec_of(v).is_empty())));
    r.pure(Kind::Vec, "capacity", |_, v, _| Ok(Value::usize(vec_of(v).capacity())));
    r.pure(Kind::Vec, "slice", |it, v, a| {
        let (x, y) = (arg_int(&a, 0, "slice")?, arg_int(&a, 1, "slice")?);
        it.index_value(v, &Value::range(x, y, IntTy::Usize))
    });
    r.pure(Kind::Vec, "iter", |_, v, _| Ok(Value::Iter(IterState::from_rc(vec_of(v)))));
    r.pure(Kind::Vec, "into_iter", |_, v, _| Ok(Value::Iter(IterState::from_rc(vec_of(v)))));
    r.pure(Kind::Vec, "to_vec", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Vec, "as_slice", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Vec, "into_boxed_slice", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Vec, "first", |_, v, _| Ok(Value::option(vec_of(v).first().cloned())));
    r.pure(Kind::Vec, "last", |_, v, _| Ok(Value::option(vec_of(v).last().cloned())));
    r.pure(Kind::Vec, "front", |_, v, _| Ok(Value::option(vec_of(v).first().cloned())));
    r.pure(Kind::Vec, "back", |_, v, _| Ok(Value::option(vec_of(v).last().cloned())));
    r.pure(Kind::Vec, "get", |_, v, a| match arg(&a, 0, "get")? {
        Value::Int(i, _) => Ok(Value::option(usize::try_from(i).ok().and_then(|i| vec_of(v).get(i).cloned()))),
        Value::Range(r, _) => {
            let vv = vec_of(v);
            let (x, y) = (r.0 as usize, r.1 as usize);
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
            Some(Value::Range(r, _)) => (r.0 as usize, r.1 as usize),
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
        Ok(new_iter((0..n).map(|i| Value::Ref(Box::new(p.proj(Proj::Index(i))))).collect()))
    });
    r.place(Kind::Vec, "get_mut", |_, p, a| {
        let i = arg_int(&a, 0, "get_mut")?;
        let n = vec_of(&p.read()?).len() as i128;
        Ok(if i >= 0 && i < n { Value::some(Value::Ref(Box::new(p.proj(Proj::Index(i as usize))))) } else { Value::none() })
    });
    r.place(Kind::Vec, "first_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(Box::new(p.proj(Proj::Index(0))))) } else { Value::none() })
    });
    r.place(Kind::Vec, "last_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(Box::new(p.proj(Proj::Index(n - 1))))) } else { Value::none() })
    });
    r.place(Kind::Vec, "front_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(Box::new(p.proj(Proj::Index(0))))) } else { Value::none() })
    });
    r.place(Kind::Vec, "back_mut", |_, p, _| {
        let n = vec_of(&p.read()?).len();
        Ok(if n > 0 { Value::some(Value::Ref(Box::new(p.proj(Proj::Index(n - 1))))) } else { Value::none() })
    });
}

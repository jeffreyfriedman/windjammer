//! Iterator methods: lazy adaptors and consumers over the protocol in `interpreter/iter.rs`.

use super::seq::{add_ints, call, collect_into, mul_ints, ord_from_closure, truthy};
use super::*;
use crate::interpreter::iter::{self as proto, IterSrc};
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;

fn it_of(it: &mut Interpreter, v: &Value) -> R<Iter> {
    proto::as_iter(it, v.clone())
}

fn adapt(src: IterSrc) -> Value {
    Value::Iter(IterState::new(src))
}

/// Feed every remaining item to `f` until it returns `Ok(false)`.
fn each(it: &mut Interpreter, v: &Value, mut f: impl FnMut(&mut Interpreter, Value) -> R<bool>) -> R<()> {
    let s = it_of(it, v)?;
    while let Some(x) = proto::next(it, &s)? {
        if !f(it, x)? {
            break;
        }
    }
    Ok(())
}

fn drain(it: &mut Interpreter, v: &Value) -> R<Vec<Value>> {
    let s = it_of(it, v)?;
    it.drain_iter(&s)
}

fn require_iter(v: &Value, what: &str) -> R<Iter> {
    proto::expect_iter(v, what)
}

pub fn register(r: &mut Registry) {
    register_sources(r);
    register_adaptors(r);
    register_consumers(r);
    register_search(r);
}

fn register_sources(r: &mut Registry) {
    r.func("iter::repeat", |_, a| Ok(adapt(IterSrc::Repeat(arg(&a, 0, "repeat")?))));
    r.func("iter::repeat_with", |_, a| {
        // `repeat_with(f)` is `from_fn(|| Some(f()))`: an endless stream of `f()` calls
        Ok(adapt(IterSrc::RepeatWith(arg(&a, 0, "repeat_with")?)))
    });
    r.func("iter::from_fn", |_, a| Ok(adapt(IterSrc::FromFn(arg(&a, 0, "from_fn")?))));
    r.func("iter::successors", |_, a| {
        let first = arg(&a, 0, "successors")?;
        let f = arg(&a, 1, "successors")?;
        Ok(adapt(IterSrc::Successors { next: RefCell::new(first.as_option().and_then(|o| o.cloned())), f }))
    });
    r.func("iter::once", |_, a| Ok(new_iter(vec![arg(&a, 0, "once")?])));
    r.func("iter::empty", |_, _| Ok(new_iter(vec![])));
}

fn register_adaptors(r: &mut Registry) {
    for name in ["iter", "into_iter", "peekable", "by_ref", "cloned", "copied", "fuse"] {
        r.pure(Kind::Iter, name, |it, v, _| Ok(Value::Iter(it_of(it, v)?)));
    }
    r.pure(Kind::Iter, "clone", |_, v, _| match v {
        Value::Iter(s) => Ok(Value::Iter(proto::clone_iter(s))),
        // collections and ranges also reach this registry; they clone by value
        other => Ok(other.clone()),
    });
    r.pure(Kind::Iter, "map", |it, v, a| Ok(adapt(IterSrc::Map(it_of(it, v)?, arg(&a, 0, "map")?))));
    r.pure(Kind::Iter, "filter", |it, v, a| Ok(adapt(IterSrc::Filter(it_of(it, v)?, arg(&a, 0, "filter")?))));
    r.pure(Kind::Iter, "filter_map", |it, v, a| Ok(adapt(IterSrc::FilterMap(it_of(it, v)?, arg(&a, 0, "filter_map")?))));
    r.pure(Kind::Iter, "flat_map", |it, v, a| {
        Ok(adapt(IterSrc::FlatMap { inner: it_of(it, v)?, f: Some(arg(&a, 0, "flat_map")?), cur: RefCell::new(None) }))
    });
    r.pure(Kind::Iter, "flatten", |it, v, _| {
        Ok(adapt(IterSrc::FlatMap { inner: it_of(it, v)?, f: None, cur: RefCell::new(None) }))
    });
    r.pure(Kind::Iter, "enumerate", |it, v, _| Ok(adapt(IterSrc::Enumerate(it_of(it, v)?, Cell::new(0)))));
    r.pure(Kind::Iter, "zip", |it, v, a| {
        let other = arg(&a, 0, "zip")?;
        Ok(adapt(IterSrc::Zip(it_of(it, v)?, it_of(it, &other)?)))
    });
    r.pure(Kind::Iter, "chain", |it, v, a| {
        let other = arg(&a, 0, "chain")?;
        Ok(adapt(IterSrc::Chain(it_of(it, v)?, it_of(it, &other)?, Cell::new(false))))
    });
    r.pure(Kind::Iter, "rev", |it, v, _| {
        let s = it_of(it, v)?;
        if proto::double_ended(&s) {
            return Ok(adapt(IterSrc::Rev(s)));
        }
        let mut items = it.drain_iter(&s)?;
        items.reverse();
        Ok(new_iter(items))
    });
    r.pure(Kind::Iter, "take", |it, v, a| Ok(adapt(IterSrc::Take(it_of(it, v)?, Cell::new(arg_usize(&a, 0, "take")?)))));
    r.pure(Kind::Iter, "skip", |it, v, a| Ok(adapt(IterSrc::Skip(it_of(it, v)?, Cell::new(arg_usize(&a, 0, "skip")?)))));
    r.pure(Kind::Iter, "step_by", |it, v, a| {
        let n = arg_usize(&a, 0, "step_by")?;
        if n == 0 {
            return Err(panic_ctl("assertion failed: step != 0"));
        }
        Ok(adapt(IterSrc::StepBy(it_of(it, v)?, n, Cell::new(false))))
    });
    r.pure(Kind::Iter, "take_while", |it, v, a| {
        Ok(adapt(IterSrc::TakeWhile(it_of(it, v)?, arg(&a, 0, "take_while")?, Cell::new(false))))
    });
    r.pure(Kind::Iter, "skip_while", |it, v, a| {
        Ok(adapt(IterSrc::SkipWhile(it_of(it, v)?, arg(&a, 0, "skip_while")?, Cell::new(true))))
    });
    r.pure(Kind::Iter, "inspect", |it, v, a| Ok(adapt(IterSrc::Inspect(it_of(it, v)?, arg(&a, 0, "inspect")?))));
    r.pure(Kind::Iter, "scan", |it, v, a| {
        let state = Place::temp(arg(&a, 0, "scan")?);
        Ok(adapt(IterSrc::Scan { inner: it_of(it, v)?, state, f: arg(&a, 1, "scan")?, done: Cell::new(false) }))
    });
    r.pure(Kind::Iter, "cycle", |it, v, _| {
        Ok(adapt(IterSrc::Cycle { inner: it_of(it, v)?, buf: RefCell::new(Vec::new()), idx: Cell::new(0), replaying: Cell::new(false) }))
    });
}

fn register_consumers(r: &mut Registry) {
    r.pure(Kind::Iter, "len", |it, v, _| {
        let s = it_of(it, v)?;
        proto::exact_len(&s).map(Value::usize).ok_or_else(|| unsupported("`len()` on an iterator without a known exact size"))
    });
    r.pure(Kind::Iter, "is_empty", |it, v, _| {
        let s = it_of(it, v)?;
        match proto::exact_len(&s) {
            Some(n) => Ok(Value::Bool(n == 0)),
            None => Err(unsupported("`is_empty()` on an iterator without a known exact size")),
        }
    });
    r.pure(Kind::Iter, "next", |it, v, _| {
        let s = require_iter(v, "next")?;
        Ok(Value::option(proto::next(it, &s)?))
    });
    r.pure(Kind::Iter, "next_back", |it, v, _| {
        let s = require_iter(v, "next_back")?;
        Ok(Value::option(proto::next_back(it, &s)?))
    });
    r.pure(Kind::Iter, "peek", |it, v, _| {
        let s = require_iter(v, "peek")?;
        Ok(Value::option(proto::peek(it, &s)?))
    });
    r.pure(Kind::Iter, "next_if", |it, v, a| {
        let s = require_iter(v, "next_if")?;
        let f = arg(&a, 0, "next_if")?;
        match proto::peek(it, &s)? {
            Some(x) if truthy(&call(it, &f, vec![x.clone()])?) => {
                proto::next(it, &s)?;
                Ok(Value::some(x))
            }
            _ => Ok(Value::none()),
        }
    });
    r.pure(Kind::Iter, "nth", |it, v, a| {
        let n = arg_usize(&a, 0, "nth")?;
        let s = it_of(it, v)?;
        for _ in 0..n {
            if proto::next(it, &s)?.is_none() {
                return Ok(Value::none());
            }
        }
        Ok(Value::option(proto::next(it, &s)?))
    });
    r.pure(Kind::Iter, "for_each", |it, v, a| {
        let f = arg(&a, 0, "for_each")?;
        each(it, v, |it, x| call(it, &f, vec![x]).map(|_| true))?;
        Ok(Value::Unit)
    });
    r.pure(Kind::Iter, "fold", |it, v, a| {
        let mut acc = arg(&a, 0, "fold")?;
        let f = arg(&a, 1, "fold")?;
        each(it, v, |it, x| {
            acc = call(it, &f, vec![std::mem::replace(&mut acc, Value::Unit), x])?;
            Ok(true)
        })?;
        Ok(acc)
    });
    r.pure(Kind::Iter, "reduce", |it, v, a| {
        let f = arg(&a, 0, "reduce")?;
        let mut acc: Option<Value> = None;
        each(it, v, |it, x| {
            acc = Some(match acc.take() {
                None => x,
                Some(p) => call(it, &f, vec![p, x])?,
            });
            Ok(true)
        })?;
        Ok(Value::option(acc))
    });
    r.pure(Kind::Iter, "sum", |it, v, _| {
        let hint = it.type_hint.take();
        let xs = drain(it, v)?;
        let mut acc = match (&hint, xs.first()) {
            (Some(h), _) if type_is_float(h).is_some() => Value::Float(0.0, type_is_float(h).unwrap()),
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
        let xs = drain(it, v)?;
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
    r.pure(Kind::Iter, "count", |it, v, _| Ok(Value::usize(drain(it, v)?.len())));
    r.pure(Kind::Iter, "last", |it, v, _| Ok(Value::option(drain(it, v)?.pop())));
    r.pure(Kind::Iter, "min", |it, v, _| extreme(it, v, Ordering::Less));
    r.pure(Kind::Iter, "max", |it, v, _| extreme(it, v, Ordering::Greater));
    r.pure(Kind::Iter, "min_by_key", |it, v, a| extreme_by_key(it, v, &a, "min_by_key", true));
    r.pure(Kind::Iter, "max_by_key", |it, v, a| extreme_by_key(it, v, &a, "max_by_key", false));
    r.pure(Kind::Iter, "min_by", |it, v, a| extreme_by(it, v, &a, "min_by", true));
    r.pure(Kind::Iter, "max_by", |it, v, a| extreme_by(it, v, &a, "max_by", false));
    r.pure(Kind::Iter, "unzip", |it, v, _| {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for x in drain(it, v)? {
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
        each(it, v, |it, x| {
            if truthy(&call(it, &f, vec![x.clone()])?) {
                yes.push(x);
            } else {
                no.push(x);
            }
            Ok(true)
        })?;
        Ok(Value::tuple(vec![Value::vec(yes), Value::vec(no)]))
    });
    r.pure(Kind::Iter, "collect", |it, v, _| {
        let hint = it.type_hint.take();
        let xs = drain(it, v)?;
        collect_into(xs, hint.as_ref())
    });
    r.pure(Kind::Iter, "cmp", |it, v, a| {
        let other = arg(&a, 0, "cmp")?;
        let (x, y) = (drain(it, v)?, drain(it, &other)?);
        Ok(ordering(Value::vec(x).structural_cmp(&Value::vec(y))))
    });
    r.pure(Kind::Iter, "eq", |it, v, a| {
        let other = arg(&a, 0, "eq")?;
        let (x, y) = (drain(it, v)?, drain(it, &other)?);
        Ok(Value::Bool(Value::vec(x).structural_eq(&Value::vec(y))))
    });
    r.pure(Kind::Iter, "contains", |_, v, a| {
        let x = arg(&a, 0, "contains")?;
        match v {
            Value::Range(r, _) => Ok(Value::Bool(matches!(x, Value::Int(n, _) if n >= r.0 && n < r.1))),
            _ => Err(semantic("`contains` on an iterator")),
        }
    });
}

fn extreme(it: &mut Interpreter, v: &Value, want: Ordering) -> R {
    let mut best: Option<Value> = None;
    for x in drain(it, v)? {
        let better = match &best {
            None => true,
            // `min` keeps the first of equals, `max` the last (as Rust does)
            Some(b) => match want {
                Ordering::Less => x.structural_cmp(b) == Ordering::Less,
                _ => x.structural_cmp(b) != Ordering::Less,
            },
        };
        if better {
            best = Some(x);
        }
    }
    Ok(Value::option(best))
}

fn extreme_by_key(it: &mut Interpreter, v: &Value, a: &[Value], what: &str, min: bool) -> R {
    let f = arg(a, 0, what)?;
    let mut best: Option<(Value, Value)> = None;
    each(it, v, |it, x| {
        let k = call(it, &f, vec![x.clone()])?;
        let better = match &best {
            None => true,
            Some((bk, _)) if min => k.structural_cmp(bk) == Ordering::Less,
            Some((bk, _)) => k.structural_cmp(bk) != Ordering::Less,
        };
        if better {
            best = Some((k, x));
        }
        Ok(true)
    })?;
    Ok(Value::option(best.map(|(_, x)| x)))
}

fn extreme_by(it: &mut Interpreter, v: &Value, a: &[Value], what: &str, min: bool) -> R {
    let f = arg(a, 0, what)?;
    let mut best: Option<Value> = None;
    each(it, v, |it, x| {
        let better = match &best {
            None => true,
            Some(b) if min => ord_from_closure(it, &f, &x, b)? == Ordering::Less,
            Some(b) => ord_from_closure(it, &f, &x, b)? != Ordering::Less,
        };
        if better {
            best = Some(x);
        }
        Ok(true)
    })?;
    Ok(Value::option(best))
}

fn register_search(r: &mut Registry) {
    r.pure(Kind::Iter, "any", |it, v, a| {
        let f = arg(&a, 0, "any")?;
        let mut hit = false;
        each(it, v, |it, x| {
            hit = truthy(&call(it, &f, vec![x])?);
            Ok(!hit)
        })?;
        Ok(Value::Bool(hit))
    });
    r.pure(Kind::Iter, "all", |it, v, a| {
        let f = arg(&a, 0, "all")?;
        let mut all = true;
        each(it, v, |it, x| {
            all = truthy(&call(it, &f, vec![x])?);
            Ok(all)
        })?;
        Ok(Value::Bool(all))
    });
    r.pure(Kind::Iter, "find", |it, v, a| {
        let f = arg(&a, 0, "find")?;
        let mut found = None;
        each(it, v, |it, x| {
            if truthy(&call(it, &f, vec![x.clone()])?) {
                found = Some(x);
                return Ok(false);
            }
            Ok(true)
        })?;
        Ok(Value::option(found))
    });
    r.pure(Kind::Iter, "find_map", |it, v, a| {
        let f = arg(&a, 0, "find_map")?;
        let mut found = None;
        each(it, v, |it, x| {
            let res = call(it, &f, vec![x])?;
            if let Some(Some(_)) = res.as_option() {
                found = Some(res);
                return Ok(false);
            }
            Ok(true)
        })?;
        Ok(found.unwrap_or_else(Value::none))
    });
    r.pure(Kind::Iter, "position", |it, v, a| {
        let f = arg(&a, 0, "position")?;
        let mut i = 0usize;
        let mut found = None;
        each(it, v, |it, x| {
            if truthy(&call(it, &f, vec![x])?) {
                found = Some(Value::usize(i));
                return Ok(false);
            }
            i += 1;
            Ok(true)
        })?;
        Ok(Value::option(found))
    });
    r.pure(Kind::Iter, "rposition", |it, v, a| {
        let f = arg(&a, 0, "rposition")?;
        let xs = drain(it, v)?;
        for (i, x) in xs.into_iter().enumerate().rev() {
            if truthy(&call(it, &f, vec![x])?) {
                return Ok(Value::some(Value::usize(i)));
            }
        }
        Ok(Value::none())
    });
}

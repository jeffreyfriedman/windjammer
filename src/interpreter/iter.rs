//! Lazy iterator protocol.
//!
//! An iterator is a tree of [`IterSrc`] nodes pulled one item at a time ([`next`]), exactly as in
//! Rust: adaptors do no work until a consumer asks, closures run in the order the compiled program
//! would run them, and infinite sources (`cycle`, unbounded ranges, `repeat`, `from_fn`) are fine
//! as long as something downstream (`take`, `take_while`, `find`, `break`...) stops.
//! All mutable state is in `Cell`/`RefCell` so a node can be pulled through a shared `Rc`.

use super::engine::Interpreter;
use super::error::{semantic, unsupported, R};
use super::int::IntTy;
use super::place::deref_value;
use super::value::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub type Iter = Rc<IterState>;

/// Materialising more items than this from one iterator is reported instead of looping forever.
pub const MATERIALISE_LIMIT: usize = 50_000_000;

pub struct IterState {
    pub src: IterSrc,
    /// One-item lookahead for `peek`: `Some(None)` = peeked and exhausted.
    pub peeked: RefCell<Option<Option<Value>>>,
}

pub enum IterSrc {
    Buf { items: Rc<Vec<Value>>, pos: Cell<usize>, end: Cell<usize> },
    /// Half-open; `end == i128::MAX` is an unbounded range (`1..`).
    Range { cur: Cell<i128>, end: Cell<i128>, ty: IntTy },
    Map(Iter, Value),
    Filter(Iter, Value),
    FilterMap(Iter, Value),
    /// `flat_map` (with a function) and `flatten` (without).
    FlatMap { inner: Iter, f: Option<Value>, cur: RefCell<Option<Iter>> },
    Enumerate(Iter, Cell<usize>),
    Zip(Iter, Iter),
    Chain(Iter, Iter, Cell<bool>),
    Take(Iter, Cell<usize>),
    Skip(Iter, Cell<usize>),
    StepBy(Iter, usize, Cell<bool>),
    TakeWhile(Iter, Value, Cell<bool>),
    SkipWhile(Iter, Value, Cell<bool>),
    Inspect(Iter, Value),
    Scan { inner: Iter, state: Place, f: Value, done: Cell<bool> },
    Cycle { inner: Iter, buf: RefCell<Vec<Value>>, idx: Cell<usize>, replaying: Cell<bool> },
    Rev(Iter),
    Repeat(Value),
    RepeatWith(Value),
    /// Messages from a channel receiver (blocking, or only what is queued when `true`).
    Recv(Rc<super::builtins::channel::Chan>, bool),
    /// `from_fn(f)`: `f()` returns `Option`.
    FromFn(Value),
    /// `successors(first, f)`: `f(&prev)` returns `Option`.
    Successors { next: RefCell<Option<Value>>, f: Value },
}

impl IterState {
    pub fn new(src: IterSrc) -> Iter {
        Rc::new(IterState { src, peeked: RefCell::new(None) })
    }
    pub fn from_items(items: Vec<Value>) -> Iter {
        let n = items.len();
        Self::new(IterSrc::Buf { items: Rc::new(items), pos: Cell::new(0), end: Cell::new(n) })
    }
    pub fn from_rc(items: Rc<Vec<Value>>) -> Iter {
        let n = items.len();
        Self::new(IterSrc::Buf { items, pos: Cell::new(0), end: Cell::new(n) })
    }
}

/// Anything iterable as an iterator (vectors share their storage; ranges stay lazy).
pub fn as_iter(it: &mut Interpreter, v: Value) -> R<Iter> {
    Ok(match deref_value(v)? {
        Value::Iter(i) => i,
        Value::Vec(x) => IterState::from_rc(x),
        Value::Range(r, ty) => IterState::new(IterSrc::Range { cur: Cell::new(r.0), end: Cell::new(r.1), ty }),
        Value::Native(ref n) if n.type_name() == "Receiver" => match super::builtins::channel::receiver_iter(&Value::Native(n.clone()), false) {
            Some(Value::Iter(i)) => i,
            _ => return Err(semantic("not a receiver")),
        },
        Value::Native(ref n) if n.iter_items().is_some() => IterState::from_items(n.iter_items().unwrap_or_default()),
        other => IterState::from_items(it.to_items(other)?),
    })
}

fn truthy(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
}

/// Pull the next item.
pub fn next(it: &mut Interpreter, s: &Iter) -> R<Option<Value>> {
    if let Some(p) = s.peeked.borrow_mut().take() {
        return Ok(p);
    }
    advance(it, s)
}

pub fn peek(it: &mut Interpreter, s: &Iter) -> R<Option<Value>> {
    if let Some(p) = &*s.peeked.borrow() {
        return Ok(p.clone());
    }
    let v = advance(it, s)?;
    *s.peeked.borrow_mut() = Some(v.clone());
    Ok(v)
}

fn advance(it: &mut Interpreter, s: &Iter) -> R<Option<Value>> {
    match &s.src {
        IterSrc::Buf { items, pos, end } => {
            let p = pos.get();
            if p < end.get() {
                pos.set(p + 1);
                Ok(Some(items[p].clone()))
            } else {
                Ok(None)
            }
        }
        IterSrc::Range { cur, end, ty } => {
            let c = cur.get();
            if c < end.get() {
                cur.set(c + 1);
                Ok(Some(Value::Int(c, *ty)))
            } else {
                Ok(None)
            }
        }
        IterSrc::Map(inner, f) => match next(it, inner)? {
            Some(x) => Ok(Some(it.call_value(f, vec![x])?)),
            None => Ok(None),
        },
        IterSrc::Filter(inner, f) => loop {
            match next(it, inner)? {
                None => return Ok(None),
                Some(x) => {
                    if truthy(&it.call_value(f, vec![x.clone()])?) {
                        return Ok(Some(x));
                    }
                }
            }
        },
        IterSrc::FilterMap(inner, f) => loop {
            match next(it, inner)? {
                None => return Ok(None),
                Some(x) => {
                    if let Some(Some(y)) = it.call_value(f, vec![x])?.as_option() {
                        return Ok(Some(y.clone()));
                    }
                }
            }
        },
        IterSrc::FlatMap { inner, f, cur } => loop {
            let sub = cur.borrow().clone();
            if let Some(c) = sub {
                match next(it, &c)? {
                    Some(x) => return Ok(Some(x)),
                    None => *cur.borrow_mut() = None,
                }
            }
            match next(it, inner)? {
                None => return Ok(None),
                Some(x) => {
                    let produced = match f {
                        Some(f) => it.call_value(f, vec![x])?,
                        None => x,
                    };
                    *cur.borrow_mut() = Some(as_iter(it, produced)?);
                }
            }
        },
        IterSrc::Enumerate(inner, n) => match next(it, inner)? {
            Some(x) => {
                let i = n.get();
                n.set(i + 1);
                Ok(Some(Value::tuple(vec![Value::usize(i), x])))
            }
            None => Ok(None),
        },
        IterSrc::Zip(a, b) => match next(it, a)? {
            None => Ok(None),
            Some(x) => match next(it, b)? {
                None => Ok(None),
                Some(y) => Ok(Some(Value::tuple(vec![x, y]))),
            },
        },
        IterSrc::Chain(a, b, first_done) => {
            if !first_done.get() {
                match next(it, a)? {
                    Some(x) => return Ok(Some(x)),
                    None => first_done.set(true),
                }
            }
            next(it, b)
        }
        IterSrc::Take(inner, n) => {
            if n.get() == 0 {
                return Ok(None);
            }
            n.set(n.get() - 1);
            next(it, inner)
        }
        IterSrc::Skip(inner, n) => {
            while n.get() > 0 {
                n.set(n.get() - 1);
                if next(it, inner)?.is_none() {
                    return Ok(None);
                }
            }
            next(it, inner)
        }
        IterSrc::StepBy(inner, step, started) => {
            if started.get() {
                for _ in 1..*step {
                    if next(it, inner)?.is_none() {
                        return Ok(None);
                    }
                }
            }
            started.set(true);
            next(it, inner)
        }
        IterSrc::TakeWhile(inner, f, done) => {
            if done.get() {
                return Ok(None);
            }
            match next(it, inner)? {
                None => Ok(None),
                Some(x) => {
                    if truthy(&it.call_value(f, vec![x.clone()])?) {
                        Ok(Some(x))
                    } else {
                        done.set(true);
                        Ok(None)
                    }
                }
            }
        }
        IterSrc::SkipWhile(inner, f, skipping) => loop {
            match next(it, inner)? {
                None => return Ok(None),
                Some(x) => {
                    if skipping.get() && truthy(&it.call_value(f, vec![x.clone()])?) {
                        continue;
                    }
                    skipping.set(false);
                    return Ok(Some(x));
                }
            }
        },
        IterSrc::Inspect(inner, f) => match next(it, inner)? {
            Some(x) => {
                it.call_value(f, vec![x.clone()])?;
                Ok(Some(x))
            }
            None => Ok(None),
        },
        IterSrc::Scan { inner, state, f, done } => {
            if done.get() {
                return Ok(None);
            }
            match next(it, inner)? {
                None => Ok(None),
                Some(x) => {
                    let res = it.call_value(f, vec![Value::Ref(Box::new(state.clone())), x])?;
                    match res.as_option() {
                        Some(Some(y)) => Ok(Some(y.clone())),
                        _ => {
                            done.set(true);
                            Ok(None)
                        }
                    }
                }
            }
        }
        IterSrc::Cycle { inner, buf, idx, replaying } => {
            if !replaying.get() {
                match next(it, inner)? {
                    Some(x) => {
                        buf.borrow_mut().push(x.clone());
                        return Ok(Some(x));
                    }
                    None => {
                        if buf.borrow().is_empty() {
                            return Ok(None);
                        }
                        replaying.set(true);
                        idx.set(0);
                    }
                }
            }
            let b = buf.borrow();
            let i = idx.get();
            idx.set((i + 1) % b.len());
            Ok(Some(b[i].clone()))
        }
        IterSrc::Rev(inner) => next_back(it, inner),
        IterSrc::Recv(chan, try_only) => it.chan_recv_item(chan, *try_only),
        IterSrc::Repeat(v) => Ok(Some(v.clone())),
        IterSrc::RepeatWith(f) => Ok(Some(it.call_value(f, vec![])?)),
        IterSrc::FromFn(f) => match it.call_value(f, vec![])?.as_option() {
            Some(Some(x)) => Ok(Some(x.clone())),
            _ => Ok(None),
        },
        IterSrc::Successors { next: slot, f } => {
            let cur = slot.borrow_mut().take();
            match cur {
                None => Ok(None),
                Some(v) => {
                    if let Some(Some(n)) = it.call_value(f, vec![v.clone()])?.as_option() {
                        *slot.borrow_mut() = Some(n.clone());
                    }
                    Ok(Some(v))
                }
            }
        }
    }
}

/// Can `next_back` be answered without materialising?
pub fn double_ended(s: &Iter) -> bool {
    match &s.src {
        IterSrc::Buf { .. } | IterSrc::Range { .. } => true,
        IterSrc::Map(i, _) | IterSrc::Filter(i, _) | IterSrc::Inspect(i, _) | IterSrc::Rev(i) => double_ended(i),
        IterSrc::Chain(a, b, _) => double_ended(a) && double_ended(b),
        _ => false,
    }
}

pub fn next_back(it: &mut Interpreter, s: &Iter) -> R<Option<Value>> {
    match &s.src {
        IterSrc::Buf { items, pos, end } => {
            if pos.get() < end.get() {
                end.set(end.get() - 1);
                Ok(Some(items[end.get()].clone()))
            } else {
                Ok(None)
            }
        }
        IterSrc::Range { cur, end, ty } => {
            if cur.get() < end.get() {
                end.set(end.get() - 1);
                Ok(Some(Value::Int(end.get(), *ty)))
            } else {
                Ok(None)
            }
        }
        IterSrc::Map(inner, f) => match next_back(it, inner)? {
            Some(x) => Ok(Some(it.call_value(f, vec![x])?)),
            None => Ok(None),
        },
        IterSrc::Filter(inner, f) => loop {
            match next_back(it, inner)? {
                None => return Ok(None),
                Some(x) => {
                    if truthy(&it.call_value(f, vec![x.clone()])?) {
                        return Ok(Some(x));
                    }
                }
            }
        },
        IterSrc::Inspect(inner, f) => match next_back(it, inner)? {
            Some(x) => {
                it.call_value(f, vec![x.clone()])?;
                Ok(Some(x))
            }
            None => Ok(None),
        },
        IterSrc::Rev(inner) => next(it, inner),
        IterSrc::Chain(a, b, _) => match next_back(it, b)? {
            Some(x) => Ok(Some(x)),
            None => next_back(it, a),
        },
        _ => Err(unsupported("`next_back`/`rev` on this iterator adaptor")),
    }
}

/// Remaining length for exact-size iterators (`len()`), `None` when Rust would not offer `len`.
pub fn exact_len(s: &Iter) -> Option<usize> {
    let peeked = match &*s.peeked.borrow() {
        Some(Some(_)) => 1,
        _ => 0,
    };
    let n = match &s.src {
        IterSrc::Buf { pos, end, .. } => end.get() - pos.get(),
        IterSrc::Range { cur, end, .. } => {
            if end.get() == i128::MAX {
                return None;
            }
            (end.get() - cur.get()).max(0) as usize
        }
        IterSrc::Map(i, _) | IterSrc::Inspect(i, _) | IterSrc::Rev(i) | IterSrc::Enumerate(i, _) => exact_len(i)?,
        IterSrc::Take(i, n) => exact_len(i)?.min(n.get()),
        IterSrc::Skip(i, n) => exact_len(i)?.saturating_sub(n.get()),
        IterSrc::Zip(a, b) => exact_len(a)?.min(exact_len(b)?),
        IterSrc::Chain(a, b, _) => exact_len(a)? + exact_len(b)?,
        _ => return None,
    };
    Some(n + peeked)
}

/// An independent copy of the iterator's current state (`iter.clone()`).
pub fn clone_iter(s: &Iter) -> Iter {
    use IterSrc::*;
    let c = clone_iter;
    let src = match &s.src {
        Buf { items, pos, end } => Buf { items: items.clone(), pos: Cell::new(pos.get()), end: Cell::new(end.get()) },
        Range { cur, end, ty } => Range { cur: Cell::new(cur.get()), end: Cell::new(end.get()), ty: *ty },
        Map(i, f) => Map(c(i), f.clone()),
        Filter(i, f) => Filter(c(i), f.clone()),
        FilterMap(i, f) => FilterMap(c(i), f.clone()),
        FlatMap { inner, f, cur } => FlatMap { inner: c(inner), f: f.clone(), cur: RefCell::new(cur.borrow().as_ref().map(c)) },
        Enumerate(i, n) => Enumerate(c(i), Cell::new(n.get())),
        Zip(a, b) => Zip(c(a), c(b)),
        Chain(a, b, d) => Chain(c(a), c(b), Cell::new(d.get())),
        Take(i, n) => Take(c(i), Cell::new(n.get())),
        Skip(i, n) => Skip(c(i), Cell::new(n.get())),
        StepBy(i, n, st) => StepBy(c(i), *n, Cell::new(st.get())),
        TakeWhile(i, f, d) => TakeWhile(c(i), f.clone(), Cell::new(d.get())),
        SkipWhile(i, f, d) => SkipWhile(c(i), f.clone(), Cell::new(d.get())),
        Inspect(i, f) => Inspect(c(i), f.clone()),
        Scan { inner, state, f, done } => {
            Scan { inner: c(inner), state: Place::temp(state.read().unwrap_or(Value::Unit)), f: f.clone(), done: Cell::new(done.get()) }
        }
        Cycle { inner, buf, idx, replaying } => {
            Cycle { inner: c(inner), buf: RefCell::new(buf.borrow().clone()), idx: Cell::new(idx.get()), replaying: Cell::new(replaying.get()) }
        }
        Rev(i) => Rev(c(i)),
        Recv(ch, t) => Recv(ch.clone(), *t),
        Repeat(v) => Repeat(v.clone()),
        RepeatWith(f) => RepeatWith(f.clone()),
        FromFn(f) => FromFn(f.clone()),
        Successors { next, f } => Successors { next: RefCell::new(next.borrow().clone()), f: f.clone() },
    };
    Rc::new(IterState { src, peeked: RefCell::new(s.peeked.borrow().clone()) })
}

impl Interpreter {
    /// Drain an iterator into a vector, refusing to run forever.
    pub(crate) fn drain_iter(&mut self, s: &Iter) -> R<Vec<Value>> {
        let mut out = Vec::new();
        while let Some(x) = next(self, s)? {
            out.push(x);
            if out.len() > MATERIALISE_LIMIT {
                return Err(unsupported(
                    "collecting an iterator that does not end within 50M items (put `take`/`take_while` before the consumer)",
                ));
            }
        }
        Ok(out)
    }
}

pub fn expect_iter(v: &Value, what: &str) -> R<Iter> {
    match v {
        Value::Iter(i) => Ok(i.clone()),
        _ => Err(semantic(format!("`{}` needs an iterator; call `.iter()` first", what))),
    }
}

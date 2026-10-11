//! `Condvar`, `Barrier`, `Once`-free primitives and atomics.

use super::channel::raw_arg;
use super::*;
use crate::interpreter::locks::CellState;
use crate::interpreter::native::{downcast, Native};
use crate::interpreter::sched::Wait;
use std::any::Any;
use std::cell::Cell;

#[derive(Default)]
pub struct CondState {
    next_ticket: Cell<u64>,
    wake_upto: Cell<u64>,
}

impl CondState {
    pub fn woken(&self, ticket: u64) -> bool {
        ticket < self.wake_upto.get()
    }
    fn take_ticket(&self) -> u64 {
        let t = self.next_ticket.get();
        self.next_ticket.set(t + 1);
        t
    }
    fn notify_one(&self) {
        if self.wake_upto.get() < self.next_ticket.get() {
            self.wake_upto.set(self.wake_upto.get() + 1);
        }
    }
    fn notify_all(&self) {
        self.wake_upto.set(self.next_ticket.get());
    }
}

pub struct BarrierState {
    n: usize,
    count: Cell<usize>,
    gen: Cell<u64>,
}

impl BarrierState {
    pub fn generation(&self) -> u64 {
        self.gen.get()
    }
}

struct CondvarObj(Rc<CondState>);
struct BarrierObj(Rc<BarrierState>);
struct Flag(&'static str, bool);

macro_rules! plain_native {
    ($t:ty, $name:literal) => {
        impl Native for $t {
            fn type_name(&self) -> &'static str {
                $name
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}
plain_native!(CondvarObj, "Condvar");
plain_native!(BarrierObj, "Barrier");
impl Native for Flag {
    fn type_name(&self) -> &'static str {
        self.0
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn debug(&self) -> String {
        format!("{}({})", self.0, self.1)
    }
}

fn guard_parts(v: &Value) -> R<(Place, Rc<crate::interpreter::locks::GuardToken>)> {
    match v {
        Value::Ref(p) => match &p.guard {
            Some(t) => Ok((p.clone(), t.clone())),
            None => Err(semantic("Condvar::wait needs a MutexGuard")),
        },
        _ => Err(semantic("Condvar::wait needs a MutexGuard")),
    }
}

/// Release the lock behind `guard`, wait for a notification (or the deadline), then re-take it.
fn cond_wait(it: &mut Interpreter, cv: &Rc<CondState>, guard: &Value, timeout_ns: Option<i128>) -> R<bool> {
    let (_, tok) = guard_parts(guard)?;
    let ticket = cv.take_ticket();
    tok.released.set(true);
    let deadline = timeout_ns.map(|n| it.sched.clock.saturating_add(n.max(0) as u64));
    let r = it.block_on(Wait::Cond(cv.clone(), ticket, deadline));
    let timed_out = !cv.woken(ticket);
    // `released` guards were skipped while waiting, so waiting for the lock never blocks on ourselves
    let state = tok.state();
    let r2 = match state {
        Some(st) => it.block_on(Wait::Lock(st, tok.exclusive)),
        None => Ok(()),
    };
    tok.released.set(false);
    r.and(r2)?;
    Ok(timed_out)
}

macro_rules! atomic_ctor {
    ($r:expr, $name:literal, $ty:expr) => {
        $r.func(concat!($name, "::new"), |_, a| {
            let v = match arg(&a, 0, "new")? {
                Value::Int(n, _) => Value::Int(n, $ty),
                other => other,
            };
            Ok(Value::Cell(CellKind::Atomic, Rc::new(RefCell::new(v)), Rc::new(CellState::default())))
        });
        $r.type_name($name);
    };
}

fn atomic_slot(v: &Value) -> R<Slot> {
    match v {
        Value::Cell(CellKind::Atomic, s, _) => Ok(s.clone()),
        other => Err(semantic(format!("expected an atomic, found {}", other.type_name()))),
    }
}

fn as_ty(cur: &Value, n: i128) -> Value {
    match cur {
        Value::Int(_, t) => Value::Int(t.wrap(n), *t),
        _ => Value::int(n),
    }
}

fn fetch_op(it: &mut Interpreter, v: &Value, a: &[Value], f: fn(i128, i128) -> i128, what: &str) -> R {
    it.preempt()?;
    let slot = atomic_slot(v)?;
    let rhs = arg_int(a, 0, what)?;
    let old = slot.borrow().clone();
    let Value::Int(cur, _) = &old else { return Err(semantic(format!("`{}` needs an integer atomic", what))) };
    *slot.borrow_mut() = as_ty(&old, f(*cur, rhs));
    Ok(old)
}

pub fn register(r: &mut Registry) {
    for t in ["Condvar", "Barrier"] {
        r.type_name(t);
    }
    atomic_ctor!(r, "AtomicUsize", IntTy::Usize);
    atomic_ctor!(r, "AtomicIsize", IntTy::Isize);
    atomic_ctor!(r, "AtomicU8", IntTy::U8);
    atomic_ctor!(r, "AtomicU16", IntTy::U16);
    atomic_ctor!(r, "AtomicU32", IntTy::U32);
    atomic_ctor!(r, "AtomicU64", IntTy::U64);
    atomic_ctor!(r, "AtomicI8", IntTy::I8);
    atomic_ctor!(r, "AtomicI16", IntTy::I16);
    atomic_ctor!(r, "AtomicI32", IntTy::I32);
    atomic_ctor!(r, "AtomicI64", IntTy::I64);
    r.type_name("AtomicBool");
    r.func("AtomicBool::new", |_, a| {
        Ok(Value::Cell(CellKind::Atomic, Rc::new(RefCell::new(arg(&a, 0, "new")?)), Rc::new(CellState::default())))
    });
    for o in ["Relaxed", "Acquire", "Release", "AcqRel", "SeqCst"] {
        r.add_const(format!("Ordering::{}", o), Value::enum_val("AtomicOrdering", o, 0, Payload::Unit));
    }
    r.pure(Kind::Cell, "load", |it, v, _| {
        it.preempt()?;
        Ok(atomic_slot(v)?.borrow().clone())
    });
    r.pure(Kind::Cell, "store", |it, v, a| {
        it.preempt()?;
        let slot = atomic_slot(v)?;
        let new = arg(&a, 0, "store")?;
        let cur = slot.borrow().clone();
        *slot.borrow_mut() = match (&cur, new) {
            (Value::Int(..), Value::Int(n, _)) => as_ty(&cur, n),
            (_, n) => n,
        };
        Ok(Value::Unit)
    });
    r.pure(Kind::Cell, "fetch_add", |it, v, a| fetch_op(it, v, &a, |x, y| x + y, "fetch_add"));
    r.pure(Kind::Cell, "fetch_sub", |it, v, a| fetch_op(it, v, &a, |x, y| x - y, "fetch_sub"));
    r.pure(Kind::Cell, "fetch_max", |it, v, a| fetch_op(it, v, &a, |x, y| x.max(y), "fetch_max"));
    r.pure(Kind::Cell, "fetch_min", |it, v, a| fetch_op(it, v, &a, |x, y| x.min(y), "fetch_min"));
    r.pure(Kind::Cell, "fetch_and", |it, v, a| fetch_bool_or_int(it, v, &a, |x, y| x & y, "fetch_and"));
    r.pure(Kind::Cell, "fetch_or", |it, v, a| fetch_bool_or_int(it, v, &a, |x, y| x | y, "fetch_or"));
    r.pure(Kind::Cell, "fetch_xor", |it, v, a| fetch_bool_or_int(it, v, &a, |x, y| x ^ y, "fetch_xor"));
    for n in ["compare_exchange", "compare_exchange_weak"] {
        r.pure(Kind::Cell, n, |it, v, a| {
            it.preempt()?;
            let slot = atomic_slot(v)?;
            let (cur, new) = (arg(&a, 0, "compare_exchange")?, arg(&a, 1, "compare_exchange")?);
            let old = slot.borrow().clone();
            if old.structural_eq(&cur) {
                *slot.borrow_mut() = match (&old, new) {
                    (Value::Int(..), Value::Int(n, _)) => as_ty(&old, n),
                    (_, n) => n,
                };
                Ok(Value::ok(old))
            } else {
                Ok(Value::err(old))
            }
        });
    }
    r.pure(Kind::Cell, "fetch_update", |it, v, a| {
        it.preempt()?;
        let slot = atomic_slot(v)?;
        let f = arg(&a, 2, "fetch_update")?;
        let old = slot.borrow().clone();
        let res = it.call_value(&f, vec![old.clone()])?;
        match res.as_option() {
            Some(Some(n)) => {
                *slot.borrow_mut() = match (&old, n) {
                    (Value::Int(..), Value::Int(k, _)) => as_ty(&old, *k),
                    (_, n) => n.clone(),
                };
                Ok(Value::ok(old))
            }
            _ => Ok(Value::err(old)),
        }
    });

    r.func("Condvar::new", |_, _| Ok(Value::Native(Rc::new(CondvarObj(Rc::new(CondState::default()))))));
    r.func("Barrier::new", |_, a| {
        let n = arg_usize(&a, 0, "Barrier::new")?;
        Ok(Value::Native(Rc::new(BarrierObj(Rc::new(BarrierState { n, count: Cell::new(0), gen: Cell::new(0) })))))
    });
    fn cv(v: &Value) -> R<Rc<CondState>> {
        downcast::<CondvarObj>(v).map(|c| c.0.clone()).ok_or_else(|| semantic("expected a Condvar"))
    }
    r.native("Condvar", "wait", |it, v, a| {
        let g = raw_arg(&a, 0, "wait")?;
        cond_wait(it, &cv(v)?, &g, None)?;
        Ok(Value::ok(g))
    });
    r.native("Condvar", "wait_while", |it, v, a| {
        let g = raw_arg(&a, 0, "wait_while")?;
        let f = arg(&a, 1, "wait_while")?;
        let c = cv(v)?;
        loop {
            // the predicate gets `&mut T`: the guard itself
            let again = it.call_value(&f, vec![g.clone()])?;
            if !matches!(again, Value::Bool(true)) {
                break;
            }
            cond_wait(it, &c, &g, None)?;
        }
        Ok(Value::ok(g))
    });
    r.native("Condvar", "wait_timeout", |it, v, a| {
        let g = raw_arg(&a, 0, "wait_timeout")?;
        let ns = super::time::duration_ns(&arg(&a, 1, "wait_timeout")?)?;
        let timed_out = cond_wait(it, &cv(v)?, &g, Some(ns))?;
        Ok(Value::ok(Value::tuple(vec![g, Value::Native(Rc::new(Flag("WaitTimeoutResult", timed_out)))])))
    });
    r.native("Condvar", "notify_one", |it, v, _| {
        it.preempt()?;
        cv(v)?.notify_one();
        Ok(Value::Unit)
    });
    r.native("Condvar", "notify_all", |it, v, _| {
        it.preempt()?;
        cv(v)?.notify_all();
        Ok(Value::Unit)
    });
    r.native("WaitTimeoutResult", "timed_out", |_, v, _| {
        Ok(Value::Bool(downcast::<Flag>(v).map(|f| f.1).unwrap_or(false)))
    });
    r.native("BarrierWaitResult", "is_leader", |_, v, _| Ok(Value::Bool(downcast::<Flag>(v).map(|f| f.1).unwrap_or(false))));
    r.native("Barrier", "wait", |it, v, _| {
        let b = downcast::<BarrierObj>(v).ok_or_else(|| semantic("expected a Barrier"))?.0.clone();
        it.preempt()?;
        let gen = b.gen.get();
        b.count.set(b.count.get() + 1);
        if b.count.get() >= b.n {
            b.count.set(0);
            b.gen.set(gen + 1);
            return Ok(Value::Native(Rc::new(Flag("BarrierWaitResult", true))));
        }
        it.block_on(Wait::Barrier(b.clone(), gen))?;
        Ok(Value::Native(Rc::new(Flag("BarrierWaitResult", false))))
    });
}

fn fetch_bool_or_int(it: &mut Interpreter, v: &Value, a: &[Value], f: fn(i128, i128) -> i128, what: &str) -> R {
    let slot = atomic_slot(v)?;
    let cur = slot.borrow().clone();
    if let Value::Bool(b) = cur {
        it.preempt()?;
        let rhs = matches!(arg(a, 0, what)?, Value::Bool(true));
        *slot.borrow_mut() = Value::Bool(f(b as i128, rhs as i128) != 0);
        return Ok(Value::Bool(b));
    }
    fetch_op(it, v, a, f, what)
}

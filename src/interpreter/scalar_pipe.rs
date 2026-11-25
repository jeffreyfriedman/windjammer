//! Typed scalar pipeline: a `map`/`filter` chain over packed integers, drained without a `Value` or a `Result`
//! per element. The argument that it is observationally the boxed drain is in `docs/design/interp-soundness/pipeline.md`:
//! every closure is a pure integer expression (no call, assignment or capture write), the pipeline
//! implements only the success branch of `fast::int_op`, and on anything else (a guard, an overflow, a zero divisor)
//! it writes nothing and declines, so the boxed loop runs from the start and raises whatever it raises.

use super::engine::Interpreter;
use super::eval_expr::lead_loc;
use super::int::IntTy;
use super::iter::{Iter, IterSrc, MATERIALISE_LIMIT};
use super::value::{Callable, Value};
use super::vecbody::{Packed, VecBody};
use crate::parser::{BinaryOp, Expression, Literal, UnaryOp};
use crate::source_map::Location;
use std::rc::Rc;

/// A closure body lowered once per source site: `x`, captured names and literals under `+ - * / %` and comparisons.
pub(crate) struct Prog {
    root: Node,
    /// The location the last executed operator of a call marks (`Interpreter::mark`).
    last_loc: Option<&'static Location>,
}

enum Node {
    X,
    Lit(i128, IntTy),
    Free(&'static str),
    Bin(BinaryOp, Box<Node>, Box<Node>),
}

fn is_cmp(op: BinaryOp) -> bool {
    matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge)
}

fn lower(it: &mut Interpreter, module: usize, param: &str, e: &'static Expression<'static>, last: &mut Option<&'static Location>) -> Option<Node> {
    Some(match e {
        Expression::Identifier { name, .. } => {
            if name == param {
                Node::X
            } else {
                Node::Free(name.as_str())
            }
        }
        Expression::Literal { value: Literal::Int(i), location } => match it.literal_width(module, location.as_ref()) {
            Some(t) => match super::compiled_facts::int_literal(*i, t) {
                Value::Int(v, t) => Node::Lit(v, t),
                _ => return None,
            },
            None => Node::Lit(*i as i128, IntTy::Lit),
        },
        // integers are never references: `*x` / `&x` of one is itself
        Expression::Unary { op: UnaryOp::Deref | UnaryOp::Ref, operand, .. } => lower(it, module, param, operand, last)?,
        Expression::Binary { left, op, right, .. } => {
            if !(is_cmp(*op) || matches!(op, BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod)) {
                return None;
            }
            let l = lower(it, module, param, left, last)?;
            let r = lower(it, module, param, right, last)?;
            if let Some(loc) = lead_loc(e) {
                *last = Some(loc);
            }
            Node::Bin(*op, Box::new(l), Box::new(r))
        }
        _ => return None,
    })
}

fn plain_param(p: &str) -> bool {
    !p.is_empty() && p != "_" && !p.starts_with(|c: char| c.is_ascii_digit()) && p.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// The lowered program of a closure body (cached by address); `None` = outside the grammar.
fn prog_of(it: &mut Interpreter, c: &Rc<Callable>) -> Option<Rc<Prog>> {
    let Callable::Closure { params, body, module, tparams, .. } = &**c else { return None };
    let key = *body as *const _ as usize;
    if let Some(p) = it.scalar_progs.get(&key) {
        return p.clone();
    }
    let p = (|| {
        if params.len() != 1 || !plain_param(params[0].as_str()) || !tparams.is_empty() {
            return None;
        }
        let mut last = None;
        let root = lower(it, *module, params[0].as_str(), body, &mut last)?;
        Some(Rc::new(Prog { root, last_loc: last }))
    })();
    it.scalar_progs.insert(key, p.clone());
    p
}

#[derive(Clone, Copy, PartialEq)]
enum Ty {
    Int(IntTy),
    Bool,
}

enum Op {
    X,
    K(i64),
    /// `x op k`
    XK { op: BinaryOp, lo: i64, hi: i64, k: i64 },
    Arith { op: BinaryOp, lo: i64, hi: i64, l: Box<Op>, r: Box<Op> },
    Cmp { op: BinaryOp, l: Box<Op>, r: Box<Op> },
    /// `(x aop k) cop c`: the filter shape `x % 3 == 0`
    CmpXK { cop: BinaryOp, aop: BinaryOp, lo: i64, hi: i64, k: i64, c: i64 },
    /// `l op k` for a comparison
    CmpK { op: BinaryOp, l: Box<Op>, k: i64 },
}

fn wide(t: IntTy) -> bool {
    matches!(t, IntTy::I128 | IntTy::U128)
}

fn range_of(t: IntTy) -> (i64, i64) {
    let lo = if t.fits(i64::MIN as i128) { i64::MIN } else { t.min() as i64 };
    let hi = if t.fits(i64::MAX as i128) { i64::MAX } else { t.max() as i64 };
    (lo, hi)
}

/// `(op tree, type, mentions only constants)`
fn bind(n: &Node, xt: IntTy, env: &[(&'static str, super::value::Slot)]) -> Option<(Op, Ty, bool)> {
    Some(match n {
        Node::X => (Op::X, Ty::Int(xt), false),
        Node::Lit(v, t) => (Op::K(i64::try_from(*v).ok()?), Ty::Int(*t), true),
        Node::Free(name) => {
            // the closure compiler's rule: the last binding of the name
            let (_, slot) = env.iter().rev().find(|(n, _)| n == name)?;
            let g = slot.try_borrow().ok()?;
            match &*g {
                Value::Int(v, t) => (Op::K(i64::try_from(*v).ok()?), Ty::Int(*t), false),
                _ => return None,
            }
        }
        Node::Bin(op, l, r) => {
            let (lo_, lt, lc) = bind(l, xt, env)?;
            let (ro_, rt, rc) = bind(r, xt, env)?;
            let (Ty::Int(a), Ty::Int(b)) = (lt, rt) else { return None };
            if wide(a) || wide(b) || (lc && rc) {
                return None;
            }
            if is_cmp(*op) {
                match ro_ {
                    Op::K(c) if matches!(lo_, Op::XK { .. }) => {
                        let Op::XK { op: aop, lo, hi, k } = lo_ else { unreachable!() };
                        (Op::CmpXK { cop: *op, aop, lo, hi, k, c }, Ty::Bool, false)
                    }
                    Op::K(k) => (Op::CmpK { op: *op, l: Box::new(lo_), k }, Ty::Bool, false),
                    r => (Op::Cmp { op: *op, l: Box::new(lo_), r: Box::new(r) }, Ty::Bool, false),
                }
            } else {
                let ty = IntTy::unify(a, b);
                let (lo, hi) = range_of(ty);
                match (lo_, ro_) {
                    (Op::X, Op::K(k)) => (Op::XK { op: *op, lo, hi, k }, Ty::Int(ty), false),
                    (l, r) => (Op::Arith { op: *op, lo, hi, l: Box::new(l), r: Box::new(r) }, Ty::Int(ty), false),
                }
            }
        }
    })
}

/// `a op b` in `i64` with the width's range check: exactly the success branch of `fast::int_op`.
#[inline(always)]
fn arith(op: BinaryOp, a: i64, b: i64, lo: i64, hi: i64) -> Option<i64> {
    let r = match op {
        BinaryOp::Add => a.checked_add(b)?,
        BinaryOp::Sub => a.checked_sub(b)?,
        BinaryOp::Mul => a.checked_mul(b)?,
        BinaryOp::Div if b != 0 => a.checked_div(b)?,
        BinaryOp::Mod if b != 0 => a.checked_rem(b)?,
        _ => return None,
    };
    (r >= lo && r <= hi).then_some(r)
}

#[inline(always)]
fn cmp(op: BinaryOp, a: i64, b: i64) -> i64 {
    (match op {
        BinaryOp::Eq => a == b,
        BinaryOp::Ne => a != b,
        BinaryOp::Lt => a < b,
        BinaryOp::Le => a <= b,
        BinaryOp::Gt => a > b,
        _ => a >= b,
    }) as i64
}

impl Op {
    /// `None`: this element needs the boxed path (overflow, zero divisor, ...).
    #[inline(always)]
    fn ev(&self, x: i64) -> Option<i64> {
        match self {
            Op::X => Some(x),
            Op::K(k) => Some(*k),
            Op::XK { op, lo, hi, k } => arith(*op, x, *k, *lo, *hi),
            Op::Arith { op, lo, hi, l, r } => {
                let a = l.ev(x)?;
                let b = r.ev(x)?;
                arith(*op, a, b, *lo, *hi)
            }
            Op::Cmp { op, l, r } => {
                let a = l.ev(x)?;
                let b = r.ev(x)?;
                Some(cmp(*op, a, b))
            }
            Op::CmpXK { cop, aop, lo, hi, k, c } => Some(cmp(*cop, arith(*aop, x, *k, *lo, *hi)?, *c)),
            Op::CmpK { op, l, k } => Some(cmp(*op, l.ev(x)?, *k)),
        }
    }
}

/// A stage compiled to a function pointer per shape and operator (`x aop k` / `(x aop k) cop c`), so the per-item
/// code has no operator dispatch; anything else walks its `Op` tree.
type StageFn = fn(&Stage, i64) -> Option<i64>;

struct Stage {
    f: StageFn,
    filter: bool,
    op: Op,
    k: i64,
    c: i64,
    lo: i64,
    hi: i64,
    loc: Option<&'static Location>,
}

fn acode(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Add => 0,
        BinaryOp::Sub => 1,
        BinaryOp::Mul => 2,
        BinaryOp::Div => 3,
        _ => 4,
    }
}

fn ccode(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Eq => 0,
        BinaryOp::Ne => 1,
        BinaryOp::Lt => 2,
        BinaryOp::Le => 3,
        BinaryOp::Gt => 4,
        _ => 5,
    }
}

#[inline(always)]
fn ar<const A: u8>(a: i64, b: i64) -> Option<i64> {
    match A {
        0 => a.checked_add(b),
        1 => a.checked_sub(b),
        2 => a.checked_mul(b),
        3 if b != 0 => a.checked_div(b),
        4 if b != 0 => a.checked_rem(b),
        _ => None,
    }
}

#[inline(always)]
fn cm<const C: u8>(a: i64, b: i64) -> bool {
    match C {
        0 => a == b,
        1 => a != b,
        2 => a < b,
        3 => a <= b,
        4 => a > b,
        _ => a >= b,
    }
}

fn map_xk<const A: u8>(s: &Stage, x: i64) -> Option<i64> {
    let r = ar::<A>(x, s.k)?;
    (r >= s.lo && r <= s.hi).then_some(r)
}

fn filt_xk<const A: u8, const C: u8>(s: &Stage, x: i64) -> Option<i64> {
    let r = ar::<A>(x, s.k)?;
    if r < s.lo || r > s.hi {
        return None;
    }
    Some(cm::<C>(r, s.c) as i64)
}

fn general(s: &Stage, x: i64) -> Option<i64> {
    s.op.ev(x)
}

macro_rules! pick_filt {
    ($a:expr, $c:expr; $($av:literal),*) => {
        match $a { $( $av => match $c {
            0 => filt_xk::<$av, 0> as StageFn, 1 => filt_xk::<$av, 1> as StageFn, 2 => filt_xk::<$av, 2> as StageFn,
            3 => filt_xk::<$av, 3> as StageFn, 4 => filt_xk::<$av, 4> as StageFn, _ => filt_xk::<$av, 5> as StageFn,
        }, )* _ => general as StageFn }
    };
}

fn stage_of(filter: bool, op: Op, loc: Option<&'static Location>) -> Stage {
    let mut st = Stage { f: general, filter, op: Op::X, k: 0, c: 0, lo: 0, hi: 0, loc };
    match &op {
        Op::XK { op: a, lo, hi, k } if !filter => {
            (st.k, st.lo, st.hi) = (*k, *lo, *hi);
            st.f = match acode(*a) {
                0 => map_xk::<0>,
                1 => map_xk::<1>,
                2 => map_xk::<2>,
                3 => map_xk::<3>,
                _ => map_xk::<4>,
            };
        }
        Op::CmpXK { cop, aop, lo, hi, k, c } if filter => {
            (st.k, st.c, st.lo, st.hi) = (*k, *c, *lo, *hi);
            st.f = pick_filt!(acode(*aop), ccode(*cop); 0, 1, 2, 3, 4);
        }
        _ => {}
    }
    st.op = op;
    st
}

enum Source<'a> {
    Buf(&'a [i64], &'a std::cell::Cell<usize>, usize),
    /// An unpacked body whose items must all be integers of the first item's width (checked as they are read).
    Vals(&'a [Value], &'a std::cell::Cell<usize>, usize),
    Range(i64, i64, &'a std::cell::Cell<i128>, i128),
}

struct Plan<'a> {
    source: Source<'a>,
    /// The source's item width and the width of what the last stage yields.
    xt: IntTy,
    ty: IntTy,
    stages: Vec<Stage>,
    count: usize,
}

/// Drain `s` (a chain of `map`/`filter` over packed integers) without boxing, or `None` with nothing changed.
pub(crate) fn drain(it: &mut Interpreter, s: &Iter) -> Option<(Vec<i64>, IntTy)> {
    let p = plan(it, s)?;
    let mut out: Vec<i64> = Vec::with_capacity(p.count);
    let ty = exec(it, p, |x| {
        out.push(x);
        Some(())
    })?;
    Some((out, ty))
}

/// `sum` of the chain, adding in `i64` as the items are produced: the boxed `sum` drains first and adds after, so an
/// addition that overflows is a bail (the boxed path then runs every closure before it raises the overflow).
/// `hint`: the declared integer type of the result, if any.
pub(crate) fn sum(it: &mut Interpreter, s: &Iter, hint: Option<IntTy>) -> Option<Value> {
    let p = plan(it, s)?;
    let acc0 = hint.unwrap_or(p.ty);
    // the boxed `add_ints` of two different concrete widths is `int_binary`'s business
    if !(acc0 == p.ty || acc0 == IntTy::Lit || p.ty == IntTy::Lit) || wide(acc0) {
        return None;
    }
    let rt = IntTy::unify(acc0, p.ty);
    let (lo, hi) = range_of(rt);
    let (mut acc, mut n) = (0i64, 0usize);
    exec(it, p, |x| {
        acc = acc.checked_add(x)?;
        if acc < lo || acc > hi {
            return None;
        }
        n += 1;
        Some(())
    })?;
    // an empty sum is the boxed `sum`'s own seed
    Some(match (n, hint) {
        (0, None) => Value::int(0),
        (0, Some(t)) => Value::Int(0, t),
        _ => Value::Int(acc as i128, rt),
    })
}

fn plan<'a>(it: &mut Interpreter, s: &'a Iter) -> Option<Plan<'a>> {
    if !it.engine_resolved || !it.drop_types.is_empty() || it.hot.is_some() || it.limits.is_some() || it.fast_off("scalarpipe") || it.fast_off("scalarbail") {
        // a bail only exists for a chain that started typed, so switching `scalarbail` off means not starting one
        return None;
    }
    // the chain, top down to the source
    let mut fs: Vec<(bool, &Rc<Callable>)> = Vec::new();
    let mut node = s;
    let src = loop {
        if node.peeked.borrow().is_some() {
            return None;
        }
        match &node.src {
            IterSrc::Map(inner, Value::Func(f)) => {
                fs.push((false, f));
                node = inner;
            }
            IterSrc::Filter(inner, Value::Func(f)) => {
                fs.push((true, f));
                node = inner;
            }
            IterSrc::Buf { .. } | IterSrc::Range { .. } => break &node.src,
            _ => return None,
        }
    };
    if fs.is_empty() {
        return None;
    }
    // the source and its element type
    let (source, xt, count) = match src {
        IterSrc::Buf { items, pos, end } => {
            let (p, e) = (pos.get(), end.get());
            match &**items {
                VecBody::Packed(Packed::Int(v, t), _) => {
                    if p > e || e > v.len() {
                        return None;
                    }
                    (Source::Buf(&v[p..e], pos, e), *t, e - p)
                }
                VecBody::Values(v) => {
                    if p > e || e > v.len() {
                        return None;
                    }
                    let t = match v[p..e].first() {
                        Some(Value::Int(_, t)) => *t,
                        Some(_) => return None,
                        None => IntTy::Lit,
                    };
                    (Source::Vals(&v[p..e], pos, e), t, e - p)
                }
                _ => return None,
            }
        }
        IterSrc::Range { cur, end, ty } => {
            let (c, e) = (cur.get(), end.get());
            if wide(*ty) || e == i128::MAX || c > e || i64::try_from(c).is_err() || i64::try_from(e).is_err() {
                return None;
            }
            (Source::Range(c as i64, e as i64, cur, e), *ty, (e - c) as usize)
        }
        _ => return None,
    };
    if wide(xt) || count > MATERIALISE_LIMIT {
        return None;
    }
    // stages, source first
    let mut stages: Vec<Stage> = Vec::with_capacity(fs.len());
    let mut ty = xt;
    for (filter, f) in fs.iter().rev() {
        let prog = prog_of(it, f)?;
        let Callable::Closure { env, .. } = &***f else { return None };
        let (op, t, _) = bind(&prog.root, ty, env)?;
        match (filter, t) {
            (true, Ty::Bool) => {}
            (false, Ty::Int(t)) if !wide(t) => ty = t,
            _ => return None,
        }
        stages.push(stage_of(*filter, op, prog.last_loc));
    }
    Some(Plan { source, xt, ty, stages, count })
}

/// Run the stages over the source, feeding each result to `sink`; `None` (a bail) leaves everything as found.
fn exec<S: FnMut(i64) -> Option<()>>(it: &mut Interpreter, p: Plan, mut sink: S) -> Option<IntTy> {
    let Plan { source, xt, ty, stages, count } = p;
    // a call checks the stack budget before it runs
    if count > 0 && it.depth_check().is_err() {
        return None;
    }
    let mut last: Option<&'static Location> = None;
    let mut run = |x0: i64| -> Option<()> {
        let mut x = x0;
        for st in &stages {
            if st.loc.is_some() {
                last = st.loc;
            }
            let r = (st.f)(st, x)?;
            if st.filter {
                if r == 0 {
                    return Some(());
                }
            } else {
                x = r;
            }
        }
        sink(x)
    };
    let done: Option<()> = (|| {
        match &source {
            Source::Buf(v, ..) => {
                for &x in v.iter() {
                    run(x)?;
                }
            }
            Source::Vals(v, ..) => {
                for x in v.iter() {
                    match x {
                        Value::Int(a, t) if *t == xt && i64::try_from(*a).is_ok() => run(*a as i64)?,
                        _ => return None,
                    }
                }
            }
            Source::Range(c, e, ..) => {
                for x in *c..*e {
                    run(x)?;
                }
            }
        }
        Some(())
    })();
    if done.is_none() {
        // nothing was written: the boxed loop runs from the source's first item and raises what it raises
        it.fast.scalarbail += 1;
        return None;
    }
    // success: leave the source and the marked location as the boxed drain does
    match source {
        Source::Buf(_, pos, e) | Source::Vals(_, pos, e) => pos.set(e),
        Source::Range(_, _, cur, e) => cur.set(e),
    }
    it.fast.scalarpipe += 1;
    if last.is_some() {
        it.cur = last;
    }
    Some(ty)
}

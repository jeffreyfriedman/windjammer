//! AST -> closure tree for `--engine=resolved`: the driver, slot typing and statements
//! (expressions are in `expr.rs`).
//!
//! Subset: parameters bound by name (no patterns, no `self`, no generics, not async/extern);
//! statements `let x [: T] = e` / `let _ = e`, assignment and compound assignment to a local,
//! `return`, `if`/`else`, `while`, `loop`, `for x in <iterable>`, `break [value]`, `continue`
//! (labels included), expression statements. Anything else returns `None` and the function runs
//! in the walker.
//!
//! Slot typing: a local whose every write provably stores an integer of one `small` width `T` is a
//! typed slot; reads of it compile to raw `i64` code (`typed.rs`). The proof is by construction:
//! each write site checks the static width of what it stores (`assign_ok`); a write that could
//! change the width (a literal that might not fit, another width, a non-integer) poisons the slot
//! and the function is compiled again with that slot untyped.

use super::expr::{self, X};
use super::fast::Cond;
use super::typed::{self, small, ICode, I};
use super::{Body, Code, CompiledFn};
use crate::interpreter::engine::{FuncDef, Interpreter};
use crate::interpreter::error::{Ctl, R};
use crate::interpreter::eval_expr::stmt_loc;
use crate::interpreter::eval_stmt::loop_exit;
use crate::interpreter::int::IntTy;
use crate::interpreter::value::*;
use crate::parser::{BinaryOp, CompoundOp, Expression, Pattern, Statement, Type};
use crate::source_map::Location;
use std::collections::HashSet;
use std::rc::Rc;

pub(super) type E = &'static Expression<'static>;
type S = &'static Statement<'static>;
type Loc = Option<&'static Location>;

pub(super) struct Cx<'a> {
    pub(super) it: &'a mut Interpreter,
    pub(super) module: usize,
    scopes: Vec<Vec<(&'static str, usize)>>,
    nslots: usize,
    /// Static width of each slot (`None`: any value).
    slot_ty: Vec<Option<IntTy>>,
    poison: &'a HashSet<usize>,
    conflicts: Vec<usize>,
    /// A `return` whose value is not a typed integer was compiled (no typed body).
    untyped_return: bool,
}

impl Cx<'_> {
    pub(super) fn local(&self, name: &str) -> Option<usize> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(n, _)| *n == name).map(|(_, i)| *i)
    }
    pub(super) fn slot_ty(&self, slot: usize) -> Option<IntTy> {
        self.slot_ty[slot]
    }
    fn declare(&mut self, name: &'static str, ty: Option<IntTy>) -> usize {
        let i = self.nslots;
        self.nslots += 1;
        self.slot_ty.push(ty.filter(|t| small(*t) && !self.poison.contains(&i)));
        self.scopes.last_mut().expect("scope").push((name, i));
        i
    }
    /// Does storing a value of static width `rhs` (`None`: not a typed integer; with its value
    /// when constant) into typed `slot` keep the slot's width, for `=` (`op` `None`) or `op=`?
    /// Records a conflict (the slot is untyped on the next compile) when it does not.
    fn assign_ok(&mut self, slot: usize, rhs: Option<(IntTy, Option<i64>)>, op: Option<BinaryOp>) -> bool {
        let Some(t) = self.slot_ty[slot] else { return false };
        let full = t.min() <= i64::MIN as i128 && t.max() >= i64::MAX as i128;
        let ok = match (rhs, op) {
            (None, _) => false,
            // `retag`: a literal adopts the slot's width when it fits
            (Some((u, k)), None) => u == t || (u == IntTy::Lit && (full || k.is_some_and(|k| t.fits(k as i128)))),
            (Some((u, _)), Some(op)) => typed::result_ty(op, t, u) == Some(t),
        };
        if !ok {
            self.conflicts.push(slot);
        }
        ok
    }
}

pub(super) fn compile_fn(it: &mut Interpreter, f: &Rc<FuncDef>) -> Option<CompiledFn> {
    if f.is_extern || f.is_async || !f.type_params.is_empty() || f.self_ty.is_some() {
        return None;
    }
    if f.params.iter().any(|p| p.name == "self" || !matches!(p.pattern, None | Some(Pattern::Identifier(_)))) {
        return None;
    }
    // a parameter the body mutates is passed by reference (inferred `&mut`): walker only
    if it.param_mutation(f).iter().any(|m| *m) {
        return None;
    }
    let mut poison = HashSet::new();
    loop {
        let mut cx = Cx {
            it: &mut *it,
            module: f.module,
            scopes: vec![Vec::new()],
            nslots: 0,
            slot_ty: Vec::new(),
            poison: &poison,
            conflicts: Vec::new(),
            untyped_return: false,
        };
        let r = compile_with(&mut cx, f);
        let conflicts = std::mem::take(&mut cx.conflicts);
        if conflicts.is_empty() || r.is_none() {
            return r;
        }
        poison.extend(conflicts);
    }
}

fn compile_with(cx: &mut Cx, f: &Rc<FuncDef>) -> Option<CompiledFn> {
    let param_int: Vec<Option<IntTy>> =
        f.params.iter().map(|p| IntTy::from_type(&p.type_).filter(|t| *t != IntTy::Lit)).collect();
    let params: Vec<usize> = f.params.iter().zip(&param_int).map(|(p, t)| cx.declare(p.name.as_str(), *t)).collect();
    let param_typed = params.iter().map(|s| cx.slot_ty[*s].is_some()).collect();
    let ret_int = f.ret.and_then(IntTy::from_type).filter(|t| *t != IntTy::Lit);
    let typed_ret = ret_int.filter(|t| small(*t));
    let (init, tail) = stmt_list(cx, f.body, typed_ret.is_some())?;
    let body = match (typed_ret, tail) {
        (Some(t), Some((loc, Tail::Int(i, u)))) if !cx.untyped_return => Body::I(typed_seq(init, u, loc, i.into_code()), u, t),
        (_, tail) => Body::V(seq(init, tail.map(|(l, t)| (l, t.code())))),
    };
    let scalar = cx.slot_ty.iter().all(|t| t.is_some());
    Some(CompiledFn { nslots: cx.nslots, params, param_int, param_typed, ret_int, scalar, body })
}

/// The last statement of a list: a typed integer expression, or any code.
enum Tail {
    Int(I, IntTy),
    Code(Code),
}

impl Tail {
    fn code(self) -> Code {
        match self {
            Tail::Int(i, t) => X::I(i, t).code(),
            Tail::Code(c) => c,
        }
    }
}

/// A statement before the tail: code, or (in a typed body) `if c { return v }` with a typed `v`,
/// which the typed body runs as an early exit instead of unwinding a `Ctl::Return`.
enum Step {
    Code(Loc, Code),
    Guard { loc: Loc, cond: Cond, rloc: Loc, val: I, w: IntTy },
}

impl Step {
    fn code(self) -> (Loc, Code) {
        match self {
            Step::Code(l, c) => (l, c),
            Step::Guard { loc, cond, rloc, val, w } => (
                loc,
                Box::new(move |it, b| {
                    if cond(it, b)? {
                        it.mark(rloc);
                        return Err(Ctl::Return(Value::Int(val.get(it, b)? as i128, w)));
                    }
                    Ok(Value::Unit)
                }),
            ),
        }
    }
}

type Init = Vec<Step>;

/// `if c { return e }` (no `else`) as a `Step::Guard` when `e` is a typed integer.
fn guard(cx: &mut Cx, s: S) -> Option<Option<Step>> {
    let Statement::If { condition, then_block, else_block: None, .. } = s else { return Some(None) };
    let [r @ Statement::Return { value: Some(e), .. }] = then_block.as_slice() else { return Some(None) };
    let cond = expr::cond(cx, condition, "if", true)?;
    cx.scopes.push(Vec::new());
    let v = expr::expr(cx, e);
    cx.scopes.pop();
    let (loc, rloc) = (stmt_loc(s), stmt_loc(r));
    Some(Some(match v? {
        X::I(val, w) => Step::Guard { loc, cond, rloc, val, w },
        x => {
            cx.untyped_return = true;
            let v = x.code();
            Step::Code(
                loc,
                Box::new(move |it, b| {
                    if cond(it, b)? {
                        it.mark(rloc);
                        return Err(Ctl::Return(v(it, b)?));
                    }
                    Ok(Value::Unit)
                }),
            )
        }
    }))
}

/// Every statement of a list but the last, and the last one (in value position).
fn stmt_list(cx: &mut Cx, list: &'static [S], guards: bool) -> Option<(Init, Option<(Loc, Tail)>)> {
    let Some((last, init)) = list.split_last() else { return Some((Vec::new(), None)) };
    let mut steps = Vec::with_capacity(init.len());
    for s in init {
        match if guards { guard(cx, s)? } else { None } {
            Some(g) => steps.push(g),
            None => steps.push(Step::Code(stmt_loc(s), stmt(cx, s)?)),
        }
    }
    let init = steps;
    let tail = match last {
        Statement::Expression { expr: e, .. } => match expr::expr(cx, e)? {
            X::I(i, t) => Tail::Int(i, t),
            x => Tail::Code(x.code()),
        },
        s => Tail::Code(stmt(cx, s)?),
    };
    Some((init, Some((stmt_loc(last), tail))))
}

/// The walker marks every statement's location before executing it.
fn seq(init: Init, tail: Option<(Loc, Code)>) -> Code {
    let init: Vec<(Loc, Code)> = init.into_iter().map(Step::code).collect();
    let Some((lloc, lc)) = tail else { return Box::new(|_, _| Ok(Value::Unit)) };
    Box::new(move |it, b| {
        for (loc, c) in &init {
            it.mark(*loc);
            c(it, b)?;
        }
        it.mark(lloc);
        lc(it, b)
    })
}

/// A typed body; guards returning the tail's width `u` exit with their value directly.
fn typed_seq(init: Init, u: IntTy, lloc: Loc, lc: ICode) -> ICode {
    let init: Vec<Step> = init.into_iter().map(|s| match s {
        Step::Guard { w, .. } if w == u => s,
        s => {
            let (l, c) = s.code();
            Step::Code(l, c)
        }
    }).collect();
    Box::new(move |it, b| {
        for s in &init {
            match s {
                Step::Code(loc, c) => {
                    it.mark(*loc);
                    c(it, b)?;
                }
                Step::Guard { loc, cond, rloc, val, .. } => {
                    it.mark(*loc);
                    if cond(it, b)? {
                        it.mark(*rloc);
                        return val.get(it, b);
                    }
                }
            }
        }
        it.mark(lloc);
        lc(it, b)
    })
}

/// A nested block: its own name scope (slots are never reused, so no runtime work).
pub(super) fn block(cx: &mut Cx, list: &'static [S]) -> Option<Code> {
    cx.scopes.push(Vec::new());
    let c = stmt_list(cx, list, false).map(|(init, tail)| seq(init, tail.map(|(l, t)| (l, t.code()))));
    cx.scopes.pop();
    c
}

fn stmt(cx: &mut Cx, s: S) -> Option<Code> {
    let code: Code = match s {
        Statement::Expression { expr: e, .. } => expr::expr(cx, e)?.code(),
        Statement::Let { pattern, type_, value, else_block: None, .. } => {
            let name = match pattern {
                Pattern::Identifier(n) | Pattern::MutBinding(n) => Some(n.as_str()),
                Pattern::Wildcard => None,
                _ => return None,
            };
            let v = expr::expr(cx, value)?;
            let_stmt(cx, name, type_.as_ref(), v)
        }
        Statement::Assignment { target, value, compound_op, .. } => {
            let Expression::Identifier { name, location } = target else { return None };
            let slot = cx.local(name)?;
            let v = expr::expr(cx, value)?;
            assign(cx, slot, v, *compound_op, location.as_ref())
        }
        Statement::Return { value, .. } => match value {
            Some(e) => match expr::expr(cx, e)? {
                X::I(i, t) => Box::new(move |it, b| Err(Ctl::Return(Value::Int(i.get(it, b)? as i128, t)))),
                x => {
                    cx.untyped_return = true;
                    let v = x.code();
                    Box::new(move |it, b| Err(Ctl::Return(v(it, b)?)))
                }
            },
            None => {
                cx.untyped_return = true;
                Box::new(|_, _| Err(Ctl::Return(Value::Unit)))
            }
        },
        Statement::If { condition, then_block, else_block, .. } => {
            let c = expr::cond(cx, condition, "if", true)?;
            let t = block(cx, then_block)?;
            let e = match else_block {
                Some(eb) => Some(block(cx, eb)?),
                None => None,
            };
            Box::new(move |it, b| match c(it, b)? {
                true => t(it, b),
                false => match &e {
                    Some(e) => e(it, b),
                    None => Ok(Value::Unit),
                },
            })
        }
        Statement::While { condition, body, label, .. } => {
            let c = expr::cond(cx, condition, "while", false)?;
            let body = block(cx, body)?;
            let label: Option<&'static str> = label.as_deref();
            Box::new(move |it, b| {
                while c(it, b)? {
                    if loop_exit(body(it, b), label)?.is_some() {
                        break;
                    }
                }
                Ok(Value::Unit)
            })
        }
        Statement::Loop { body, label, .. } => {
            let body = block(cx, body)?;
            let label: Option<&'static str> = label.as_deref();
            Box::new(move |it, b| loop {
                if let Some(v) = loop_exit(body(it, b), label)? {
                    return Ok(v);
                }
            })
        }
        Statement::For { pattern, iterable, body, label, .. } => for_stmt(cx, pattern, iterable, body, label.as_deref())?,
        Statement::Break { label, value, .. } => {
            let v = match value {
                Some(e) => Some(expr::expr(cx, e)?.code()),
                None => None,
            };
            let label = label.clone();
            Box::new(move |it, b| {
                let value = match &v {
                    Some(v) => v(it, b)?,
                    None => Value::Unit,
                };
                Err(Ctl::Break(Box::new((label.clone(), value))))
            })
        }
        Statement::Continue { label, .. } => {
            let label = label.clone();
            Box::new(move |_, _| Err(Ctl::Continue(Box::new(label.clone()))))
        }
        Statement::Use { .. } => Box::new(|_, _| Ok(Value::Unit)),
        _ => return None,
    };
    Some(code)
}

/// `let [name][: ty] = v`: a typed integer value (with an integer or no annotation) makes a typed slot.
fn let_stmt(cx: &mut Cx, name: Option<&'static str>, ty: Option<&'static Type>, v: X) -> Code {
    let width = match (&v, ty.map(IntTy::from_type)) {
        (X::I(_, u), None) => Some(*u),
        (X::I(..), Some(Some(t))) if small(t) => Some(t),
        _ => None,
    };
    if let (Some(t), X::I(..)) = (width, &v) {
        let X::I(i, u) = v else { unreachable!() };
        let slot = name.map(|n| cx.declare(n, Some(t)));
        let typed_slot = slot.is_some_and(|s| cx.slot_ty(s).is_some());
        return Box::new(move |it, b| {
            let mut x = i.get(it, b)?;
            if let Some(ty) = ty {
                x = typed::retype(it, x, u, t, ty)?;
            }
            match slot {
                Some(s) if typed_slot => typed::set_int(it, b + s, x, t),
                Some(s) => super::fast::set(it, b + s, Value::Int(x as i128, t)),
                None => {}
            }
            Ok(Value::Unit)
        });
    }
    let v = v.code();
    let slot = name.map(|n| cx.declare(n, None));
    Box::new(move |it, b| {
        let mut x = v(it, b)?;
        if let Some(t) = ty {
            x = it.coerce(x, t)?;
        }
        if let Some(s) = slot {
            super::fast::set(it, b + s, x);
        }
        Ok(Value::Unit)
    })
}

/// `slot = v` / `slot op= v`; the walker evaluates the right side, marks the target, then stores.
fn assign(cx: &mut Cx, slot: usize, v: X, op: Option<CompoundOp>, tloc: Loc) -> Code {
    let bop = op.map(crate::interpreter::ops::compound_to_binary);
    let rhs = match &v {
        X::I(I::Const(k), u) => Some((*u, Some(*k))),
        X::I(_, u) => Some((*u, None)),
        _ => None,
    };
    if let Some(t) = cx.slot_ty(slot) {
        if cx.assign_ok(slot, rhs, bop) {
            let X::I(i, u) = v else { unreachable!("assign_ok accepts typed integers only") };
            return match bop {
                None => Box::new(move |it, b| {
                    let x = i.get(it, b)?;
                    it.mark(tloc);
                    typed::set_int(it, b + slot, x, t);
                    Ok(Value::Unit)
                }),
                Some(bop) => {
                    let (code, _) = typed::arith(bop, I::Slot(slot), t, i, u, tloc).expect("integer operator");
                    Box::new(move |it, b| {
                        let x = code(it, b)?;
                        typed::set_int(it, b + slot, x, t);
                        Ok(Value::Unit)
                    })
                }
            };
        }
    }
    untyped_assign(slot, v.code(), op, tloc)
}

fn untyped_assign(slot: usize, v: Code, op: Option<CompoundOp>, tloc: Loc) -> Code {
    Box::new(move |it, b| {
        let rhs = v(it, b)?;
        it.mark(tloc);
        let new = match op {
            None => retag(&it.rstack[b + slot], rhs),
            Some(cop) => {
                let bop = crate::interpreter::ops::compound_to_binary(cop);
                if let Some(r) = super::fast::compound_int(it, b, slot, bop, &rhs) {
                    return r.map(|_| Value::Unit);
                }
                let cur = std::mem::replace(&mut it.rstack[b + slot], Value::Unit);
                if let (Value::Str(_), Value::Str(add), CompoundOp::Add) = (&cur, &rhs, cop) {
                    let mut cur = cur;
                    if let Value::Str(rc) = &mut cur {
                        Rc::make_mut(rc).push_str(add);
                    }
                    it.rstack[b + slot] = cur;
                    return Ok(Value::Unit);
                }
                match it.binary(bop, cur.clone(), rhs) {
                    Ok(x) => x,
                    Err(e) => {
                        it.rstack[b + slot] = cur;
                        return Err(e);
                    }
                }
            }
        };
        super::fast::set(it, b + slot, new);
        Ok(Value::Unit)
    })
}

/// The walker's `retag`: assigning a bare literal keeps the declared width of the old value.
fn retag(old: &Value, new: Value) -> Value {
    match (old, &new) {
        (Value::Int(_, ot), Value::Int(n, IntTy::Lit)) if *ot != IntTy::Lit && ot.fits(*n) => Value::Int(*n, *ot),
        (Value::Float(_, ot), Value::Float(n, FloatTy::Lit)) => Value::Float(*n, *ot),
        (Value::Float(_, ot), Value::Int(n, IntTy::Lit)) => Value::Float(*n as f64, *ot),
        _ => new,
    }
}

/// The loop variable's slot (in a new scope the caller pops), or `Err(())` for an unsupported pattern.
fn loop_var(cx: &mut Cx, pattern: &'static Pattern<'static>, ty: Option<IntTy>) -> Result<Option<usize>, ()> {
    cx.scopes.push(Vec::new());
    match pattern {
        Pattern::Identifier(n) | Pattern::MutBinding(n) => Ok(Some(cx.declare(n.as_str(), ty))),
        Pattern::Wildcard => Ok(None),
        _ => {
            cx.scopes.pop();
            Err(())
        }
    }
}

fn for_stmt(cx: &mut Cx, pattern: &'static Pattern<'static>, iterable: E, body: &'static [S], label: Option<&'static str>) -> Option<Code> {
    let src = match iterable {
        Expression::Range { start, end, inclusive, .. } => match (expr::expr(cx, start)?, expr::expr(cx, end)?) {
            // `for i in a..b` over typed bounds: count in place, the variable is a typed slot
            (X::I(s, st), X::I(e, et)) => return typed_for(cx, (s, e, *inclusive, IntTy::unify(st, et)), pattern, body, label),
            (s, e) => expr::range_code(s.code(), e.code(), *inclusive),
        },
        _ => expr::expr(cx, iterable)?.code(),
    };
    let slot = loop_var(cx, pattern, None).ok()?;
    let body = block(cx, body);
    cx.scopes.pop();
    let body = body?;
    Some(Box::new(move |it, b| {
        let v = src(it, b)?;
        if let Value::Range(lo, hi, ty) = v {
            let mut i = lo;
            while i < hi {
                if let Some(s) = slot {
                    it.rstack[b + s] = Value::Int(i, ty);
                }
                if loop_exit(body(it, b), label)?.is_some() {
                    break;
                }
                i += 1;
            }
            return Ok(Value::Unit);
        }
        let iter = crate::interpreter::iter::as_iter(it, v)?;
        while let Some(x) = crate::interpreter::iter::next(it, &iter)? {
            if let Some(s) = slot {
                it.rstack[b + s] = x;
            }
            if loop_exit(body(it, b), label)?.is_some() {
                break;
            }
        }
        Ok(Value::Unit)
    }))
}

fn typed_for(cx: &mut Cx, range: (I, I, bool, IntTy), pattern: &'static Pattern<'static>, body: &'static [S], label: Option<&'static str>) -> Option<Code> {
    let (s, e, inclusive, ty) = range;
    let slot = loop_var(cx, pattern, Some(ty)).ok()?;
    let body = block(cx, body);
    cx.scopes.pop();
    let body = body?;
    let typed_var = slot.is_some_and(|s| cx.slot_ty(s).is_some());
    Some(Box::new(move |it, b| {
        let lo = s.get(it, b)? as i128;
        let hi = e.get(it, b)? as i128 + inclusive as i128;
        let mut i = lo;
        while i < hi {
            match slot {
                Some(s) if typed_var => typed::set_int(it, b + s, i as i64, ty),
                Some(s) => super::fast::set(it, b + s, Value::Int(i, ty)),
                None => {}
            }
            if loop_exit(body(it, b), label)?.is_some() {
                break;
            }
            i += 1;
        }
        Ok(Value::Unit)
    }))
}

pub(super) fn run_all(it: &mut Interpreter, b: usize, codes: &[Code]) -> R<Vec<Value>> {
    let mut out = Vec::with_capacity(codes.len());
    for c in codes {
        out.push(c(it, b)?);
    }
    Ok(out)
}

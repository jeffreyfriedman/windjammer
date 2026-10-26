//! `for` loops of `--engine=resolved`: integer ranges count in place (typed bounds make the variable a
//! typed slot), anything else iterates the walker's way (`iter::as_iter`). A range loop is an OSR
//! site: once hot, its remaining iterations can run as machine code (`jit/osr.rs`), which hands back
//! the counter to go on from.

use super::compile::{block, scope_clear, scoped, slot_kind, store, Cx, Kind, E};
use super::expr::{self, X};
use super::osr::{self, BlockInfo, OsrSite};
use super::typed::{self, I};
use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::R;
use crate::interpreter::eval_stmt::loop_exit;
use crate::interpreter::int::IntTy;
use crate::interpreter::value::Value;
use crate::parser::{Expression, Pattern, Statement};
use std::rc::Rc;

type S = &'static Statement<'static>;

/// The loop variable's slot (in a new scope the caller pops), or `Err(())` for an unsupported pattern.
fn loop_var(cx: &mut Cx, pattern: &'static Pattern<'static>, ty: Option<IntTy>) -> Result<Option<usize>, ()> {
    cx.enter();
    match pattern {
        Pattern::Identifier(n) | Pattern::MutBinding(n) => Ok(Some(cx.declare(n.as_str(), ty))),
        Pattern::Wildcard => Ok(None),
        _ => {
            cx.leave();
            Err(())
        }
    }
}

/// The loop variable and the body (released per iteration as the walker's scope is), registered
/// for machine-code loops: `(slot, kind, body)`.
fn var_and_body(cx: &mut Cx, s: S, pattern: &'static Pattern<'static>, ty: Option<IntTy>, body: &'static [S]) -> Option<(Option<usize>, Kind, Code)> {
    let slot = loop_var(cx, pattern, ty).ok()?;
    let kind = slot_kind(cx, slot);
    let code = cx.in_loop(|cx| block(cx, body));
    cx.reg.record_for(s, slot, BlockInfo { poison: cx.boxed_here(), clear: scope_clear(cx) });
    let code = scoped(cx, code);
    cx.leave();
    Some((slot, kind, code?))
}

pub(super) fn for_stmt(cx: &mut Cx, s: S, pattern: &'static Pattern<'static>, iterable: E, body: &'static [S], label: Option<&'static str>) -> Option<Code> {
    // the walker iterates a reference by reference (`for x in &mut v`, a `&mut` parameter)
    if cx.boxed_ident(iterable) {
        return None;
    }
    let site = Rc::new(OsrSite::new(s, cx.all_locals(), Rc::downgrade(&cx.reg)));
    let src = match iterable {
        Expression::Range { start, end, inclusive, .. } => match (expr::expr(cx, start)?, expr::expr(cx, end)?) {
            // `for i in a..b` over typed bounds: count in place, the variable is a typed slot
            (X::I(s0, st), X::I(e0, et)) => return typed_for(cx, s, (s0, e0, *inclusive, IntTy::unify(st, et)), pattern, body, label, site),
            (s0, e0) => expr::range_code(s0.clean().code(), e0.clean().code(), *inclusive),
        },
        _ => match expr::expr(cx, iterable)? {
            // a reference is iterated by reference: the walker's
            X::R(_) => return None,
            x => x.code(),
        },
    };
    let (slot, kind, body) = var_and_body(cx, s, pattern, None, body)?;
    Some(Box::new(move |it, b| {
        let v = src(it, b)?;
        if let Value::Range(lo, hi, ty) = v {
            return range(it, b, (lo, hi, ty), (slot, kind), &body, label, &site);
        }
        let iter = crate::interpreter::iter::as_iter(it, v)?;
        while let Some(x) = crate::interpreter::iter::next(it, &iter)? {
            if let Some(s) = slot {
                store(it, b + s, x, kind);
            }
            if loop_exit(body(it, b), label)?.is_some() {
                break;
            }
        }
        Ok(Value::Unit)
    }))
}

fn typed_for(cx: &mut Cx, s: S, bounds: (I, I, bool, IntTy), pattern: &'static Pattern<'static>, body: &'static [S], label: Option<&'static str>, site: Rc<OsrSite>) -> Option<Code> {
    let (s0, e0, inclusive, ty) = bounds;
    let (slot, kind, body) = var_and_body(cx, s, pattern, Some(ty), body)?;
    Some(Box::new(move |it, b| {
        let lo = s0.get(it, b)? as i128;
        let hi = e0.get(it, b)? as i128 + inclusive as i128;
        range(it, b, (lo, hi, ty), (slot, kind), &body, label, &site)
    }))
}

/// Iterations `lo..hi` of a range loop; a hot loop goes on in machine code from its current counter.
fn range(it: &mut Interpreter, b: usize, (lo, hi, ty): (i128, i128, IntTy), (slot, kind): (Option<usize>, Kind), body: &Code, label: Option<&'static str>, site: &OsrSite) -> R {
    let mut i = lo;
    // a machine-code run that handed the loop back is not retried before one iteration here
    let mut fresh = true;
    while i < hi {
        if fresh && site.armed(it) {
            if let Some(next) = osr::enter_for(it, b, site, i, hi, ty)? {
                i = next;
                fresh = false;
                continue;
            }
        } else {
            fresh = true;
        }
        match slot {
            Some(s) if kind == Kind::Typed => typed::set_int(it, b + s, i as i64, ty),
            Some(s) => store(it, b + s, Value::Int(i, ty), kind),
            None => {}
        }
        if loop_exit(body(it, b), label)?.is_some() {
            break;
        }
        site.tick();
        i += 1;
    }
    Ok(Value::Unit)
}

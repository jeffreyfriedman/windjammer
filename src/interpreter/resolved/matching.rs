//! `match` in `--engine=resolved`: the walker's `exec_match` with a compiled scrutinee, guards and arm
//! bodies; patterns are the walker's own `match_pattern`, and each arm's bindings go to boxed locals
//! of the arm's scope (what the walker's `define` creates), released when the arm ends.
//!
//! Only a scrutinee that is not a place (`match m.get(&k)`, `match f(x)`, a literal...) is lowered:
//! matching a place binds by reference when it is reached through one, which the walker decides.

use super::compile::{Cx, E};
use super::expr::{expr, expr_eval};
use super::island::{self, Island};
use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::{panic_ctl, R};
use crate::interpreter::place::deref_value;
use crate::interpreter::value::Value;
use crate::parser::{Expression, MatchArm, Pattern, UnaryOp};

struct Arm {
    pat: &'static Pattern<'static>,
    /// The arm's bindings: name -> boxed slot.
    slots: Vec<(&'static str, usize)>,
    guard: Option<Code>,
    body: Code,
}

/// `e` as the walker's `eval_raw`: lowered, else an island.
fn raw(cx: &mut Cx, e: E) -> Option<Code> {
    // a boxed local as is (a lock guard stays a guard), as the walker reads a variable
    if let Expression::Identifier { name, .. } = e {
        if let Some(slot) = cx.local(name).filter(|s| cx.is_boxed(*s)) {
            let name: &'static str = name.as_str();
            return Some(Box::new(move |it, b| island::read_boxed_raw(it, b + slot, name)));
        }
    }
    if let Some(x) = expr(cx, e) {
        return Some(x.code());
    }
    let island = Island::new(cx, &format!("{:?}", e));
    Some(Box::new(move |it, b| island.run(it, b, |it| it.eval_raw(e))))
}

pub(super) fn match_stmt(cx: &mut Cx, value: E, arms: &'static [MatchArm<'static>]) -> Option<Code> {
    if matches!(value, Expression::Unary { op: UnaryOp::MutRef, .. }) || Interpreter::is_place_expr(value) {
        return None;
    }
    let scrut = raw(cx, value)?;
    let mut compiled = Vec::with_capacity(arms.len());
    for arm in arms {
        cx.enter();
        let mut names = Vec::new();
        island::pattern_names(&arm.pattern, &mut names);
        let slots: Vec<(&'static str, usize)> = names.into_iter().map(|n| (n, cx.declare_boxed(n))).collect();
        let guard = match arm.guard {
            Some(g) => Some(expr_eval(cx, g)?.code()),
            None => None,
        };
        let body = raw(cx, arm.body);
        cx.leave();
        compiled.push(Arm { pat: &arm.pattern, slots, guard, body: body? });
    }
    Some(Box::new(move |it, b| {
        let raw = scrut(it, b)?;
        let (v, place, by_ref) = match raw {
            Value::Ref(p) => {
                let p = it.chase(*p)?;
                (p.read()?, Some(p), true)
            }
            other => (other, None, false),
        };
        let v = deref_value(v)?;
        for arm in &compiled {
            let mut binds = Vec::new();
            if !it.match_pattern(arm.pat, &v, place.as_ref(), by_ref, &mut binds) {
                continue;
            }
            for (n, val) in binds {
                if let Some((_, s)) = arm.slots.iter().find(|(m, _)| *m == n) {
                    super::fast::set(it, b + s, island::boxed(val));
                }
            }
            if let Some(g) = &arm.guard {
                match g(it, b) {
                    Ok(Value::Bool(true)) => {}
                    Ok(_) => {
                        release(it, b, &arm.slots, &Ok(Value::Unit));
                        continue;
                    }
                    Err(e) => {
                        let r = Err(e);
                        release(it, b, &arm.slots, &r);
                        return r;
                    }
                }
            }
            let r = (arm.body)(it, b);
            release(it, b, &arm.slots, &r);
            return r;
        }
        Err(panic_ctl(format!("no match arm matched value of type `{}`", v.type_name())))
    }))
}

/// The walker's `pop_scope` for an arm: a panic poisons the lock guards its bindings hold, then
/// the bindings are released.
fn release(it: &mut Interpreter, b: usize, slots: &[(&'static str, usize)], r: &R) {
    if matches!(r, Err(crate::interpreter::error::Ctl::Err(e)) if e.kind == crate::interpreter::error::ErrorKind::Panic) {
        for (_, s) in slots {
            if let Value::Ref(p) = &it.rstack[b + s] {
                crate::interpreter::scope::poison_if_guard(&p.slot.borrow());
            }
        }
    }
    for (_, s) in slots {
        super::fast::set(it, b + s, Value::Unit);
    }
}

//! Assignments to places -- `v[i] = x`, `p.x += d`, `a[i].f -= y`, `*r = x`, a boxed local -- with
//! the walker's own place resolution (`resolve_place_in`) and store (`assign_to_place`): the
//! compiler only supplies the variables (boxed locals' walker slots) and compiled code for the
//! sub-expressions the walker would evaluate, looked up by expression.

use super::compile::{Cx, E};
use super::expr::{expr, expr_arg, expr_eval};
use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::{semantic, R};
use crate::interpreter::place::PlaceEnv;
use crate::interpreter::value::{Slot, Value};
use crate::parser::{CompoundOp, Expression, UnaryOp};

/// Variables and compiled sub-expressions handed to a walker routine through `PlaceEnv`.
#[derive(Default)]
pub(super) struct PlaceCode {
    /// Locals named in the target (boxed): name -> slot.
    vars: Vec<(&'static str, usize)>,
    /// Compiled sub-expressions by address: what `eval` / `eval_keep_ref` would compute.
    evals: Vec<(usize, Code)>,
    raws: Vec<(usize, Code)>,
}

fn addr(e: E) -> usize {
    e as *const _ as usize
}

pub(super) struct Env<'a> {
    pc: &'a PlaceCode,
    b: usize,
}

impl PlaceCode {
    /// Compile `e` as the walker's `eval` of it.
    pub(super) fn add_eval(&mut self, cx: &mut Cx, e: E) -> Option<()> {
        let c = expr_eval(cx, e)?.code();
        self.evals.push((addr(e), c));
        Some(())
    }
    pub(super) fn env(&self, b: usize) -> Env<'_> {
        Env { pc: self, b }
    }
}

impl PlaceEnv for Env<'_> {
    fn var(&mut self, it: &Interpreter, name: &str) -> Option<Slot> {
        let (_, s) = self.pc.vars.iter().rev().find(|(n, _)| *n == name)?;
        match &it.rstack[self.b + s] {
            Value::Ref(p) => Some(p.slot.clone()),
            _ => None,
        }
    }
    fn eval(&mut self, it: &mut Interpreter, e: E) -> R {
        run(&self.pc.evals, it, self.b, e)
    }
    fn eval_keep_ref(&mut self, it: &mut Interpreter, e: E) -> R {
        run(&self.pc.raws, it, self.b, e)
    }
}

fn run(codes: &[(usize, Code)], it: &mut Interpreter, b: usize, e: E) -> R {
    match codes.iter().find(|(a, _)| *a == addr(e)) {
        Some((_, c)) => c(it, b),
        None => Err(semantic("resolved engine: a place sub-expression was not compiled")),
    }
}

/// Mirror `resolve_place_in`'s recursion: register what it may look up or evaluate.
fn prepare(cx: &mut Cx, e: E, pc: &mut PlaceCode) -> Option<()> {
    match e {
        Expression::Identifier { name, .. } => {
            if let Some(s) = cx.local(name) {
                cx.need_box(s);
                pc.vars.push((name.as_str(), s));
            }
        }
        Expression::FieldAccess { object, .. } => {
            prepare(cx, object, pc)?;
            pc.evals.push((addr(object), expr_eval(cx, object)?.code()));
        }
        Expression::Index { object, index, .. } => {
            prepare(cx, object, pc)?;
            pc.evals.push((addr(object), expr_eval(cx, object)?.code()));
            pc.evals.push((addr(index), expr_eval(cx, index)?.code()));
        }
        Expression::Unary { op: UnaryOp::Deref | UnaryOp::MutRef | UnaryOp::Ref, operand, .. } => prepare(cx, operand, pc)?,
        Expression::MethodCall { .. } | Expression::Call { .. } => pc.raws.push((addr(e), expr(cx, e)?.code())),
        _ => {}
    }
    Some(())
}

/// `target op= value` through a place (`None`: leave it to an island).
pub(super) fn assign(cx: &mut Cx, target: E, value: E, op: Option<CompoundOp>) -> Option<Code> {
    let ident = match target {
        Expression::Identifier { name, .. } => {
            let s = cx.local(name)?;
            if !cx.is_boxed(s) {
                return None; // plain and typed locals have their own stores
            }
            true
        }
        Expression::FieldAccess { .. } | Expression::Index { .. } | Expression::Unary { op: UnaryOp::Deref, .. } => false,
        _ => return None,
    };
    // the walker evaluates the right side first (`eval_keep_guard`), then resolves the target
    let rhs = expr_arg(cx, value)?.code();
    let mut pc = PlaceCode::default();
    prepare(cx, target, &mut pc)?;
    let name: &'static str = match target {
        Expression::Identifier { name, .. } => name.as_str(),
        _ => "",
    };
    Some(Box::new(move |it, b| {
        let rhs = rhs(it, b)?;
        let mut env = Env { pc: &pc, b };
        let Some(place) = it.resolve_place_in(target, &mut env)? else {
            return Err(semantic("invalid left-hand side of assignment"));
        };
        // writing a reference into a variable rebinds the variable
        if ident && matches!(rhs, Value::Ref(_)) {
            if let Some(slot) = env.var(it, name) {
                *slot.borrow_mut() = rhs;
                return Ok(Value::Unit);
            }
        }
        it.assign_to_place(target, place, rhs, op)?;
        Ok(Value::Unit)
    }))
}

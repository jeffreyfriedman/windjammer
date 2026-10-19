//! Expressions of `--engine=resolved`: each compiles to an `X` -- a typed integer (`typed.rs`),
//! a local slot, a constant, or general `Code` -- so a parent can pick the cheapest form.
//!
//! Subset: literals, locals, paths (consts, function values), unary/binary operators, `as`,
//! indexing, field reads, tuples, arrays, ranges, blocks, `format`/interpolation,
//! `print`/`println`/`eprint`/`eprintln`, calls to user functions and to callable locals.

use super::compile::{block, run_all, Cx, E};
use super::fast::{self, Cond, Operand};
use super::island::{self, Island};
use super::typed::{self, small, unbox, I};
use super::{Code, CompiledFn};
use crate::interpreter::engine::{FuncDef, Interpreter};
use crate::interpreter::error::{semantic, unsupported, R};
use crate::interpreter::int::IntTy;
use crate::interpreter::intrinsics::Intrinsic;
use crate::interpreter::place::{deref_value, strip_refs};
use crate::interpreter::value::*;
use crate::parser::{BinaryOp, Expression, Type, UnaryOp};
use crate::source_map::Location;
use std::rc::Rc;

type Loc = Option<&'static Location>;

/// A compiled expression.
pub(super) enum X {
    /// An integer of a statically known `small` width.
    I(I, IntTy),
    /// An untyped local.
    Slot(usize),
    Const(Value),
    V(Code),
}

impl X {
    pub(super) fn code(self) -> Code {
        match self {
            X::V(c) => c,
            X::Slot(s) | X::I(I::Slot(s), _) => Box::new(move |it, b| Ok(it.rstack[b + s].clone())),
            X::Const(v) => Box::new(move |_, _| Ok(v.clone())),
            X::I(I::Const(c), t) => Box::new(move |_, _| Ok(Value::Int(c as i128, t))),
            X::I(I::Code(c), t) => Box::new(move |it, b| Ok(Value::Int(c(it, b)? as i128, t))),
        }
    }

    fn operand(self) -> Operand {
        match self {
            X::Slot(s) | X::I(I::Slot(s), _) => Operand::Slot(s),
            X::Const(v) => Operand::Const(v),
            X::I(I::Const(c), t) => Operand::Const(Value::Int(c as i128, t)),
            X::I(I::Code(c), t) => Operand::Int(c, t),
            X::V(c) => Operand::Code(c),
        }
    }

    /// Evaluate to a `Value` (call arguments).
    #[inline(always)]
    fn eval(&self, it: &mut Interpreter, b: usize) -> R {
        match self {
            X::I(i, t) => Ok(Value::Int(i.get(it, b)? as i128, *t)),
            X::Slot(s) => Ok(it.rstack[b + s].clone()),
            X::Const(v) => Ok(v.clone()),
            X::V(c) => c(it, b),
        }
    }
}

fn is_cmp(op: BinaryOp) -> bool {
    matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge)
}

/// A condition for `if` / `while`: comparisons (and `&&`/`||`/`!` of them) yield a `bool` directly;
/// anything else must evaluate to a bool.
pub(super) fn cond(cx: &mut Cx, e: E, what: &'static str, deref: bool) -> Option<Cond> {
    match e {
        Expression::Binary { left, op, right, location } if is_cmp(*op) => {
            let (l, r) = (expr_eval(cx, left)?, expr_eval(cx, right)?);
            Some(match (l, r) {
                (X::I(l, _), X::I(r, _)) => typed::compare(*op, l, r, location.as_ref())?,
                (l, r) => fast::compare(l.operand(), r.operand(), *op, location.as_ref(), what)?,
            })
        }
        Expression::Binary { left, op: op @ (BinaryOp::And | BinaryOp::Or), right, .. } if boolish(left) && boolish(right) => {
            let (l, r) = (cond(cx, left, what, deref)?, cond(cx, right, what, deref)?);
            Some(if matches!(op, BinaryOp::And) {
                Box::new(move |it, b| Ok(l(it, b)? && r(it, b)?))
            } else {
                Box::new(move |it, b| Ok(l(it, b)? || r(it, b)?))
            })
        }
        Expression::Unary { op: UnaryOp::Not, operand, .. } if boolish(operand) => {
            let c = cond(cx, operand, what, deref)?;
            Some(Box::new(move |it, b| Ok(!c(it, b)?)))
        }
        _ => Some(fast::general(expr_eval(cx, e)?.code(), what, deref)),
    }
}

/// Always a `bool` when it evaluates (a comparison, or `&&`/`||`/`!` of such).
fn boolish(e: E) -> bool {
    match e {
        Expression::Binary { op, .. } if is_cmp(*op) => true,
        Expression::Binary { left, op: BinaryOp::And | BinaryOp::Or, right, .. } => boolish(left) && boolish(right),
        Expression::Unary { op: UnaryOp::Not, operand, .. } => boolish(operand),
        _ => false,
    }
}

fn exprs(cx: &mut Cx, list: impl IntoIterator<Item = E>) -> Option<Vec<Code>> {
    list.into_iter().map(|e| Some(expr(cx, e)?.code())).collect()
}

fn evals(cx: &mut Cx, list: impl IntoIterator<Item = E>) -> Option<Vec<Code>> {
    list.into_iter().map(|e| Some(expr_eval(cx, e)?.code())).collect()
}

/// `e` where the walker evaluates it with `eval` (references followed, no expected type): lowered
/// when the compiler can, else a walker island.
pub(super) fn expr_eval(cx: &mut Cx, e: E) -> Option<X> {
    if let Some(x) = expr(cx, e) {
        return Some(x);
    }
    let island = Island::new(cx, &format!("{:?}", e));
    Some(X::V(Box::new(move |it, b| island.run(it, b, |it| it.eval(e)))))
}

pub(super) fn expr(cx: &mut Cx, e: E) -> Option<X> {
    Some(match e {
        Expression::Literal { value, .. } => match cx.it.literal(value) {
            Value::Int(v, t) if small(t) && v as i64 as i128 == v => X::I(I::Const(v as i64), t),
            v => X::Const(v),
        },
        Expression::Identifier { name, location } => match cx.local(name) {
            Some(slot) if cx.is_boxed(slot) => {
                let name: &'static str = name.as_str();
                X::V(Box::new(move |it, b| island::read_boxed(it, b + slot, name)))
            }
            Some(slot) => match cx.slot_ty(slot) {
                Some(t) => X::I(I::Slot(slot), t),
                None => X::Slot(slot),
            },
            None => {
                let loc = location.as_ref();
                let name: &'static str = name.as_str();
                X::V(Box::new(move |it, _| {
                    it.mark(loc);
                    strip_refs(it.eval_path_value(name)?)
                }))
            }
        },
        Expression::Binary { left, op, right, location } if !matches!(op, BinaryOp::And | BinaryOp::Or) => {
            let (l, r) = (expr_eval(cx, left)?, expr_eval(cx, right)?);
            binary(l, *op, r, location.as_ref())
        }
        Expression::Binary { left, op, right, .. } => {
            let (l, r) = (expr_eval(cx, left)?.code(), expr_eval(cx, right)?.code());
            let is_and = matches!(op, BinaryOp::And);
            X::V(Box::new(move |it, b| match l(it, b)? {
                Value::Bool(x) if x != is_and => Ok(Value::Bool(x)),
                Value::Bool(_) => r(it, b),
                _ => Err(semantic(if is_and { "`&&` needs bool operands" } else { "`||` needs bool operands" })),
            }))
        }
        Expression::Unary { op, operand, location } => {
            if matches!(operand, Expression::Range { .. }) {
                return None;
            }
            let v = expr(cx, operand)?;
            let loc = location.as_ref();
            match (op, v) {
                (UnaryOp::Neg, X::I(i, t)) => X::I(I::Code(typed::negate(i, t, loc)), t),
                // integers are never references: `*x` / `&x` of one is itself
                (UnaryOp::Deref | UnaryOp::Ref, v @ X::I(..)) => v,
                (UnaryOp::Ref, v) => v,
                (UnaryOp::MutRef, _) => return None,
                (op, v) => {
                    let v = v.code();
                    X::V(match op {
                        UnaryOp::Neg => Box::new(move |it, b| {
                            let x = v(it, b)?;
                            it.mark(loc);
                            it.negate(x)
                        }),
                        UnaryOp::Not => Box::new(move |it, b| {
                            let x = v(it, b)?;
                            it.logical_not(x)
                        }),
                        _ => Box::new(move |it, b| deref_value(v(it, b)?)),
                    })
                }
            }
        }
        Expression::Cast { expr: inner, type_, location } => cast(cx, inner, type_, location.as_ref())?,
        Expression::Index { object, index, location } => {
            let (o, i) = (expr_eval(cx, object)?.code(), expr_eval(cx, index)?.code());
            let loc = location.as_ref();
            X::V(Box::new(move |it, b| {
                let obj = o(it, b)?;
                let idx = i(it, b)?;
                it.mark(loc);
                it.index_value(&obj, &idx)
            }))
        }
        Expression::FieldAccess { object, field, location } => {
            // `Enum.Variant` / `Type.assoc` spellings resolve as paths in the walker
            if let Expression::Identifier { name, .. } = object {
                if cx.local(name).is_none() {
                    return None;
                }
            }
            let o = expr_eval(cx, object)?.code();
            let loc = location.as_ref();
            let field: &'static str = field.as_str();
            X::V(Box::new(move |it, b| {
                let obj = o(it, b)?;
                it.mark(loc);
                it.field_of(&obj, field)
            }))
        }
        Expression::Tuple { elements, .. } => {
            let cs = evals(cx, elements.iter().copied())?;
            X::V(Box::new(move |it, b| Ok(Value::tuple(run_all(it, b, &cs)?))))
        }
        Expression::Array { elements, .. } => {
            let cs = evals(cx, elements.iter().copied())?;
            X::V(Box::new(move |it, b| Ok(Value::vec(run_all(it, b, &cs)?))))
        }
        Expression::Range { start, end, inclusive, .. } => {
            let (s, en) = (expr_eval(cx, start)?.code(), expr_eval(cx, end)?.code());
            X::V(range_code(s, en, *inclusive))
        }
        Expression::Block { statements, is_unsafe: false, .. } => X::V(block(cx, statements)?),
        Expression::MacroInvocation { name, args, is_repeat: false, location, .. } if name == "format" => {
            let cs = exprs(cx, args.iter().copied())?;
            let loc = location.as_ref();
            X::V(Box::new(move |it, b| {
                it.mark(loc);
                let vals = run_all(it, b, &cs)?;
                Ok(Value::str(it.format_args(&vals)?))
            }))
        }
        Expression::Call { function, arguments, location } => call(cx, function, arguments, location.as_ref())?,
        _ => return None,
    })
}

/// `l op r` (not `&&`/`||`): typed when both sides are typed integers.
fn binary(l: X, op: BinaryOp, r: X, loc: Loc) -> X {
    match (l, r) {
        (X::I(l, lt), X::I(r, rt)) if is_cmp(op) => {
            let c = typed::compare(op, l, r, loc).expect("comparison");
            let _ = (lt, rt);
            X::V(Box::new(move |it, b| Ok(Value::Bool(c(it, b)?))))
        }
        (X::I(l, lt), X::I(r, rt)) => match typed::arith(op, l, lt, r, rt, loc) {
            Some((c, t)) => X::I(I::Code(c), t),
            None => unreachable!("every non-logical, non-comparison operator is an integer operator"),
        },
        (l, r) => X::V(fast::binary(l.operand(), r.operand(), op, loc)),
    }
}

/// `start..end` / `start..=end` as a value.
pub(super) fn range_code(s: Code, en: Code, inclusive: bool) -> Code {
    Box::new(move |it, b| {
        let x = s(it, b)?;
        let y = en(it, b)?;
        match (&x, &y) {
            (Value::Int(a, at), Value::Int(c, ct)) => Ok(Value::Range(*a, if inclusive { *c + 1 } else { *c }, IntTy::unify(*at, *ct))),
            (Value::Char(a), Value::Char(c)) => {
                let hi = if inclusive { *c as u32 + 1 } else { *c as u32 };
                Ok(Value::vec((*a as u32..hi).filter_map(char::from_u32).map(Value::Char).collect()))
            }
            _ => Err(unsupported("range over non-integer values")),
        }
    })
}

/// `e as T`: a `small` integer target is typed (`cast` always yields that width or fails).
fn cast(cx: &mut Cx, inner: E, ty: &'static Type, loc: Loc) -> Option<X> {
    let target = IntTy::from_type(ty).filter(|t| small(*t));
    Some(match (expr_eval(cx, inner)?, target) {
        (X::I(i, _), Some(t)) => X::I(
            I::Code(Box::new(move |it, b| {
                let x = i.get(it, b)?;
                it.mark(loc);
                Ok(t.wrap(x as i128) as i64)
            })),
            t,
        ),
        (v, Some(t)) => {
            let v = v.code();
            X::I(
                I::Code(Box::new(move |it, b| {
                    let x = v(it, b)?;
                    it.mark(loc);
                    unbox(it.cast(x, ty)?, t)
                })),
                t,
            )
        }
        (v, None) => {
            let v = v.code();
            X::V(Box::new(move |it, b| {
                let x = v(it, b)?;
                it.mark(loc);
                it.cast(x, ty)
            }))
        }
    })
}

fn call(cx: &mut Cx, function: E, arguments: &'static [(Option<String>, E)], loc: Loc) -> Option<X> {
    let Expression::Identifier { name, .. } = function else { return None };
    if arguments.iter().any(|(label, _)| label.is_some()) {
        return None;
    }
    let arg_exprs = || arguments.iter().map(|(_, e)| *e);
    // a callable local (closure or function value)
    if let Some(slot) = cx.local(name) {
        if cx.is_boxed(slot) {
            return None;
        }
        let cs = exprs(cx, arg_exprs())?;
        return Some(X::V(Box::new(move |it, b| {
            it.mark(loc);
            let f = it.rstack[b + slot].clone();
            let args = run_all(it, b, &cs)?;
            strip_refs(it.call_value(&f, args)?)
        })));
    }
    // a module function (shadows intrinsics, as in the walker)
    if let Some(f) = cx.it.modules[cx.module].funcs.get(name.as_str()).cloned() {
        // a parameter the callee mutates would need a reference to our slot: walker only
        let mutated = cx.it.param_mutation(&f);
        if mutated.iter().any(|m| *m) || !f.type_params.is_empty() || f.params.first().is_some_and(|p| p.name == "self") {
            return None;
        }
        // arguments are plain values in the walker (`eval_raw_for_arg`), except an explicit `&mut x`
        if arg_exprs().any(|e| matches!(e, Expression::Unary { op: UnaryOp::MutRef, .. })) {
            return None;
        }
        let args: Vec<X> = arg_exprs().map(|e| expr_eval(cx, e)).collect::<Option<_>>()?;
        let ret = f.ret.and_then(IntTy::from_type).filter(|t| small(*t) && !f.is_async && !f.is_extern);
        let widths = args.iter().map(|a| if let X::I(_, u) = a { Some(*u) } else { None }).collect();
        let site = Site { f, args, widths, target: std::cell::OnceCell::new() };
        return Some(match ret {
            Some(t) => X::I(
                I::Code(Box::new(move |it, b| {
                    it.mark(loc);
                    match site.target(it) {
                        Some((c, bound)) => {
                            let base = fill_frame(it, b, c, &site.args)?;
                            it.run_compiled_int(&site.f, c, base, t, *bound)
                        }
                        None => unbox(site.walk(it, b)?, t),
                    }
                })),
                t,
            ),
            None => X::V(Box::new(move |it, b| {
                it.mark(loc);
                match site.target(it) {
                    Some((c, bound)) => {
                        let base = fill_frame(it, b, c, &site.args)?;
                        it.run_compiled_at(&site.f, c, base, *bound)
                    }
                    None => site.walk(it, b),
                }
            })),
        });
    }
    if cx.it.modules[cx.module].imports.contains_key(name.as_str()) {
        return None;
    }
    let intr = Intrinsic::from_call_name(name)?;
    if !matches!(intr, Intrinsic::Print | Intrinsic::Println | Intrinsic::Eprint | Intrinsic::Eprintln) {
        return None;
    }
    let cs = exprs(cx, arg_exprs())?;
    Some(X::V(Box::new(move |it, b| {
        it.mark(loc);
        let vals = run_all(it, b, &cs)?;
        let mut s = match vals.len() {
            0 => String::new(),
            1 => it.display_string(&vals[0])?,
            _ => it.format_args(&vals)?,
        };
        if intr.appends_newline() {
            s.push('\n');
        }
        if intr.writes_to_stderr() {
            it.write_err(&s);
        } else {
            it.write_out(&s);
        }
        Ok(Value::Unit)
    })))
}

/// A call site of a module function.
struct Site {
    f: Rc<FuncDef>,
    args: Vec<X>,
    /// Static width of each argument (`None`: not a typed integer).
    widths: Vec<Option<IntTy>>,
    /// The callee's compiled form, looked up on first call (it may be the function being compiled),
    /// and whether these arguments always arrive with the declared widths (no binding needed).
    target: std::cell::OnceCell<Option<(Rc<CompiledFn>, bool)>>,
}

impl Site {
    #[inline(always)]
    fn target(&self, it: &mut Interpreter) -> Option<&(Rc<CompiledFn>, bool)> {
        self.target
            .get_or_init(|| {
                let f = &self.f;
                let c = if it.engine_resolved && !f.is_extern && !f.is_async { it.compiled_for(f)? } else { return None };
                let bound = self.widths.len() == c.params.len() && self.widths.iter().zip(&c.param_int).all(|(w, p)| w.is_some() && w == p);
                Some((c, bound))
            })
            .as_ref()
    }

    /// The walker's call (the callee is not compiled).
    fn walk(&self, it: &mut Interpreter, b: usize) -> R {
        let vals = self.args.iter().map(|a| a.eval(it, b)).collect::<R<Vec<_>>>()?;
        it.call_func_g(&self.f, vals, None, &[], None)
    }
}

/// Open the callee's frame on `rstack`: the arguments are evaluated left to right (all of them, as
/// in the walker) and pushed as its first slots (the parameters), then the other slots are added.
/// Calls made while evaluating an argument use (and release) the stack above. Returns the base.
#[inline(always)]
fn fill_frame(it: &mut Interpreter, b: usize, c: &CompiledFn, args: &[X]) -> R<usize> {
    let base = it.rstack.len();
    let np = c.params.len();
    for (k, a) in args.iter().enumerate() {
        match a.eval(it, b) {
            Ok(v) if k < np => it.rstack.push(v),
            Ok(_) => {}
            Err(e) => {
                it.rstack.truncate(base);
                return Err(e);
            }
        }
    }
    it.rstack.resize(base + c.nslots, Value::Unit);
    Ok(base)
}

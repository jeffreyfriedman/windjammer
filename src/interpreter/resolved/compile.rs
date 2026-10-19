//! AST -> closure tree for the milestone-1 subset of `--engine=resolved`.
//!
//! Subset: parameters bound by name (no patterns, no `self`, no generics, not async/extern);
//! statements `let x [: T] = e` / `let _ = e`, assignment and compound assignment to a local,
//! `return`, `if`/`else`, `while`, `loop`, `for x in <iterable>`, `break [value]`, `continue`
//! (labels included), expression statements; expressions: literals, locals, paths (consts,
//! function values), unary/binary operators, `as`, indexing, field reads, tuples, arrays, ranges,
//! blocks, `format`/interpolation, `print`/`println`/`eprint`/`eprintln`, calls to user functions
//! and to callable locals. Anything else returns `None` and the function runs in the walker.

use super::fast::{self, Operand};
use super::{Code, CompiledFn};
use crate::interpreter::engine::{FuncDef, Interpreter};
use crate::interpreter::error::{semantic, unsupported, Ctl, R};
use crate::interpreter::eval_expr::stmt_loc;
use crate::interpreter::int::IntTy;
use crate::interpreter::place::{deref_value, strip_refs};
use crate::interpreter::value::*;
use crate::interpreter::intrinsics::Intrinsic;
use crate::parser::{BinaryOp, Expression, Pattern, Statement, Type, UnaryOp};
use std::rc::Rc;

type E = &'static Expression<'static>;
type S = &'static Statement<'static>;

struct Cx<'a> {
    it: &'a mut Interpreter,
    module: usize,
    scopes: Vec<Vec<(&'static str, usize)>>,
    nslots: usize,
}

impl Cx<'_> {
    fn local(&self, name: &str) -> Option<usize> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(n, _)| *n == name).map(|(_, i)| *i)
    }
    fn declare(&mut self, name: &'static str) -> usize {
        let i = self.nslots;
        self.nslots += 1;
        self.scopes.last_mut().expect("scope").push((name, i));
        i
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
    let mut cx = Cx { it, module: f.module, scopes: vec![Vec::new()], nslots: 0 };
    let params = f.params.iter().map(|p| cx.declare(p.name.as_str())).collect();
    let param_int = f.params.iter().map(|p| IntTy::from_type(&p.type_).filter(|t| *t != IntTy::Lit)).collect();
    let ret_int = f.ret.and_then(IntTy::from_type).filter(|t| *t != IntTy::Lit);
    let body = stmts(&mut cx, f.body)?;
    Some(CompiledFn { nslots: cx.nslots, params, param_int, ret_int, body })
}

/// A statement sequence in the current name scope; the value is the last statement's.
fn stmts(cx: &mut Cx, list: &'static [S]) -> Option<Code> {
    // the walker marks every statement's location before executing it
    let codes: Vec<(Option<&'static crate::source_map::Location>, Code)> =
        list.iter().map(|s| Some((stmt_loc(s), stmt(cx, s)?))).collect::<Option<_>>()?;
    Some(Box::new(move |it, b| {
        let Some(((lloc, lc), init)) = codes.split_last() else { return Ok(Value::Unit) };
        for (loc, c) in init {
            it.mark(*loc);
            c(it, b)?;
        }
        it.mark(*lloc);
        lc(it, b)
    }))
}

/// A nested block: its own name scope (slots are never reused, so no runtime work).
fn block(cx: &mut Cx, list: &'static [S]) -> Option<Code> {
    cx.scopes.push(Vec::new());
    let c = stmts(cx, list);
    cx.scopes.pop();
    c
}

fn stmt(cx: &mut Cx, s: S) -> Option<Code> {
    let code: Code = match s {
        Statement::Expression { expr, .. } => expr_code(cx, expr)?,
        Statement::Let { pattern, type_, value, else_block: None, .. } => {
            let v = expr_code(cx, value)?;
            let ty: Option<&'static Type> = type_.as_ref();
            match pattern {
                Pattern::Identifier(n) | Pattern::MutBinding(n) => {
                    let slot = cx.declare(n.as_str());
                    Box::new(move |it, b| {
                        let mut x = v(it, b)?;
                        if let Some(t) = ty {
                            x = it.coerce(x, t)?;
                        }
                        fast::set(it, b + slot, x);
                        Ok(Value::Unit)
                    })
                }
                Pattern::Wildcard => Box::new(move |it, b| {
                    let x = v(it, b)?;
                    if let Some(t) = ty {
                        it.coerce(x, t)?;
                    }
                    Ok(Value::Unit)
                }),
                _ => return None,
            }
        }
        Statement::Assignment { target, value, compound_op, .. } => {
            let Expression::Identifier { name, location } = target else { return None };
            let slot = cx.local(name)?;
            let v = expr_code(cx, value)?;
            let op = *compound_op;
            let tloc = location.as_ref();
            Box::new(move |it, b| {
                let rhs = v(it, b)?;
                it.mark(tloc);
                let new = match op {
                    None => retag(&it.rstack[b + slot], rhs),
                    Some(cop) => {
                        let bop = crate::interpreter::ops::compound_to_binary(cop);
                        if let Some(r) = fast::compound_int(it, b, slot, bop, &rhs) {
                            return r.map(|_| Value::Unit);
                        }
                        let cur = std::mem::replace(&mut it.rstack[b + slot], Value::Unit);
                        if let (Value::Str(_), Value::Str(add), crate::parser::CompoundOp::Add) = (&cur, &rhs, cop) {
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
                fast::set(it, b + slot, new);
                Ok(Value::Unit)
            })
        }
        Statement::Return { value, .. } => match value {
            Some(e) => {
                let v = expr_code(cx, e)?;
                Box::new(move |it, b| Err(Ctl::Return(v(it, b)?)))
            }
            None => Box::new(|_, _| Err(Ctl::Return(Value::Unit))),
        },
        Statement::If { condition, then_block, else_block, .. } => {
            let c = cond(cx, condition, "if", true)?;
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
            let c = cond(cx, condition, "while", false)?;
            let body = block(cx, body)?;
            let label: Option<&'static str> = label.as_deref();
            Box::new(move |it, b| {
                loop {
                    if !c(it, b)? {
                        break;
                    }
                    let r = body(it, b);
                    if crate::interpreter::eval_stmt::loop_exit(r, label)?.is_some() {
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
                let r = body(it, b);
                if let Some(v) = crate::interpreter::eval_stmt::loop_exit(r, label)? {
                    return Ok(v);
                }
            })
        }
        Statement::For { pattern, iterable, body, label, .. } => {
            let src = expr_code(cx, iterable)?;
            cx.scopes.push(Vec::new());
            let slot = match pattern {
                Pattern::Identifier(n) | Pattern::MutBinding(n) => Some(cx.declare(n.as_str())),
                Pattern::Wildcard => None,
                _ => {
                    cx.scopes.pop();
                    return None;
                }
            };
            let body = block(cx, body);
            cx.scopes.pop();
            let body = body?;
            let label: Option<&'static str> = label.as_deref();
            Box::new(move |it, b| {
                let v = src(it, b)?;
                if let Value::Range(lo, hi, ty) = v {
                    let mut i = lo;
                    while i < hi {
                        if let Some(s) = slot {
                            it.rstack[b + s] = Value::Int(i, ty);
                        }
                        if crate::interpreter::eval_stmt::loop_exit(body(it, b), label)?.is_some() {
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
                    if crate::interpreter::eval_stmt::loop_exit(body(it, b), label)?.is_some() {
                        break;
                    }
                }
                Ok(Value::Unit)
            })
        }
        Statement::Break { label, value, .. } => {
            let v = match value {
                Some(e) => Some(expr_code(cx, e)?),
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

/// The walker's `retag`: assigning a bare literal keeps the declared width of the old value.
fn retag(old: &Value, new: Value) -> Value {
    match (old, &new) {
        (Value::Int(_, ot), Value::Int(n, IntTy::Lit)) if *ot != IntTy::Lit && ot.fits(*n) => Value::Int(*n, *ot),
        (Value::Float(_, ot), Value::Float(n, FloatTy::Lit)) => Value::Float(*n, *ot),
        (Value::Float(_, ot), Value::Int(n, IntTy::Lit)) => Value::Float(*n as f64, *ot),
        _ => new,
    }
}

fn operand(cx: &mut Cx, e: E) -> Option<Operand> {
    Some(match e {
        Expression::Identifier { name, .. } if cx.local(name).is_some() => Operand::Slot(cx.local(name)?),
        Expression::Literal { value, .. } => Operand::Const(cx.it.literal(value)),
        _ => Operand::Code(expr_code(cx, e)?),
    })
}

/// A condition for `if` / `while`: comparisons are specialized, anything else must yield a bool.
fn cond(cx: &mut Cx, e: E, what: &'static str, deref: bool) -> Option<fast::Cond> {
    if let Expression::Binary { left, op, right, location } = e {
        if !matches!(op, BinaryOp::And | BinaryOp::Or) {
            let (l, r) = (operand(cx, left)?, operand(cx, right)?);
            if let Some(c) = fast::compare(l, r, *op, location.as_ref(), what) {
                return Some(c);
            }
            // not a comparison: rebuild the operands as a plain binary
            let (l, r) = (operand(cx, left)?, operand(cx, right)?);
            return Some(fast::general(fast::binary(l, r, *op, location.as_ref()), what, deref));
        }
    }
    Some(fast::general(expr_code(cx, e)?, what, deref))
}

fn exprs(cx: &mut Cx, list: impl IntoIterator<Item = E>) -> Option<Vec<Code>> {
    list.into_iter().map(|e| expr_code(cx, e)).collect()
}

fn run_all(it: &mut Interpreter, b: usize, codes: &[Code]) -> R<Vec<Value>> {
    let mut out = Vec::with_capacity(codes.len());
    for c in codes {
        out.push(c(it, b)?);
    }
    Ok(out)
}

fn expr_code(cx: &mut Cx, e: E) -> Option<Code> {
    Some(match e {
        Expression::Literal { value, .. } => {
            let v = cx.it.literal(value);
            Box::new(move |_, _| Ok(v.clone()))
        }
        Expression::Identifier { name, location } => match cx.local(name) {
            Some(slot) => Box::new(move |it, b| Ok(it.rstack[b + slot].clone())),
            None => {
                let loc = location.as_ref();
                let name: &'static str = name.as_str();
                Box::new(move |it, _| {
                    it.mark(loc);
                    strip_refs(it.eval_path_value(name)?)
                })
            }
        },
        Expression::Binary { left, op, right, location } if !matches!(op, BinaryOp::And | BinaryOp::Or) => {
            let (l, r) = (operand(cx, left)?, operand(cx, right)?);
            fast::binary(l, r, *op, location.as_ref())
        }
        Expression::Binary { left, op, right, location } => {
            let (l, r) = (expr_code(cx, left)?, expr_code(cx, right)?);
            let loc = location.as_ref();
            match op {
                BinaryOp::And | BinaryOp::Or => {
                    let is_and = matches!(op, BinaryOp::And);
                    Box::new(move |it, b| match l(it, b)? {
                        Value::Bool(x) if x != is_and => Ok(Value::Bool(x)),
                        Value::Bool(_) => r(it, b),
                        _ => Err(semantic(if is_and { "`&&` needs bool operands" } else { "`||` needs bool operands" })),
                    })
                }
                _ => {
                    let op = *op;
                    Box::new(move |it, b| {
                        let x = l(it, b)?;
                        let y = r(it, b)?;
                        it.mark(loc);
                        it.binary(op, x, y)
                    })
                }
            }
        }
        Expression::Unary { op, operand, location } => {
            if matches!(operand, Expression::Range { .. }) {
                return None;
            }
            let v = expr_code(cx, operand)?;
            let loc = location.as_ref();
            match op {
                UnaryOp::Neg => Box::new(move |it, b| {
                    let x = v(it, b)?;
                    it.mark(loc);
                    it.negate(x)
                }),
                UnaryOp::Not => Box::new(move |it, b| {
                    let x = v(it, b)?;
                    it.logical_not(x)
                }),
                UnaryOp::Deref => Box::new(move |it, b| deref_value(v(it, b)?)),
                UnaryOp::Ref => v,
                UnaryOp::MutRef => return None,
            }
        }
        Expression::Cast { expr, type_, location } => {
            let v = expr_code(cx, expr)?;
            let loc = location.as_ref();
            let ty: &'static Type = type_;
            Box::new(move |it, b| {
                let x = v(it, b)?;
                it.mark(loc);
                it.cast(x, ty)
            })
        }
        Expression::Index { object, index, location } => {
            let (o, i) = (expr_code(cx, object)?, expr_code(cx, index)?);
            let loc = location.as_ref();
            Box::new(move |it, b| {
                let obj = o(it, b)?;
                let idx = i(it, b)?;
                it.mark(loc);
                it.index_value(&obj, &idx)
            })
        }
        Expression::FieldAccess { object, field, location } => {
            // `Enum.Variant` / `Type.assoc` spellings resolve as paths in the walker
            if let Expression::Identifier { name, .. } = object {
                if cx.local(name).is_none() {
                    return None;
                }
            }
            let o = expr_code(cx, object)?;
            let loc = location.as_ref();
            let field: &'static str = field.as_str();
            Box::new(move |it, b| {
                let obj = o(it, b)?;
                it.mark(loc);
                it.field_of(&obj, field)
            })
        }
        Expression::Tuple { elements, .. } => {
            let cs = exprs(cx, elements.iter().copied())?;
            Box::new(move |it, b| Ok(Value::tuple(run_all(it, b, &cs)?)))
        }
        Expression::Array { elements, .. } => {
            let cs = exprs(cx, elements.iter().copied())?;
            Box::new(move |it, b| Ok(Value::vec(run_all(it, b, &cs)?)))
        }
        Expression::Range { start, end, inclusive, .. } => {
            let (s, en) = (expr_code(cx, start)?, expr_code(cx, end)?);
            let inclusive = *inclusive;
            Box::new(move |it, b| {
                let x = s(it, b)?;
                let y = en(it, b)?;
                match (&x, &y) {
                    (Value::Int(a, at), Value::Int(c, ct)) => {
                        Ok(Value::Range(*a, if inclusive { *c + 1 } else { *c }, IntTy::unify(*at, *ct)))
                    }
                    (Value::Char(a), Value::Char(c)) => {
                        let hi = if inclusive { *c as u32 + 1 } else { *c as u32 };
                        Ok(Value::vec((*a as u32..hi).filter_map(char::from_u32).map(Value::Char).collect()))
                    }
                    _ => Err(unsupported("range over non-integer values")),
                }
            })
        }
        Expression::Block { statements, is_unsafe: false, .. } => block(cx, statements)?,
        Expression::MacroInvocation { name, args, is_repeat: false, location, .. } if name == "format" => {
            let cs = exprs(cx, args.iter().copied())?;
            let loc = location.as_ref();
            Box::new(move |it, b| {
                it.mark(loc);
                let vals = run_all(it, b, &cs)?;
                Ok(Value::str(it.format_args(&vals)?))
            })
        }
        Expression::Call { function, arguments, location } => call(cx, function, arguments, location.as_ref())?,
        _ => return None,
    })
}

fn call(
    cx: &mut Cx,
    function: E,
    arguments: &'static [(Option<String>, E)],
    loc: Option<&'static crate::source_map::Location>,
) -> Option<Code> {
    let Expression::Identifier { name, .. } = function else { return None };
    if arguments.iter().any(|(label, _)| label.is_some()) {
        return None;
    }
    let arg_exprs = || arguments.iter().map(|(_, e)| *e);
    // a callable local (closure or function value)
    if let Some(slot) = cx.local(name) {
        let cs = exprs(cx, arg_exprs())?;
        return Some(Box::new(move |it, b| {
            it.mark(loc);
            let f = it.rstack[b + slot].clone();
            let args = run_all(it, b, &cs)?;
            strip_refs(it.call_value(&f, args)?)
        }));
    }
    // a module function (shadows intrinsics, as in the walker)
    if let Some(f) = cx.it.modules[cx.module].funcs.get(name.as_str()).cloned() {
        // a parameter the callee mutates would need a reference to our slot: walker only
        let mutated = cx.it.param_mutation(&f);
        if mutated.iter().any(|m| *m) || !f.type_params.is_empty() || f.params.first().is_some_and(|p| p.name == "self") {
            return None;
        }
        let cs = exprs(cx, arg_exprs())?;
        // the callee's compiled form, looked up on first call (it may be the function being compiled)
        let target: std::cell::OnceCell<Option<Rc<CompiledFn>>> = std::cell::OnceCell::new();
        return Some(Box::new(move |it, b| {
            it.mark(loc);
            let compiled = match target.get() {
                Some(c) => c.clone(),
                None => {
                    let c = if it.engine_resolved && !f.is_extern && !f.is_async { it.compiled_for(&f) } else { None };
                    let _ = target.set(c.clone());
                    c
                }
            };
            match compiled {
                Some(c) => {
                    // evaluate the arguments into a small buffer, then straight into the callee frame
                    let mut args: smallvec::SmallVec<[Value; 4]> = smallvec::SmallVec::new();
                    for a in &cs {
                        args.push(a(it, b)?);
                    }
                    it.run_compiled_with(&f, &c, args.len(), move |it, base, slots| {
                        for (slot, a) in slots.iter().zip(args) {
                            it.rstack[base + slot] = a;
                        }
                        Ok(())
                    })
                }
                None => {
                    let args = run_all(it, b, &cs)?;
                    it.call_func_g(&f, args, None, &[], None)
                }
            }
        }));
    }
    if cx.it.modules[cx.module].imports.contains_key(name.as_str()) {
        return None;
    }
    let intr = Intrinsic::from_call_name(name)?;
    if !matches!(intr, Intrinsic::Print | Intrinsic::Println | Intrinsic::Eprint | Intrinsic::Eprintln) {
        return None;
    }
    let cs = exprs(cx, arg_exprs())?;
    Some(Box::new(move |it, b| {
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
    }))
}


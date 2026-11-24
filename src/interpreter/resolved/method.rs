//! Method calls of `--engine=resolved`: `recv.method(args)` on a local or on a computed value,
//! with the receiver place, user-method lookup and builtin dispatch of the walker's
//! `eval_method_call` (`find_user_method`, `call_user_method`, `call_builtin_method`), and the
//! arguments compiled. A receiver local is boxed, so the walker slot is the place a mutating method
//! writes through.
//!
//! The walker passes `&mut` places for the arguments of a user method that mutates its parameters;
//! which method runs is only known from the receiver's value, so such a call (when an argument is a
//! place expression) takes the island path for the whole expression -- decided after reading the
//! receiver and before evaluating anything else, so nothing runs twice.

use super::compile::{Cx, E};
use super::fast::Cond;
use super::expr::{expr, expr_arg, X};
use super::island::Island;
use super::Code;
use super::CompiledFn;
use crate::interpreter::engine::{FuncDef, Interpreter};
use crate::interpreter::builtins::{Kind, MethodImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use crate::interpreter::error::R;
use crate::interpreter::place::deref_value;
use crate::interpreter::value::{Name, Place, Value};
use crate::interpreter::int::IntTy;
use crate::parser::{BinaryOp, Expression, Type, UnaryOp};
use crate::source_map::Location;

enum Recv {
    /// A (boxed) local: its walker slot is the place.
    Local(usize),
    /// A computed receiver (no place unless it evaluates to a reference).
    Value(Code),
}

pub(super) fn method_call(
    cx: &mut Cx,
    whole: E,
    object: E,
    method: &'static str,
    type_args: Option<&'static Vec<Type>>,
    arguments: &'static [(Option<String>, E)],
    loc: Option<&'static Location>,
) -> Option<X> {
    let generic = method_call_generic(cx, whole, object, method, type_args, arguments, loc)?;
    Some(len_fast(cx, object, method, arguments, generic))
}

fn method_call_generic(
    cx: &mut Cx,
    whole: E,
    object: E,
    method: &'static str,
    type_args: Option<&'static Vec<Type>>,
    arguments: &'static [(Option<String>, E)],
    loc: Option<&'static Location>,
) -> Option<X> {
    // `x.parse::<T>()` parser quirk, legacy `Type.f()` / `module.f()` spellings, `&mut` arguments
    if method.is_empty() || arguments.iter().any(|(_, a)| matches!(a, Expression::Unary { op: UnaryOp::MutRef, .. })) {
        return None;
    }
    let recv = match object {
        Expression::Identifier { name, .. } => {
            let s = cx.local(name)?;
            cx.need_box(s);
            Recv::Local(s)
        }
        // a field / index receiver is a place the walker resolves
        e if Interpreter::is_place_expr(e) => return None,
        e => Recv::Value(expr(cx, e)?.code()),
    };
    let place_args = arguments.iter().any(|(_, a)| Interpreter::is_place_expr(a));
    // the walker's whole-expression path, for a user method taking a place argument by `&mut`
    let fallback = match (&recv, place_args) {
        (_, false) => None,
        // a builtin map or set: no user method, so its arguments are only read (no box for a place argument)
        (Recv::Local(s), true) if cx.builtin_collection(*s) => None,
        (Recv::Local(_), true) => {
            // the receiver and place arguments may be taken by `&mut`: boxed. Other arguments the walker
            // only reads when none holds a closure or a call: they keep their slots
            let (places, reads): (Vec<E>, Vec<E>) = arguments.iter().map(|(_, a)| *a).partition(|a| Interpreter::is_place_expr(a));
            if !cx.it.fast_off("argcopy") && reads.iter().all(|a| super::mapfast::simple(a)) {
                Some(Island::with_copies(cx, &format!("{object:?} {places:?}"), &format!("{reads:?}")))
            } else {
                Some(Island::new(cx, &format!("{:?}", whole)))
            }
        }
        (Recv::Value(_), true) => return None,
    };
    let args: Vec<X> = arguments.iter().map(|(_, a)| expr_arg(cx, a)).collect::<Option<_>>()?;
    let hint: Option<Type> = type_args.and_then(|t| t.first().cloned());
    let cache = SiteCache::default();
    Some(X::R(Box::new(move |it, b| {
        let (place, recv) = match &recv {
            Recv::Local(s) => {
                let Value::Ref(boxed) = &it.rstack[b + s] else {
                    // a name a walker `let` did not bind: resolved as the walker would
                    let Expression::Identifier { name, .. } = object else { unreachable!() };
                    let v = super::island::read_boxed(it, b + s, name.as_str())?;
                    let vals = args.iter().map(|a| a.eval(it, b)).collect::<R<Vec<_>>>()?;
                    return it.call_method_value(v, method, vals, None);
                };
                let slot = boxed.slot.clone();
                let v = slot.borrow().clone();
                match v {
                    Value::Ref(p) => {
                        let place = it.chase(*p)?;
                        let recv = place.read()?;
                        (Some(place), recv)
                    }
                    // the walker slot itself is the place: reading it again would give `v`
                    v => (Some(Place::of_slot(slot)), v),
                }
            }
            Recv::Value(c) => {
                let v = c(it, b)?;
                match &v {
                    Value::Ref(p) => (Some(it.chase(*p.clone())?), v),
                    _ => (None, v),
                }
            }
        };
        if method == "clone" && matches!(recv, Value::Rc(_)) {
            return Ok(recv); // Rc::clone: another handle to the same allocation
        }
        let recv = deref_value(recv)?;
        let user = cache.user(it, &recv, method);
        if let (Some(f), Some(island)) = (&user, &fallback) {
            if it.param_mutation(f).iter().any(|m| *m) {
                drop(recv);
                return island.run(it, b, |it| it.eval_raw(whole));
            }
        }
        // `call_user_method` -> `call_func_run` for a compiled method with a `self` receiver: the
        // arguments go straight into the callee's parameter slots (after `self`)
        if let Some(f) = &user {
            if let Some(c) = cache.compiled(it, f) {
                let base = it.rstack.len();
                it.rstack.resize(base + c.nslots, Value::Unit);
                for (k, a) in args.iter().enumerate() {
                    match a.eval(it, b) {
                        Ok(v) if k + 1 < c.params.len() => it.rstack[base + c.params[k + 1]] = v,
                        Ok(_) => {}
                        Err(e) => {
                            it.rstack.truncate(base);
                            return Err(e);
                        }
                    }
                }
                it.mark(loc);
                it.type_hint = hint.clone();
                let self_val = match place {
                    Some(p) if it.self_mutation(f) => {
                        drop(recv);
                        Value::Ref(Box::new(p))
                    }
                    _ => recv,
                };
                it.rstack[base + c.params[0]] = self_val;
                return it.run_compiled_at(f, &c, base, false);
            }
        }
        let mut vals = Vec::with_capacity(args.len());
        for a in &args {
            vals.push(a.eval(it, b)?);
        }
        it.mark(loc);
        it.type_hint = hint.clone();
        if method == "into" && hint.is_none() {
            it.type_hint = it.take_expected(whole);
        }
        match user {
            Some(f) => it.call_user_method(&f, recv, place, vals),
            None if Interpreter::builtin_by_kind(&recv) && it.hot.is_none() => {
                let kind = Kind::of(&recv);
                let imp = cache.builtin(it, kind, method);
                it.run_builtin_method(imp, kind, recv, place, method, vals)
            }
            None => it.call_builtin_method(recv, place, method, vals),
        }
    })))
}

/// Inline cache of a method call site: the user method found for the last receiver type, and its
/// compiled form. Methods are fixed once a program is loaded, except under `--hot` (no caching).
#[derive(Default)]
struct SiteCache {
    ty: RefCell<Option<(Name, Option<Rc<FuncDef>>)>>,
    /// The builtin implementation for the last receiver kind (the registry is fixed once loaded).
    builtin: Cell<Option<(Kind, Option<MethodImpl>)>>,
    compiled: RefCell<Option<(Rc<FuncDef>, Option<Rc<CompiledFn>>)>>,
}

impl SiteCache {
    fn builtin(&self, it: &Interpreter, kind: Kind, method: &str) -> Option<MethodImpl> {
        match self.builtin.get() {
            Some((k, imp)) if k == kind => imp,
            _ => {
                let imp = it.builtins.method(kind, method);
                self.builtin.set(Some((kind, imp)));
                imp
            }
        }
    }

    fn user(&self, it: &mut Interpreter, recv: &Value, method: &str) -> Option<Rc<FuncDef>> {
        let name = |recv: &Value| match recv {
            Value::Struct(s) => Some(Name::Shared(s.name.clone())),
            Value::Enum(e) => Some(e.ty.clone()),
            _ => None,
        };
        let ty: &str = match recv {
            Value::Struct(s) => &s.name,
            Value::Enum(e) => &e.ty,
            _ => return it.find_user_method(recv, method),
        };
        if it.hot.is_some() {
            return it.find_user_method(recv, method);
        }
        if let Some((t, f)) = &*self.ty.borrow() {
            // interned names: the same text at the same address is the common case
            if std::ptr::eq(t.as_ptr(), ty.as_ptr()) && t.len() == ty.len() || **t == *ty {
                return f.clone();
            }
        }
        let f = it.find_user_method(recv, method);
        *self.ty.borrow_mut() = name(recv).map(|n| (n, f.clone()));
        f
    }

    /// The compiled form of a method the walker would run compiled (`call_func_run` with a receiver).
    fn compiled(&self, it: &mut Interpreter, f: &Rc<FuncDef>) -> Option<Rc<CompiledFn>> {
        if !it.engine_resolved || it.hot.is_some() || f.is_async || f.is_extern || !super::compile::has_self(f) {
            return None;
        }
        if let Some((g, c)) = &*self.compiled.borrow() {
            if Rc::ptr_eq(g, f) {
                return c.clone();
            }
        }
        let c = it.compiled_for(f);
        *self.compiled.borrow_mut() = Some((f.clone(), c.clone()));
        c
    }
}

/// `s.len()` on a local string, vector or map when no user type has a `len` method: the length read in place,
/// without the receiver's place, clone and method dispatch (the builtins' `Value::usize`). Anything else, a
/// box in use or a lock guard takes the generic call.
fn len_fast(cx: &mut Cx, object: E, method: &str, arguments: &[(Option<String>, E)], generic: X) -> X {
    let Some(slot) = len_receiver(cx, object, method, arguments) else { return generic };
    let generic = generic.code();
    X::R(Box::new(move |it, b| {
        if !it.fast_off("lenfast") {
            if let Some(n) = local_len(&it.rstack[b + slot]) {
                it.fast.lenfast += 1;
                return Ok(Value::usize(n));
            }
        }
        generic(it, b)
    }))
}

/// The slot of `x` in `x.len()` when no user type has a `len` method (the shape `len_fast` reads in place).
fn len_receiver(cx: &Cx, object: E, method: &str, arguments: &[(Option<String>, E)]) -> Option<usize> {
    let Expression::Identifier { name, .. } = object else { return None };
    let slot = cx.local(name)?;
    if method != "len" || !arguments.is_empty() || cx.it.methods.keys().any(|(_, m)| m == "len") {
        return None;
    }
    Some(slot)
}

/// The length held by a local (directly or through a plain box), if it is a string, vector or map.
#[inline(always)]
fn local_len(v: &Value) -> Option<usize> {
    match v {
        Value::Ref(p) if p.path.is_empty() && p.guard.is_none() => match p.slot.try_borrow() {
            Ok(v) => length(&v),
            Err(_) => None,
        },
        v => length(v),
    }
}

/// `x.len() <op> k` (a condition over a local's length and an integer constant), in one step: the length
/// is read in place and compared as the generic condition `fallback` would (`usize` against `k`, the
/// location marked after both operands), with no `Value` built for the length. Any other receiver value
/// (or `u128` ordering) runs `fallback`, which evaluates the whole comparison itself, so nothing runs twice.
pub(super) fn len_cmp(cx: &Cx, left: E, op: BinaryOp, k: (i128, IntTy), loc: Option<&'static Location>, fallback: Cond) -> Cond {
    let Expression::MethodCall { object, method, type_args: None, arguments, .. } = left else { return fallback };
    let Some(slot) = len_receiver(cx, object, method, arguments) else { return fallback };
    Box::new(move |it, b| {
        if !it.fast_off("lencmp") {
            if let Some(n) = local_len(&it.rstack[b + slot]) {
                if let Some(c) = super::fast::int_cmp(op, n as i128, IntTy::Usize, k.0, k.1) {
                    it.mark(loc);
                    it.fast.lenfast += 1;
                    it.fast.lencmp += 1;
                    return Ok(c);
                }
            }
        }
        fallback(it, b)
    })
}

/// The length of a string (bytes), vector or map held directly.
fn length(v: &Value) -> Option<usize> {
    match v {
        Value::Str(s) => Some(s.len()),
        Value::Vec(xs) => Some(xs.len()),
        Value::Map(m) => Some(m.len()),
        _ => None,
    }
}

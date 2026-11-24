//! Closure bodies in `--engine=resolved`. A closure value is still the walker's `Callable::Closure`
//! (its environment is the creating scope's walker slots); calling it runs a compiled form of its
//! body when there is one: the parameters are locals, and each name of the body that the
//! environment binds is a boxed local holding that very environment slot, so reads, writes,
//! borrows and nested closures see the walker's storage. Names the environment does not bind
//! resolve as the walker resolves them (globals, functions).
//!
//! The compiled form is per body and per environment shape (the names of the creating scope, which
//! are the same at every creation from one site), so each free name is a fixed position in the
//! environment. No environment slot is held beyond a call (a sender a thread's closure captured
//! must die with it, as in the walker).

use super::compile::{fixpoint, Unit, E};
use super::expr::expr_eval;
use super::Code;
use crate::interpreter::engine::{Interpreter, Scope};
use crate::interpreter::error::{Ctl, R};
use crate::interpreter::value::{Callable, Place, Slot, Value};
use std::collections::HashSet;
use std::rc::Rc;

pub(crate) struct CompiledClosure {
    /// The environment's variable names this form was compiled for (in order).
    env_names: Vec<&'static str>,
    nslots: usize,
    /// Slot of each parameter (`None`: `_`).
    params: Vec<Option<usize>>,
    /// Environment names the body uses: (position in the environment, slot).
    free: Vec<(usize, usize)>,
    /// Per `free` name: its slot is plain and takes a copy of the environment value at the call (`by_value`).
    free_by_value: Vec<bool>,
    /// The parameters' boxed slots: lock guards they hold are poisoned on a panic. A parameter the
    /// body only reads is a plain slot (no walker slot per call); it takes plain values only.
    boxed: Vec<usize>,
    body: Code,
    /// No boxed parameter slot and no free name: the guard loops of a call have nothing to check or bind.
    simple: bool,
    /// The closure value whose environment last matched `env_names`. A weak reference keeps its allocation
    /// (never its environment) alive, so no other closure can appear at that address and pass unchecked.
    checked: std::cell::RefCell<std::rc::Weak<Callable>>,
}

/// A closure expression in a compiled function, created without a walker island: its environment is the walker slots
/// of the locals its body names (boxed, so the closure and the function share them), not every local of the scope as
/// the walker's capture holds. Reads and writes through the environment are the same (a name the body does not use
/// is never looked up); only the boxing changes: the function's other locals stay plain slots.
pub(super) fn create(cx: &mut super::compile::Cx, parameters: &'static [String], body: E) -> Option<super::expr::X> {
    if cx.closure_body || cx.it.fast_off("closurecreate") {
        return None;
    }
    let text = format!("{:?}", body);
    let used: HashSet<&str> = text.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect();
    let free = cx.visible_locals(&used);
    for (_, s) in &free {
        cx.need_box(*s);
    }
    let (module, self_ty) = (cx.module, cx.self_ty.clone());
    Some(super::expr::X::V(Box::new(move |it, b| {
        it.closure_counter += 1;
        it.fast.closurecreate += 1;
        let mut env: Vec<(&'static str, Slot)> = Vec::with_capacity(free.len());
        for (n, s) in &free {
            if let Value::Ref(p) = &it.rstack[b + s] {
                env.push((n, p.slot.clone()));
            }
        }
        Ok(Value::Func(Rc::new(Callable::Closure { params: parameters, body, env, module, self_ty: self_ty.clone(), tparams: Vec::new() })))
    })))
}

/// `|x|`, `|x: T|`, `|&x|`, `|mut x|`: the bound name (`None`: a tuple pattern, the walker's).
fn param_name(p: &'static str) -> Option<&'static str> {
    let p = p.split(':').next().unwrap_or(p).trim().trim_start_matches('&').trim();
    let p = p.strip_prefix("mut ").unwrap_or(p).trim();
    (!p.starts_with('(') && !p.is_empty()).then_some(p)
}

fn compile(it: &mut Interpreter, params: &'static [String], body: E, env: &[(&'static str, Slot)], module: usize, self_ty: Option<Rc<str>>) -> Option<CompiledClosure> {
    let names: Vec<&'static str> = params.iter().map(|p| param_name(p.as_str())).collect::<Option<_>>()?;
    let text = format!("{:?}", body);
    let used: HashSet<&str> = text.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect();
    // environment names the body may use (the walker looks names up from the end: last wins)
    let mut free: Vec<&'static str> = Vec::new();
    for (n, _) in env.iter().rev() {
        if used.contains(n) && !free.contains(n) {
            free.push(n);
        }
    }
    free.reverse();
    let unit = Unit { module, ret: None, self_ty, tail_hint: false, closure_body: true };
    // environment names are boxed (their slots are the environment's). A parameter is boxed only
    // when the body needs its walker slot (islands, borrows, captures); a plain one is given plain
    // values only: a call passing a reference to it (an `iter_mut` item) runs in the walker
    // A body that is one expression with no call, closure, assignment, block or macro cannot change an environment
    // slot while it runs: its free names may be plain slots holding a copy of the value at the call (no reference
    // boxed per name per call). Whatever still needs a walker slot (an island naming it) is boxed by the fixpoint.
    let by_value = !it.fast_off("closurecopy")
        && !["Call {", "MethodCall {", "Closure {", "Assignment", "MacroInvocation", "Block {", "Await", "Spawn", "Async", "Scope {", "Select {"].iter().any(|w| text.contains(w));
    let boxed_init: HashSet<usize> = if by_value { HashSet::new() } else { (0..free.len()).collect() };
    let (nslots, pslots, body_code, boxed, free_boxed) = fixpoint(it, &unit, boxed_init, |cx| {
        let free_slots: Vec<usize> = free.iter().map(|n| cx.declare_local(n)).collect();
        debug_assert!(free_slots.iter().enumerate().all(|(i, s)| i == *s));
        cx.enter();
        let pslots: Vec<Option<usize>> = names.iter().map(|n| (*n != "_").then(|| cx.declare_local(n))).collect();
        let code = expr_eval(cx, body)?.code();
        let boxed: Vec<usize> = cx.boxed_here();
        cx.leave();
        let free_boxed: Vec<bool> = free_slots.iter().map(|s| cx.is_boxed(*s)).collect();
        Some((cx.nslots(), pslots, code, boxed, free_boxed))
    })?;
    Some(CompiledClosure {
        env_names: env.iter().map(|(n, _)| *n).collect(),
        nslots,
        params: pslots,
        // the walker looks a name up from the end of the scope: its last binding
        free: free.iter().enumerate().map(|(i, n)| (env.iter().rposition(|(m, _)| m == n).expect("free name is in the environment"), i)).collect(),
        free_by_value: free_boxed.iter().map(|b| !b).collect(),
        simple: boxed.is_empty() && free.is_empty(),
        boxed,
        body: body_code,
        checked: std::cell::RefCell::new(std::rc::Weak::new()),
    })
}

impl Interpreter {
    /// `call_callable` for a closure, compiled; `None` = run it in the walker.
    pub(crate) fn call_compiled_closure(&mut self, c: &Rc<Callable>, args: &mut [Value]) -> Option<R> {
        let Callable::Closure { params, body, env, module, self_ty, tparams } = &**c else { return None };
        if !self.engine_resolved || !tparams.is_empty() || !self.drop_types.is_empty() || self.hot.is_some() {
            return None;
        }
        let key = *body as *const _ as usize;
        // the closures of a loop (an iterator chain calls each in turn) hit a direct-mapped cache: no hash probe
        let slot = (key >> 4) & 7;
        let cc = match &self.closure_cache[slot] {
            Some((k, cc)) if *k == key => cc.clone(),
            _ => {
                let cc = match self.compiled_closures.get(&key) {
                    Some(cc) => cc.clone(),
                    None => {
                        let cc = compile(self, params, body, env, *module, self_ty.clone()).map(Rc::new);
                        self.compiled_closures.insert(key, cc.clone());
                        cc
                    }
                }?;
                self.closure_cache[slot] = Some((key, cc.clone()));
                cc
            }
        };
        // the environment shape is checked once per closure value (its environment never changes shape)
        if std::ptr::eq(cc.checked.borrow().as_ptr(), Rc::as_ptr(c)) && !self.fast_off("closureenv") {
            self.fast.closureenv += 1;
        } else {
            if cc.env_names.len() != env.len() || cc.env_names.iter().zip(env.iter()).any(|(a, (b, _))| a != b) {
                return None; // another environment shape: the walker
            }
            *cc.checked.borrow_mut() = Rc::downgrade(c);
        }
        if let Err(e) = self.depth_check() {
            return Some(Err(e));
        }
        // a reference for a plain parameter: the walker binds it (reads follow it there)
        let simple = cc.simple;
        if cc.params.iter().zip(args.iter()).any(|(p, a)| matches!(p, Some(s) if simple || !cc.boxed.contains(s)) && matches!(a, Value::Ref(_))) {
            return None;
        }
        // a by-value name whose environment value is a reference (or in use) takes the walker, as a plain parameter does
        if !simple {
            for ((pos, _), v) in cc.free.iter().zip(&cc.free_by_value) {
                if *v && env[*pos].1.try_borrow().map_or(true, |x| matches!(&*x, Value::Ref(_))) {
                    return None;
                }
            }
        }
        let base = self.rstack.len();
        self.rstack.reserve(cc.nslots);
        for _ in 0..cc.nslots {
            self.rstack.push(Value::Unit);
        }
        if !simple {
            for ((pos, s), v) in cc.free.iter().zip(&cc.free_by_value) {
                self.rstack[base + s] = if *v {
                    self.fast.closurecopy += 1;
                    env[*pos].1.borrow().clone()
                } else {
                    Value::Ref(Box::new(Place::of_slot(env[*pos].1.clone())))
                };
            }
        }
        for (i, p) in cc.params.iter().enumerate() {
            let a = if i < args.len() { std::mem::replace(&mut args[i], Value::Unit) } else { Value::Unit };
            if let Some(s) = p {
                self.rstack[base + s] = if !simple && cc.boxed.contains(s) { super::island::boxed(a) } else { a };
            }
        }
        // the walker's fresh scope for the body; when the caller's scope is already exactly that (compiled
        // code runs in an empty scope of its module), the body runs in it: islands install their own and
        // nothing compiled leaves a variable, mark or defer behind
        let s = &self.scope;
        let same = s.module == *module && s.self_ty == *self_ty && s.ret_ty.is_none() && s.vars.is_empty() && s.marks.is_empty() && s.defers.is_empty() && s.tparams.is_empty();
        let swap = !same || self.fast_off("closurescope");
        if swap {
            let mut scope = Scope::new(*module);
            scope.self_ty = self_ty.clone();
            let saved = std::mem::replace(&mut self.scope, scope);
            self.frames.push(saved);
        } else {
            self.fast.closurescope += 1;
        }
        self.depth += 1;
        let r = match (cc.body)(self, base) {
            Err(Ctl::Return(v)) => Ok(v),
            other => other,
        };
        // the walker's `exit_scope_vars`: a panic poisons the lock guards the closure's own variables hold
        if !simple && matches!(&r, Err(Ctl::Err(e)) if e.kind == crate::interpreter::error::ErrorKind::Panic) {
            for s in &cc.boxed {
                if let Value::Ref(p) = &self.rstack[base + s] {
                    crate::interpreter::scope::poison_if_guard(&p.slot.borrow());
                }
            }
        }
        // pop the window; a slot holding a scalar owns nothing, so it skips the destructor call
        while self.rstack.len() > base {
            match self.rstack.pop() {
                Some(v @ (Value::Int(..) | Value::Bool(_) | Value::Unit | Value::Float(..) | Value::Char(_))) => std::mem::forget(v),
                Some(v) => drop(v),
                None => break,
            }
        }
        self.depth -= 1;
        if swap {
            self.scope = self.frames.pop().expect("frame stack");
        }
        Some(r)
    }
}

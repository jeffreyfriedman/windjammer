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
    /// Environment names the body uses: (position in the environment, boxed slot).
    free: Vec<(usize, usize)>,
    /// The parameters' boxed slots: lock guards they hold are poisoned on a panic. A parameter the
    /// body only reads is a plain slot (no walker slot per call); it takes plain values only.
    boxed: Vec<usize>,
    body: Code,
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
    let unit = Unit { module, ret: None, self_ty, tail_hint: false };
    // environment names are boxed (their slots are the environment's). A parameter is boxed only
    // when the body needs its walker slot (islands, borrows, captures); a plain one is given plain
    // values only: a call passing a reference to it (an `iter_mut` item) runs in the walker
    let boxed_init: HashSet<usize> = (0..free.len()).collect();
    let (nslots, pslots, body_code, boxed) = fixpoint(it, &unit, boxed_init, |cx| {
        let free_slots: Vec<usize> = free.iter().map(|n| cx.declare_local(n)).collect();
        debug_assert!(free_slots.iter().enumerate().all(|(i, s)| i == *s));
        cx.enter();
        let pslots: Vec<Option<usize>> = names.iter().map(|n| (*n != "_").then(|| cx.declare_local(n))).collect();
        let code = expr_eval(cx, body)?.code();
        let boxed: Vec<usize> = cx.boxed_here();
        cx.leave();
        Some((cx.nslots(), pslots, code, boxed))
    })?;
    Some(CompiledClosure {
        env_names: env.iter().map(|(n, _)| *n).collect(),
        nslots,
        params: pslots,
        // the walker looks a name up from the end of the scope: its last binding
        free: free.iter().enumerate().map(|(i, n)| (env.iter().rposition(|(m, _)| m == n).expect("free name is in the environment"), i)).collect(),
        boxed,
        body: body_code,
    })
}

impl Interpreter {
    /// `call_callable` for a closure, compiled; `None` = run it in the walker.
    pub(crate) fn call_compiled_closure(&mut self, c: &Rc<Callable>, args: &mut Vec<Value>) -> Option<R> {
        let Callable::Closure { params, body, env, module, self_ty, tparams } = &**c else { return None };
        if !self.engine_resolved || !tparams.is_empty() || !self.drop_types.is_empty() || self.hot.is_some() {
            return None;
        }
        let key = *body as *const _ as usize;
        let cc = match self.compiled_closures.get(&key) {
            Some(cc) => cc.clone(),
            None => {
                let cc = compile(self, params, body, env, *module, self_ty.clone()).map(Rc::new);
                self.compiled_closures.insert(key, cc.clone());
                cc
            }
        }?;
        if cc.env_names.len() != env.len() || cc.env_names.iter().zip(env.iter()).any(|(a, (b, _))| a != b) {
            return None; // another environment shape: the walker
        }
        if let Err(e) = self.depth_check() {
            return Some(Err(e));
        }
        // a reference for a plain parameter: the walker binds it (reads follow it there)
        if cc.params.iter().zip(args.iter()).any(|(p, a)| matches!(p, Some(s) if !cc.boxed.contains(s)) && matches!(a, Value::Ref(_))) {
            return None;
        }
        let base = self.rstack.len();
        self.rstack.resize(base + cc.nslots, Value::Unit);
        for (pos, s) in &cc.free {
            self.rstack[base + s] = Value::Ref(Box::new(Place::of_slot(env[*pos].1.clone())));
        }
        for (i, p) in cc.params.iter().enumerate() {
            let a = if i < args.len() { std::mem::replace(&mut args[i], Value::Unit) } else { Value::Unit };
            if let Some(s) = p {
                self.rstack[base + s] = if cc.boxed.contains(s) { super::island::boxed(a) } else { a };
            }
        }
        let mut scope = Scope::new(*module);
        scope.self_ty = self_ty.clone();
        let saved = std::mem::replace(&mut self.scope, scope);
        self.frames.push(saved);
        self.depth += 1;
        let r = match (cc.body)(self, base) {
            Err(Ctl::Return(v)) => Ok(v),
            other => other,
        };
        // the walker's `exit_scope_vars`: a panic poisons the lock guards the closure's own variables hold
        if matches!(&r, Err(Ctl::Err(e)) if e.kind == crate::interpreter::error::ErrorKind::Panic) {
            for s in &cc.boxed {
                if let Value::Ref(p) = &self.rstack[base + s] {
                    crate::interpreter::scope::poison_if_guard(&p.slot.borrow());
                }
            }
        }
        self.rstack.truncate(base);
        self.depth -= 1;
        self.scope = self.frames.pop().expect("frame stack");
        Some(r)
    }
}

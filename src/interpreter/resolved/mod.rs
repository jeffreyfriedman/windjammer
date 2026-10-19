//! `--engine=resolved` (ADR-0020 option B): functions compiled to closure trees with
//! slot-indexed locals.
//!
//! A function is compiled the first time it is called, or never: every construct outside the
//! subset (see `compile.rs`, `expr.rs`) makes the whole function run in the tree walker, so the two
//! engines never disagree about what is supported. The compiled code calls the same helpers as the
//! walker (`binary`, `coerce`, `format_args`, `call_func_g`...), marks the same source locations and
//! keeps the same frame/backtrace discipline, so values, panics and error spans are identical.
//!
//! Locals live in `Interpreter::rstack` (one window per active compiled call), which reachability
//! (`heap.rs`) walks as a root and the scheduler swaps per task. Programs with destructors
//! (`impl Drop`) are not compiled: scope-exit drops need the walker's slots. Integer expressions of a
//! statically known width run unboxed (`typed.rs`).

mod compile;
mod expr;
mod fast;
mod island;
mod method;
mod typed;

use super::engine::{FuncDef, Interpreter, Scope};
use super::error::{semantic, Ctl, R};
use super::int::IntTy;
use super::place::deref_value;
use super::value::Value;
use std::rc::Rc;

/// Compiled code: runs against the interpreter and the base index of its frame in `rstack`.
pub(crate) type Code = Box<dyn Fn(&mut Interpreter, usize) -> R>;

pub(crate) enum Body {
    V(Code),
    /// A typed body: the tail's width, then the declared return width (every `return` is typed too).
    I(typed::ICode, IntTy, IntTy),
}

pub(crate) struct CompiledFn {
    nslots: usize,
    /// Slots of the parameters, in order (always `0..n`: parameters are declared first).
    params: Vec<usize>,
    /// Integer width of each parameter / the return type, when declared (skip `coerce` for exact matches).
    param_int: Vec<Option<IntTy>>,
    /// Parameters whose slot is typed: their argument must be an integer (else the call deopts).
    param_typed: Vec<bool>,
    ret_int: Option<IntTy>,
    /// Every slot is typed: once the parameters are bound the frame holds only integers (and `Unit`).
    scalar: bool,
    /// Replay the walker's `expect_tail_type` at entry.
    tail_hint: bool,
    body: Body,
}

impl Interpreter {
    /// The compiled form of `f`, compiling it on first use; `None` = run it in the walker.
    pub(crate) fn compiled_for(&mut self, f: &Rc<FuncDef>) -> Option<Rc<CompiledFn>> {
        let key = Rc::as_ptr(f) as usize;
        if let Some((_, c)) = self.compiled.get(&key) {
            return c.clone();
        }
        let c = if self.drop_types.is_empty() { compile::compile_fn(self, f).map(Rc::new) } else { None };
        self.compiled.insert(key, (f.clone(), c.clone()));
        c
    }

    /// `call_func_run` for a compiled function: same frame, backtrace, binding and return rules.
    pub(crate) fn run_compiled(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, args: Vec<Value>) -> R {
        let base = self.rstack.len();
        self.rstack.resize(base + c.nslots, Value::Unit);
        for (slot, a) in c.params.iter().zip(args) {
            self.rstack[base + slot] = a;
        }
        self.run_compiled_at(f, c, base, false)
    }

    /// Run `c` on the frame at `base`, whose parameter slots already hold the arguments (`bound`:
    /// already of the declared widths, see `expr::call`).
    pub(crate) fn run_compiled_at(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, base: usize, bound: bool) -> R {
        let own = self.enter_compiled(f, base)?;
        let r = match if bound { Ok(true) } else { self.bind_params(f, c, base) } {
            Ok(true) if c.tail_hint => {
                self.expect_tail_type(f);
                self.run_body(f, c, base)
            }
            Ok(true) => self.run_body(f, c, base),
            Ok(false) => {
                let args = self.deopt_args(c, base, own);
                return self.call_func_walk(f, args, None, &[], None);
            }
            Err(e) => {
                let r = self.finish_value(f, c, Err(e));
                self.leave_compiled(base, own, false);
                return r;
            }
        };
        self.leave_compiled(base, own, c.scalar);
        r
    }

    #[inline(always)]
    fn run_body(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, base: usize) -> R {
        match &c.body {
            Body::V(code) => {
                let r = code(self, base);
                self.finish_value(f, c, r)
            }
            Body::I(code, u, t) => {
                let r = code(self, base);
                self.finish_int(f, r, *u, *t).map(|v| Value::Int(v as i128, *t))
            }
        }
    }

    /// `run_compiled_at` for a function declared to return the `small` width `t`: the result unboxed.
    pub(crate) fn run_compiled_int(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, base: usize, t: IntTy, bound: bool) -> R<i64> {
        let own = self.enter_compiled(f, base)?;
        let r = match if bound { Ok(true) } else { self.bind_params(f, c, base) } {
            Ok(true) => match &c.body {
                _ if c.tail_hint => {
                    self.expect_tail_type(f);
                    self.run_body(f, c, base).and_then(|v| typed::unbox(v, t))
                }
                Body::I(code, u, _) => {
                    let r = code(self, base);
                    self.finish_int(f, r, *u, t)
                }
                Body::V(code) => {
                    let r = code(self, base);
                    self.finish_value(f, c, r).and_then(|v| typed::unbox(v, t))
                }
            },
            Ok(false) => {
                let args = self.deopt_args(c, base, own);
                return self.call_func_walk(f, args, None, &[], None).and_then(|v| typed::unbox(v, t));
            }
            Err(e) => {
                let r = self.finish_value(f, c, Err(e)).and_then(|v| typed::unbox(v, t));
                self.leave_compiled(base, own, false);
                return r;
            }
        };
        self.leave_compiled(base, own, c.scalar);
        r
    }

    /// Depth check, backtrace and (when the caller's scope is not reusable) a walker frame.
    #[inline(always)]
    fn enter_compiled(&mut self, f: &Rc<FuncDef>, base: usize) -> R<bool> {
        if let Err(e) = self.depth_check() {
            self.rstack.truncate(base);
            return Err(e);
        }
        self.backtrace.push((f.qual.clone(), self.cur));
        // a compiled caller's scope is empty and in the same module: no need for a new walker frame
        let s = &self.scope;
        let own = !(s.vars.is_empty() && s.marks.is_empty() && s.module == f.module && s.defers.is_empty() && s.tparams.is_empty() && s.self_ty.is_none());
        if own {
            let mut scope = Scope::new(f.module);
            scope.ret_ty = f.ret;
            let saved = std::mem::replace(&mut self.scope, scope);
            self.frames.push(saved);
        }
        self.depth += 1;
        Ok(own)
    }

    /// `scalar`: the frame holds no value with a destructor (popped without drop glue).
    #[inline(always)]
    fn leave_compiled(&mut self, base: usize, own: bool, scalar: bool) {
        if scalar {
            while self.rstack.len() > base {
                std::mem::forget(self.rstack.pop());
            }
        } else {
            self.rstack.truncate(base);
        }
        self.depth -= 1;
        if own {
            self.scope = self.frames.pop().expect("frame stack");
        }
        self.backtrace.pop();
    }

    /// A typed parameter received a non-integer: unwind the frame and hand the arguments to the walker.
    #[cold]
    fn deopt_args(&mut self, c: &CompiledFn, base: usize, own: bool) -> Vec<Value> {
        let args = c.params.iter().map(|s| std::mem::replace(&mut self.rstack[base + s], Value::Unit)).collect();
        self.leave_compiled(base, own, false);
        args
    }

    /// The walker's result rules: `return` ends the call, the value is coerced to the return type.
    fn finish_value(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, r: R) -> R {
        let r = match r {
            Err(Ctl::Return(v)) => Ok(v),
            Err(Ctl::Break(_)) | Err(Ctl::Continue(_)) => Err(semantic("`break`/`continue` outside of a loop")),
            other => other,
        };
        match r {
            Ok(v) => match (f.ret, &c.ret_int) {
                (None, _) => Ok(Value::Unit),
                (Some(_), Some(t)) if matches!(&v, Value::Int(_, vt) if vt == t) => Ok(v),
                (Some(t), _) => deref_value(v).and_then(|v| self.coerce(v, t)),
            },
            Err(Ctl::Err(e)) => Err(Ctl::Err(Box::new(self.annotate(*e)))),
            other => other,
        }
    }

    /// `finish_value` for a typed body whose tail has width `u`, returning width `t`.
    fn finish_int(&mut self, f: &Rc<FuncDef>, r: R<i64>, u: IntTy, t: IntTy) -> R<i64> {
        let r = match r {
            Ok(v) => Ok((v, u)),
            Err(Ctl::Return(Value::Int(a, w))) => Ok((a as i64, w)),
            Err(Ctl::Return(v)) => Err(semantic(format!("typed return produced {}", v.type_name()))),
            Err(Ctl::Break(_)) | Err(Ctl::Continue(_)) => Err(semantic("`break`/`continue` outside of a loop")),
            Err(e) => Err(e),
        };
        match r {
            Ok((v, w)) => typed::retype(self, v, w, t, f.ret.expect("typed body has a return type")),
            Err(Ctl::Err(e)) => Err(Ctl::Err(Box::new(self.annotate(*e)))),
            Err(e) => Err(e),
        }
    }

    /// Coerce the arguments already in the frame to the parameter types (as the walker binds them).
    /// `false`: a typed parameter did not receive an integer (the caller runs the walker instead).
    fn bind_params(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, base: usize) -> R<bool> {
        for (((p, slot), fast), typed) in f.params.iter().zip(&c.params).zip(&c.param_int).zip(&c.param_typed) {
            let v = &self.rstack[base + slot];
            if let (Some(t), Value::Int(a, vt)) = (fast, v) {
                if vt == t && (!typed || *a as i64 as i128 == *a) {
                    continue; // already the declared width
                }
            }
            let v = std::mem::replace(&mut self.rstack[base + slot], Value::Unit);
            self.rstack[base + slot] = self.coerce(v, &p.type_)?;
            if *typed && !matches!(self.rstack[base + slot], Value::Int(a, _) if a as i64 as i128 == a) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

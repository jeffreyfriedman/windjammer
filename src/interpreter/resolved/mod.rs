//! `--engine=resolved` (ADR-0020 option B, milestone 1): functions compiled to closure trees with
//! slot-indexed locals.
//!
//! A function is compiled the first time it is called, or never: every construct outside the
//! milestone subset (see `compile.rs`) makes the whole function run in the tree walker, so the two
//! engines never disagree about what is supported. The compiled code calls the same helpers as the
//! walker (`binary`, `coerce`, `format_args`, `call_func_g`...), marks the same source locations and
//! keeps the same frame/backtrace discipline, so values, panics and error spans are identical.
//!
//! Locals live in `Interpreter::rstack` (one window per active compiled call), which reachability
//! (`heap.rs`) walks as a root and the scheduler swaps per task. Programs with destructors
//! (`impl Drop`) are not compiled: scope-exit drops need the walker's slots.

mod compile;
mod fast;

use super::engine::{FuncDef, Interpreter, Scope};
use super::error::{semantic, Ctl, R};
use super::place::deref_value;
use super::value::Value;
use std::rc::Rc;

/// Compiled code: runs against the interpreter and the base index of its frame in `rstack`.
pub(crate) type Code = Box<dyn Fn(&mut Interpreter, usize) -> R>;

pub(crate) struct CompiledFn {
    nslots: usize,
    /// Slots of the parameters, in order.
    params: Vec<usize>,
    /// Integer width of each parameter / the return type, when declared (skip `coerce` for exact matches).
    param_int: Vec<Option<super::int::IntTy>>,
    ret_int: Option<super::int::IntTy>,
    body: Code,
}

impl Interpreter {
    /// The compiled form of `f`, compiling it on first use; `None` = run it in the walker.
    pub(crate) fn compiled_for(&mut self, f: &Rc<FuncDef>) -> Option<Rc<CompiledFn>> {
        let key = Rc::as_ptr(f) as usize;
        if let Some(c) = self.compiled.get(&key) {
            return c.clone();
        }
        let c = if self.drop_types.is_empty() { compile::compile_fn(self, f).map(Rc::new) } else { None };
        self.compiled.insert(key, c.clone());
        c
    }

    /// `call_func_run` for a compiled function: same frame, backtrace, binding and return rules.
    pub(crate) fn run_compiled(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, args: Vec<Value>) -> R {
        self.run_compiled_with(f, c, args.len(), |it, base, slots| {
            for (slot, a) in slots.iter().zip(args) {
                it.rstack[base + slot] = a;
            }
            Ok(())
        })
    }

    /// Run `c` with arguments written straight into its frame by `fill` (no argument vector).
    pub(crate) fn run_compiled_with(
        &mut self,
        f: &Rc<FuncDef>,
        c: &CompiledFn,
        nargs: usize,
        fill: impl FnOnce(&mut Interpreter, usize, &[usize]) -> R<()>,
    ) -> R {
        let _ = nargs;
        self.depth_check()?;
        self.backtrace.push((f.qual.clone(), self.cur));
        // a compiled caller's scope is empty and in the same module: no need for a new walker frame
        let own_frame = !(self.scope.vars.is_empty() && self.scope.marks.is_empty() && self.scope.module == f.module && self.scope.defers.is_empty() && self.scope.tparams.is_empty() && self.scope.self_ty.is_none());
        if own_frame {
            let mut scope = Scope::new(f.module);
            scope.ret_ty = f.ret;
            let saved = std::mem::replace(&mut self.scope, scope);
            self.frames.push(saved);
        }
        self.depth += 1;
        let base = self.rstack.len();
        self.rstack.resize(base + c.nslots, Value::Unit);

        let r = match fill(self, base, &c.params).and_then(|_| self.bind_params(f, c, base)) {
            Err(e) => Err(e),
            Ok(()) => (c.body)(self, base),
        };
        let r = match r {
            Err(Ctl::Return(v)) => Ok(v),
            Err(Ctl::Break { .. }) | Err(Ctl::Continue { .. }) => Err(semantic("`break`/`continue` outside of a loop")),
            other => other,
        };
        let r = match r {
            Ok(v) => match (f.ret, &c.ret_int) {
                (None, _) => Ok(Value::Unit),
                (Some(_), Some(t)) if matches!(&v, Value::Int(_, vt) if vt == t) => Ok(v),
                (Some(t), _) => deref_value(v).and_then(|v| self.coerce(v, t)),
            },
            Err(Ctl::Err(e)) => Err(Ctl::Err(Box::new(self.annotate(*e)))),
            other => other,
        };
        self.rstack.truncate(base);
        self.depth -= 1;
        if own_frame {
            self.scope = self.frames.pop().expect("frame stack");
        }
        self.backtrace.pop();
        r
    }

    /// Coerce the arguments already in the frame to the parameter types (as the walker binds them).
    fn bind_params(&mut self, f: &Rc<FuncDef>, c: &CompiledFn, base: usize) -> R<()> {
        for ((p, slot), fast) in f.params.iter().zip(&c.params).zip(&c.param_int) {
            let v = &self.rstack[base + slot];
            if let (Some(t), Value::Int(_, vt)) = (fast, v) {
                if vt == t {
                    continue; // already the declared width
                }
            }
            let v = std::mem::replace(&mut self.rstack[base + slot], Value::Unit);
            self.rstack[base + slot] = self.coerce(v, &p.type_)?;
        }
        Ok(())
    }
}

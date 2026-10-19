//! Cranelift tier (ADR-0020 step 7, spike behind `--features jit`): hot functions whose every value
//! is an `int` (`i64` / unsuffixed literal) and whose body is arithmetic, comparisons, `let`,
//! assignment, `if`, `while`, `return` and calls to such functions compile to machine code.
//!
//! Semantics by construction: a jitted call has no side effect, so on any edge -- an overflow, a
//! zero divisor, `i64::MIN / -1`, recursion deeper than `MAX_DEPTH` -- it reports a bail-out and the
//! call simply runs again in the closure engine, which produces the panic, location and backtrace.
//! With only `i64` and literal widths in play, the walker's width rules (`unify`, `retag`, `fits`)
//! all reduce to checked `i64` arithmetic, which is what the generated code does.

use crate::interpreter::engine::{FuncDef, Interpreter};
use crate::interpreter::int::IntTy;
use crate::parser::{BinaryOp, Expression, Literal, Pattern, Statement, Type, UnaryOp};
use cranelift_codegen::ir::{condcodes::IntCC, types, AbiParam, FuncRef, InstBuilder, MemFlags, Value as V};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use std::collections::HashMap;
use std::rc::Rc;

mod lower;
mod lower_expr;
pub(crate) mod osr;
mod rt;
mod kinds;

type E = &'static Expression<'static>;
type S = &'static Statement<'static>;

/// Calls after which a function is jitted.
pub(crate) const HOT: u32 = 64;
/// Nested jitted calls before bailing out (far below the interpreter's own recursion limit).
const MAX_DEPTH: i64 = 2000;
const MAX_ARITY: usize = 4;

#[derive(Clone, Copy)]
pub(crate) struct JitFn {
    ptr: *const u8,
    arity: usize,
}

impl JitFn {
    /// Run it; `None` = bailed out (run the call in the closure engine instead).
    pub(crate) fn call(&self, a: &[i64]) -> Option<i64> {
        let mut status: i64 = 0;
        let st: *mut i64 = &mut status;
        // SAFETY: `ptr` is a finalized function of the module kept alive by `Jit`, with exactly
        // `arity` i64 parameters followed by the depth and the status pointer.
        let r = unsafe {
            match self.arity {
                0 => std::mem::transmute::<*const u8, extern "C" fn(i64, *mut i64) -> i64>(self.ptr)(0, st),
                1 => std::mem::transmute::<*const u8, extern "C" fn(i64, i64, *mut i64) -> i64>(self.ptr)(a[0], 0, st),
                2 => std::mem::transmute::<*const u8, extern "C" fn(i64, i64, i64, *mut i64) -> i64>(self.ptr)(a[0], a[1], 0, st),
                3 => std::mem::transmute::<*const u8, extern "C" fn(i64, i64, i64, i64, *mut i64) -> i64>(self.ptr)(a[0], a[1], a[2], 0, st),
                _ => std::mem::transmute::<*const u8, extern "C" fn(i64, i64, i64, i64, i64, *mut i64) -> i64>(self.ptr)(a[0], a[1], a[2], a[3], 0, st),
            }
        };
        (status == 0).then_some(r)
    }
}

pub(crate) struct Jit {
    pub(super) module: JITModule,
    done: HashMap<usize, Option<JitFn>>,
    /// Keeps every jitted `FuncDef` alive (its address is the cache key).
    keep: Vec<Rc<FuncDef>>,
}

fn key(f: &Rc<FuncDef>) -> usize {
    Rc::as_ptr(f) as usize
}

fn is_i64(t: &Type) -> bool {
    IntTy::from_type(t) == Some(IntTy::I64) && !matches!(t, Type::Reference(_) | Type::MutableReference(_))
}

impl Jit {
    fn new() -> Option<Jit> {
        let mut flags = settings::builder();
        flags.set("use_colocated_libcalls", "false").ok()?;
        flags.set("is_pic", "false").ok()?;
        flags.set("opt_level", "speed").ok()?;
        let isa = cranelift_native::builder().ok()?.finish(settings::Flags::new(flags)).ok()?;
        let module = JITModule::new(JITBuilder::with_isa(isa, cranelift_module::default_libcall_names()));
        Some(Jit { module, done: HashMap::new(), keep: Vec::new() })
    }
}

/// The interpreter's Cranelift module, taken out while code is added to it (put it back in
/// `it.jit`); `None` when the host has no Cranelift backend (machine code is then off for the run).
fn take_jit(it: &mut Interpreter) -> Option<Box<Jit>> {
    if let Some(j) = it.jit.take() {
        return Some(j);
    }
    match Jit::new() {
        Some(j) => Some(Box::new(j)),
        None => {
            it.jit_enabled = false;
            None
        }
    }
}

impl Interpreter {
    /// The machine-code form of `f` (compiled with every function it calls), or `None`.
    pub(crate) fn jitted(&mut self, f: &Rc<FuncDef>) -> Option<JitFn> {
        if self.jit.is_none() {
            match Jit::new() {
                Some(j) => self.jit = Some(Box::new(j)),
                None => {
                    // no Cranelift backend for this host: closures only, and never try again
                    self.jit_enabled = false;
                    return None;
                }
            }
        }
        if let Some(r) = self.jit.as_ref().and_then(|j| j.done.get(&key(f)).copied()) {
            return r;
        }
        let r = self.jit_group(f);
        let j = self.jit.as_mut()?;
        j.done.insert(key(f), r);
        r
    }

    /// Declare `f` and its callees, translate them all, finalize once.
    fn jit_group(&mut self, f: &Rc<FuncDef>) -> Option<JitFn> {
        let mut group: Vec<Rc<FuncDef>> = Vec::new();
        if !self.collect(f, &mut group) {
            return None;
        }
        let mut jit = self.jit.take()?;
        let r = (|| {
            let mut ids: HashMap<usize, FuncId> = HashMap::new();
            for g in &group {
                let sig = signature(&jit.module, g.params.len());
                let id = jit.module.declare_function(&format!("wj_{}_{:x}", g.name, key(g)), Linkage::Local, &sig).ok()?;
                ids.insert(key(g), id);
            }
            let mut ctx = jit.module.make_context();
            let mut fctx = FunctionBuilderContext::new();
            for g in &group {
                ctx.func.signature = signature(&jit.module, g.params.len());
                translate(self, &mut jit.module, &mut ctx.func, &mut fctx, g, &ids)?;
                jit.module.define_function(ids[&key(g)], &mut ctx).ok()?;
                jit.module.clear_context(&mut ctx);
            }
            jit.module.finalize_definitions().ok()?;
            for g in &group {
                let ptr = jit.module.get_finalized_function(ids[&key(g)]);
                jit.done.insert(key(g), Some(JitFn { ptr, arity: g.params.len() }));
                jit.keep.push(g.clone());
            }
            jit.done.get(&key(f)).copied().flatten()
        })();
        self.jit = Some(jit);
        r
    }

    /// `f` and the functions it calls, if all are in the jittable subset (by signature; bodies are
    /// checked by `translate`).
    fn collect(&mut self, f: &Rc<FuncDef>, group: &mut Vec<Rc<FuncDef>>) -> bool {
        if group.iter().any(|g| Rc::ptr_eq(g, f)) {
            return true;
        }
        if f.is_extern || f.is_async || f.self_ty.is_some() || !f.type_params.is_empty() || f.params.len() > MAX_ARITY {
            return false;
        }
        if !f.params.iter().all(|p| p.name != "self" && matches!(p.pattern, None | Some(Pattern::Identifier(_))) && is_i64(&p.type_))
            || !f.ret.is_some_and(is_i64)
            || self.param_mutation(f).iter().any(|m| *m)
            || self.jit.as_ref().is_some_and(|j| j.done.get(&key(f)) == Some(&None))
        {
            return false;
        }
        group.push(f.clone());
        let mut callees = Vec::new();
        for s in f.body {
            calls_in_stmt(s, &mut callees);
        }
        for name in callees {
            let Some(g) = self.modules[f.module].funcs.get(name).cloned() else { return false };
            if g.module != f.module || !self.collect(&g, group) {
                return false;
            }
        }
        true
    }
}

impl PartialEq for JitFn {
    fn eq(&self, o: &JitFn) -> bool {
        self.ptr == o.ptr
    }
}

fn signature(m: &JITModule, n: usize) -> cranelift_codegen::ir::Signature {
    let mut sig = m.make_signature();
    for _ in 0..n + 1 {
        sig.params.push(AbiParam::new(types::I64));
    }
    sig.params.push(AbiParam::new(m.target_config().pointer_type()));
    sig.returns.push(AbiParam::new(types::I64));
    sig
}

fn calls_in_stmt(s: S, out: &mut Vec<&'static str>) {
    match s {
        Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => calls_in(expr, out),
        Statement::Let { value, .. } | Statement::Assignment { value, .. } => calls_in(value, out),
        Statement::If { condition, then_block, else_block, .. } => {
            calls_in(condition, out);
            then_block.iter().chain(else_block.iter().flat_map(|b| b.iter())).for_each(|s| calls_in_stmt(s, out));
        }
        Statement::While { condition, body, .. } => {
            calls_in(condition, out);
            body.iter().for_each(|s| calls_in_stmt(s, out));
        }
        _ => {}
    }
}

fn calls_in(e: E, out: &mut Vec<&'static str>) {
    match e {
        Expression::Call { function: Expression::Identifier { name, .. }, arguments, .. } => {
            out.push(name.as_str());
            arguments.iter().for_each(|(_, a)| calls_in(a, out));
        }
        Expression::Binary { left, right, .. } => {
            calls_in(left, out);
            calls_in(right, out);
        }
        Expression::Unary { operand, .. } => calls_in(operand, out),
        _ => {}
    }
}

/// A translated expression: an `i64`, or a condition (`i8`).
enum T {
    Int(V),
    Bool(V),
}

struct Tr<'a> {
    b: FunctionBuilder<'a>,
    scopes: Vec<Vec<(&'static str, Variable)>>,
    depth: V,
    status: V,
    fns: HashMap<&'static str, FuncRef>,
    /// The current block already ended (after `return`): further code goes to a dead block.
    ended: bool,
}

fn translate(
    it: &mut Interpreter,
    module: &mut JITModule,
    func: &mut cranelift_codegen::ir::Function,
    fctx: &mut FunctionBuilderContext,
    f: &Rc<FuncDef>,
    ids: &HashMap<usize, FuncId>,
) -> Option<()> {
    let mut fns = HashMap::new();
    let mut callees = Vec::new();
    f.body.iter().for_each(|s| calls_in_stmt(s, &mut callees));
    for name in callees {
        let g = it.modules[f.module].funcs.get(name)?;
        let id = *ids.get(&key(g))?;
        fns.insert(name, module.declare_func_in_func(id, func));
    }
    let mut b = FunctionBuilder::new(func, fctx);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    b.seal_block(entry);
    let p = b.block_params(entry).to_vec();
    let n = f.params.len();
    let (depth, status) = (p[n], p[n + 1]);
    let mut tr = Tr { b, scopes: vec![Vec::new()], depth, status, fns, ended: false };
    // recursion budget
    let too_deep = tr.b.ins().icmp_imm(IntCC::SignedGreaterThan, depth, MAX_DEPTH);
    tr.bail_if(too_deep);
    for (param, v) in f.params.iter().zip(&p[..n]) {
        let var = tr.b.declare_var(types::I64);
        tr.b.def_var(var, *v);
        tr.scopes[0].push((param.name.as_str(), var));
    }
    let (last, init) = f.body.split_last()?;
    for s in init {
        tr.stmt(s)?;
    }
    match last {
        Statement::Expression { expr, .. } => {
            let T::Int(v) = tr.expr(expr)? else { return None };
            tr.ret(v);
        }
        Statement::Return { .. } => tr.stmt(last)?,
        _ => return None,
    }
    if !tr.ended {
        return None;
    }
    tr.b.finalize();
    Some(())
}

impl Tr<'_> {
    fn var(&self, name: &str) -> Option<Variable> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(n, _)| *n == name).map(|(_, v)| *v)
    }

    /// Continue in a fresh block (the previous one ended).
    fn dead(&mut self) {
        let blk = self.b.create_block();
        self.b.switch_to_block(blk);
        self.b.seal_block(blk);
        self.ended = false;
    }

    fn ret(&mut self, v: V) {
        self.b.ins().return_(&[v]);
        self.ended = true;
        self.dead();
        self.ended = true;
    }

    /// `if cond { *status = 1; return 0 }`
    fn bail_if(&mut self, cond: V) {
        let bail = self.b.create_block();
        let cont = self.b.create_block();
        self.b.ins().brif(cond, bail, &[], cont, &[]);
        self.b.switch_to_block(bail);
        self.b.seal_block(bail);
        let one = self.b.ins().iconst(types::I64, 1);
        self.b.ins().store(MemFlags::trusted(), one, self.status, 0);
        let zero = self.b.ins().iconst(types::I64, 0);
        self.b.ins().return_(&[zero]);
        self.b.switch_to_block(cont);
        self.b.seal_block(cont);
    }

    fn block(&mut self, list: &'static [S]) -> Option<()> {
        self.scopes.push(Vec::new());
        for s in list {
            self.stmt(s)?;
        }
        self.scopes.pop();
        Some(())
    }

    fn stmt(&mut self, s: S) -> Option<()> {
        self.ended = false;
        match s {
            Statement::Let { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), type_, value, else_block: None, .. } => {
                if type_.as_ref().is_some_and(|t| !is_i64(t)) {
                    return None;
                }
                let T::Int(v) = self.expr(value)? else { return None };
                let var = self.b.declare_var(types::I64);
                self.b.def_var(var, v);
                self.scopes.last_mut()?.push((n.as_str(), var));
            }
            Statement::Assignment { target: Expression::Identifier { name, .. }, value, compound_op, .. } => {
                let var = self.var(name)?;
                let T::Int(v) = self.expr(value)? else { return None };
                let v = match compound_op {
                    None => v,
                    Some(op) => {
                        let cur = self.b.use_var(var);
                        self.arith(crate::interpreter::ops::compound_to_binary(*op), cur, v)?
                    }
                };
                self.b.def_var(var, v);
            }
            Statement::Return { value: Some(e), .. } => {
                let T::Int(v) = self.expr(e)? else { return None };
                self.ret(v);
            }
            Statement::If { condition, then_block, else_block, .. } => {
                let T::Bool(c) = self.expr(condition)? else { return None };
                let (then_b, else_b, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().brif(c, then_b, &[], else_b, &[]);
                self.b.switch_to_block(then_b);
                self.b.seal_block(then_b);
                self.block(then_block)?;
                self.b.ins().jump(join, &[]);
                self.b.switch_to_block(else_b);
                self.b.seal_block(else_b);
                if let Some(eb) = else_block {
                    self.block(eb)?;
                }
                self.b.ins().jump(join, &[]);
                self.b.switch_to_block(join);
                self.b.seal_block(join);
            }
            Statement::While { condition, body, label: None, .. } => {
                let (head, body_b, exit) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
                self.b.ins().jump(head, &[]);
                self.b.switch_to_block(head);
                let T::Bool(c) = self.expr(condition)? else { return None };
                self.b.ins().brif(c, body_b, &[], exit, &[]);
                self.b.switch_to_block(body_b);
                self.b.seal_block(body_b);
                self.block(body)?;
                self.b.ins().jump(head, &[]);
                self.b.seal_block(head);
                self.b.switch_to_block(exit);
                self.b.seal_block(exit);
            }
            Statement::Expression { expr, .. } => {
                self.expr(expr)?;
            }
            _ => return None,
        }
        Some(())
    }

    /// Checked `x op y` (the walker's `int_binary` on `i64`): any edge bails out.
    fn arith(&mut self, op: BinaryOp, x: V, y: V) -> Option<V> {
        let ins = |b: &mut FunctionBuilder, op: BinaryOp| match op {
            BinaryOp::Add => Some(b.ins().sadd_overflow(x, y)),
            BinaryOp::Sub => Some(b.ins().ssub_overflow(x, y)),
            BinaryOp::Mul => Some(b.ins().smul_overflow(x, y)),
            _ => None,
        };
        if let Some((r, of)) = ins(&mut self.b, op) {
            self.bail_if(of);
            return Some(r);
        }
        match op {
            BinaryOp::Div | BinaryOp::Mod => {
                let zero = self.b.ins().icmp_imm(IntCC::Equal, y, 0);
                self.bail_if(zero);
                let min = self.b.ins().icmp_imm(IntCC::Equal, x, i64::MIN);
                let neg1 = self.b.ins().icmp_imm(IntCC::Equal, y, -1);
                let both = self.b.ins().band(min, neg1);
                self.bail_if(both);
                Some(if op == BinaryOp::Div { self.b.ins().sdiv(x, y) } else { self.b.ins().srem(x, y) })
            }
            _ => None,
        }
    }

    fn expr(&mut self, e: E) -> Option<T> {
        Some(match e {
            Expression::Literal { value: Literal::Int(i), .. } => T::Int(self.b.ins().iconst(types::I64, *i)),
            Expression::Literal { value: Literal::IntSuffixed(i, s), .. } if s == "i64" => T::Int(self.b.ins().iconst(types::I64, *i)),
            Expression::Identifier { name, .. } => T::Int(self.b.use_var(self.var(name)?)),
            Expression::Unary { op: UnaryOp::Neg, operand, .. } if !matches!(operand, Expression::Range { .. }) => {
                let T::Int(x) = self.expr(operand)? else { return None };
                let min = self.b.ins().icmp_imm(IntCC::Equal, x, i64::MIN);
                self.bail_if(min);
                T::Int(self.b.ins().ineg(x))
            }
            Expression::Unary { op: UnaryOp::Not, operand, .. } => {
                let T::Bool(c) = self.expr(operand)? else { return None };
                T::Bool(self.b.ins().icmp_imm(IntCC::Equal, c, 0))
            }
            Expression::Binary { left, op: op @ (BinaryOp::And | BinaryOp::Or), right, .. } => {
                let T::Bool(l) = self.expr(left)? else { return None };
                let (rhs, join) = (self.b.create_block(), self.b.create_block());
                self.b.append_block_param(join, types::I8);
                if *op == BinaryOp::And {
                    self.b.ins().brif(l, rhs, &[], join, &[l.into()]);
                } else {
                    self.b.ins().brif(l, join, &[l.into()], rhs, &[]);
                }
                self.b.switch_to_block(rhs);
                self.b.seal_block(rhs);
                let T::Bool(r) = self.expr(right)? else { return None };
                self.b.ins().jump(join, &[r.into()]);
                self.b.switch_to_block(join);
                self.b.seal_block(join);
                T::Bool(self.b.block_params(join)[0])
            }
            Expression::Binary { left, op, right, .. } => {
                let (T::Int(x), T::Int(y)) = (self.expr(left)?, self.expr(right)?) else { return None };
                let cc = match op {
                    BinaryOp::Eq => Some(IntCC::Equal),
                    BinaryOp::Ne => Some(IntCC::NotEqual),
                    BinaryOp::Lt => Some(IntCC::SignedLessThan),
                    BinaryOp::Le => Some(IntCC::SignedLessThanOrEqual),
                    BinaryOp::Gt => Some(IntCC::SignedGreaterThan),
                    BinaryOp::Ge => Some(IntCC::SignedGreaterThanOrEqual),
                    _ => None,
                };
                match cc {
                    Some(cc) => T::Bool(self.b.ins().icmp(cc, x, y)),
                    None => T::Int(self.arith(*op, x, y)?),
                }
            }
            Expression::Call { function: Expression::Identifier { name, .. }, arguments, .. } => {
                if self.var(name).is_some() || arguments.iter().any(|(l, _)| l.is_some()) {
                    return None;
                }
                let fr = *self.fns.get(name.as_str())?;
                let mut args = Vec::with_capacity(arguments.len() + 2);
                for (_, a) in arguments.iter() {
                    let T::Int(v) = self.expr(a)? else { return None };
                    args.push(v);
                }
                let d = self.b.ins().iadd_imm(self.depth, 1);
                args.push(d);
                args.push(self.status);
                let call = self.b.ins().call(fr, &args);
                let r = self.b.inst_results(call)[0];
                let st = self.b.ins().load(types::I64, MemFlags::trusted(), self.status, 0);
                let failed = self.b.ins().icmp_imm(IntCC::NotEqual, st, 0);
                self.bail_if(failed);
                T::Int(r)
            }
            _ => return None,
        })
    }
}

impl Interpreter {
    /// Run a hot call of `c` (frame at `base`, arguments in its first slots) as machine code;
    /// `None`: not jittable / not hot yet / bailed out (the caller runs the closures).
    pub(crate) fn try_jit(&mut self, f: &Rc<FuncDef>, c: &super::CompiledFn, base: usize) -> Option<i64> {
        let n = c.calls.get();
        if n < HOT {
            c.calls.set(n + 1);
            return None;
        }
        if self.hot.is_some() || !self.jit_enabled {
            return None;
        }
        let j = match c.jit.get() {
            Some(j) => *j,
            None => {
                let j = self.jitted(f);
                let _ = c.jit.set(j);
                j
            }
        }?;
        let mut a = [0i64; MAX_ARITY];
        for (k, slot) in a.iter_mut().enumerate().take(j.arity) {
            match &self.rstack[base + k] {
                crate::interpreter::value::Value::Int(v, _) if *v as i64 as i128 == *v => *slot = *v as i64,
                _ => return None,
            }
        }
        let r = j.call(&a[..j.arity])?;
        self.rstack.truncate(base);
        Some(r)
    }
}

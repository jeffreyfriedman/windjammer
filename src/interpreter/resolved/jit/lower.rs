//! Lowering of a hot `while` or range `for` loop (a loop unit) to Cranelift IR: statements, variables and exits
//! (expressions are in `lower_expr.rs`).
//!
//! Locals the loop reads or writes are Cranelift variables while it runs; each carries the walker's
//! width tag statically (a tag that would differ between branches or iterations refuses the loop).
//! Every check (overflow, bounds, an element of another type...) branches to the exit of the
//! statement being lowered: the exit stores the variables in scope to the spill buffer and returns
//! its number, and `osr.rs` writes them back to their slots and resumes the closure engine at that
//! statement, which has done nothing yet. `break`, `continue` and `return` are exits too.
//!
//! Subset: `let` (identifier, scalar value), assignment and compound assignment to a scalar local,
//! to `v[i]`, `v[i].f` and `s.f`; `v.push(x)`; `if`/`else`; nested `while` and `for i in a..b`;
//! `break`, `continue`, `return`. Anything else refuses the loop (it keeps running in the closure
//! engine). A `for` loop's counter and end are hidden variables: an exit inside its body hands them
//! to the closure engine, which runs the remaining iterations.

use super::rt;
use super::kinds::{int_ok, unsigned_repr, Class, Ty};
use crate::interpreter::int::IntTy;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::resolved::osr::{loc, Reg};
use crate::parser::{Expression, Pattern, Statement};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, AbiParam, Block, Function, InstBuilder, MemFlags, SigRef, Type as CTy, Value as V};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use std::collections::HashMap;

pub(super) type E = &'static Expression<'static>;
pub(super) type S = &'static Statement<'static>;
/// Where an exit resumes: the statement at `path.last()`, nested in the statements before it (an
/// empty path is the loop head).
pub(crate) type Path = Vec<Level>;

/// Statement `idx` of `list`. When the path goes on into the body of that statement and it is a
/// `for` loop, `for_` has the loop's counter state.
#[derive(Clone, Copy)]
pub(crate) struct Level {
    pub(crate) list: &'static [S],
    pub(crate) idx: usize,
    pub(crate) for_: Option<ForState>,
}

/// A `for` loop's counter and end (spill buffer indexes), width, and whether the end is included.
#[derive(Clone, Copy)]
pub(crate) struct ForState {
    pub(crate) counter: usize,
    pub(crate) end: usize,
    pub(crate) ty: IntTy,
    pub(crate) inclusive: bool,
}

/// The slot of a hidden variable (a `for` counter or end): never written back.
pub(crate) const NO_SLOT: usize = usize::MAX;

pub(crate) struct Exit {
    pub(crate) path: Path,
    /// Every variable in scope.
    pub(crate) spills: Vec<Spill>,
    /// `break`/`continue`/`return` or the loop's end, not a failed check.
    pub(crate) planned: bool,
}

/// A variable an exit writes back: its spill buffer index, slot and type; `fresh` for a `let` of the
/// loop (a boxed one gets a new walker slot, as its binding would).
#[derive(Clone, Copy)]
pub(crate) struct Spill {
    pub(crate) buf: usize,
    pub(crate) slot: usize,
    pub(crate) ty: Ty,
    pub(crate) fresh: bool,
}

pub(crate) struct Plan {
    /// Scalars read from their slots at entry: (spill buffer index, slot, type).
    pub(crate) live: Vec<(usize, usize, Ty)>,
    /// A `for` unit: the spill buffer index of its counter (the end follows it), filled at entry.
    pub(crate) counter: Option<usize>,
    pub(crate) exits: Vec<Exit>,
    pub(crate) nbuf: usize,
}

#[derive(Clone)]
pub(super) enum Bind {
    Var(usize),
    Handle(usize, Class),
}

pub(super) struct Var {
    pub(super) var: Variable,
    slot: usize,
    pub(super) ty: Ty,
    /// Bound by a `let` of the loop (not a local the loop was entered with).
    fresh: bool,
}

/// A lowered value; `k` is its value when it is an integer constant.
#[derive(Clone, Copy)]
pub(super) struct Val {
    pub(super) v: V,
    pub(super) ty: Ty,
    pub(super) k: Option<i64>,
}

impl Val {
    pub(super) fn of(v: V, ty: Ty) -> Val {
        Val { v, ty, k: None }
    }
}

pub(super) struct Lo<'a, 'i> {
    pub(super) b: FunctionBuilder<'a>,
    pub(super) it: &'i mut Interpreter,
    reg: &'i Reg,
    pub(super) ctx: V,
    buf: V,
    pub(super) ptr: CTy,
    scopes: Vec<Vec<(&'static str, Bind)>>,
    pub(super) vars: Vec<Var>,
    path: Path,
    exits: Vec<Exit>,
    /// The exit block of each statement being lowered (innermost last), created on first use.
    stmt_exit: Vec<Option<Block>>,
    /// Exit blocks whose bodies are emitted at the end: (block, exit number).
    pending: Vec<(Block, usize)>,
    sigs: HashMap<&'static str, SigRef>,
    /// The innermost construct that refused the loop (`WJ_JIT_DEBUG`).
    pub(super) why: Option<String>,
    /// The loops being lowered, innermost last (targets of `break` / `continue`).
    loops: Vec<LoopCtx>,
    /// The unit loop's label.
    unit_label: Option<&'static str>,
}

/// A loop being lowered: where `continue` goes (the head, or a `for` loop's increment), where `break`
/// goes (`None` for the unit's own loop: an exit, so the closure engine leaves its loop), and the
/// variable types at its head (both jumps must arrive with them).
struct LoopCtx {
    label: Option<&'static str>,
    cont: Block,
    brk: Option<Block>,
    tys: Vec<Ty>,
    /// Some jump reached `cont` (a `for` loop's increment block is emitted only then).
    cont_used: bool,
}

pub(crate) fn signature(ptr: CTy, call_conv: cranelift_codegen::isa::CallConv) -> cranelift_codegen::ir::Signature {
    let mut sig = cranelift_codegen::ir::Signature::new(call_conv);
    sig.params.push(AbiParam::new(ptr));
    sig.params.push(AbiParam::new(ptr));
    sig.returns.push(AbiParam::new(types::I64));
    sig
}

/// Lower `site`'s loop with the locals it names (`refs`: name, slot, class at entry) into `func`.
pub(crate) fn lower(
    func: &mut Function,
    fctx: &mut FunctionBuilderContext,
    it: &mut Interpreter,
    stmt: S,
    reg: &Reg,
    refs: &[(&'static str, usize, Class)],
    ptr: CTy,
) -> Result<Plan, String> {
    let mut b = FunctionBuilder::new(func, fctx);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let (ctx, buf) = (b.block_params(entry)[0], b.block_params(entry)[1]);
    let mut lo = Lo {
        b,
        it,
        reg,
        ctx,
        buf,
        ptr,
        scopes: vec![Vec::new()],
        vars: Vec::new(),
        path: Vec::new(),
        exits: Vec::new(),
        stmt_exit: Vec::new(),
        pending: Vec::new(),
        sigs: HashMap::new(),
        why: None,
        loops: Vec::new(),
        unit_label: match stmt {
            Statement::While { label, .. } | Statement::For { label, .. } => label.as_deref(),
            _ => None,
        },
    };
    let mut live = Vec::new();
    for (name, slot, class) in refs {
        if *slot == NO_SLOT {
            continue; // a `for` unit's counter type
        }
        let bind = match class {
            Class::Scalar(t) => {
                let id = lo.new_var(*slot, *t, false);
                let x = lo.b.ins().load(types::I64, MemFlags::trusted(), buf, (id * 8) as i32);
                let x = lo.from_bits(x, *t);
                lo.b.def_var(lo.vars[id].var, x);
                live.push((id, *slot, *t));
                Bind::Var(id)
            }
            c => Bind::Handle(*slot, c.clone()),
        };
        lo.scopes[0].push((name, bind));
    }
    let (ok, counter) = match stmt {
        Statement::While { condition, body, .. } => (lo.unit_loop(condition, body).is_some(), None),
        Statement::For { pattern, body, .. } => {
            let ty = match refs.last() {
                Some((_, _, Class::Scalar(Ty::Int(t)))) => *t,
                _ => return Err("no counter type".into()),
            };
            let c = lo.vars.len();
            (lo.unit_for(stmt, pattern, body, ty).is_some(), Some(c))
        }
        _ => return Err("not a loop".into()),
    };
    if !ok {
        return Err(lo.why.unwrap_or_default());
    }
    for (blk, id) in std::mem::take(&mut lo.pending) {
        lo.b.switch_to_block(blk);
        lo.exit_body(id);
    }
    lo.b.seal_all_blocks();
    lo.b.finalize();
    Ok(Plan { live, counter, nbuf: lo.vars.len(), exits: lo.exits })
}

impl Lo<'_, '_> {
    pub(super) fn lookup(&self, name: &str) -> Option<&Bind> {
        self.scopes.iter().rev().flat_map(|s| s.iter().rev()).find(|(n, _)| *n == name).map(|(_, b)| b)
    }

    fn new_var(&mut self, slot: usize, ty: Ty, fresh: bool) -> usize {
        let var = self.b.declare_var(cty(ty));
        self.vars.push(Var { var, slot, ty, fresh });
        self.vars.len() - 1
    }

    fn tys(&self) -> Vec<Ty> {
        self.vars.iter().map(|v| v.ty).collect()
    }

    fn set_tys(&mut self, tys: &[Ty]) {
        for (v, t) in self.vars.iter_mut().zip(tys) {
            v.ty = *t;
        }
    }

    /// Do the variables that existed at a loop head (`head`) still have their head types?
    fn same_tys(&mut self, head: &[Ty]) -> bool {
        let same = self.vars.iter().zip(head).all(|(v, t)| v.ty == *t);
        if !same {
            self.refused(|| "a local's width changes from one iteration to the next".into());
        }
        same
    }

    pub(super) fn from_bits(&mut self, x: V, t: Ty) -> V {
        match t {
            Ty::Int(_) => x,
            Ty::Flt(_) => self.b.ins().bitcast(types::F64, MemFlags::new(), x),
            Ty::Bool => self.b.ins().ireduce(types::I8, x),
        }
    }

    pub(super) fn to_bits(&mut self, x: V, t: Ty) -> V {
        match t {
            Ty::Int(_) => x,
            Ty::Flt(_) => self.b.ins().bitcast(types::I64, MemFlags::new(), x),
            Ty::Bool => self.b.ins().uextend(types::I64, x),
        }
    }

    fn new_exit(&mut self, planned: bool) -> usize {
        let mut spills = Vec::new();
        for scope in &self.scopes {
            for (_, bind) in scope {
                if let Bind::Var(id) = bind {
                    let v = &self.vars[*id];
                    spills.push(Spill { buf: *id, slot: v.slot, ty: v.ty, fresh: v.fresh });
                }
            }
        }
        self.exits.push(Exit { path: self.path.clone(), spills, planned });
        self.exits.len() - 1
    }

    /// Store the variables of exit `id` and return its number (emitted after the loop is lowered).
    fn exit_body(&mut self, id: usize) {
        let spills = self.exits[id].spills.clone();
        for sp in spills {
            let x = self.b.use_var(self.vars[sp.buf].var);
            let x = self.to_bits(x, sp.ty);
            self.b.ins().store(MemFlags::trusted(), x, self.buf, (sp.buf * 8) as i32);
        }
        let r = self.b.ins().iconst(types::I64, id as i64);
        self.b.ins().return_(&[r]);
    }

    /// Leave through the current statement's exit when `cond` holds.
    pub(super) fn bail_if(&mut self, cond: V) {
        let ex = match self.stmt_exit.last().copied().flatten() {
            Some(blk) => blk,
            None => {
                let id = self.new_exit(false);
                let blk = self.b.create_block();
                self.pending.push((blk, id));
                *self.stmt_exit.last_mut().expect("statement") = Some(blk);
                blk
            }
        };
        let cont = self.b.create_block();
        self.b.ins().brif(cond, ex, &[], cont, &[]);
        self.b.switch_to_block(cont);
        self.b.seal_block(cont);
    }

    /// Leave unconditionally (`break`/`continue`/`return`/the loop's end), resuming at the current path.
    fn exit_now(&mut self, planned: bool) {
        let id = self.new_exit(planned);
        let blk = self.b.create_block();
        self.b.ins().jump(blk, &[]);
        self.pending.push((blk, id));
    }

    /// A runtime helper's signature: `n` `i64` parameters after the context (`f`: all `f64`, no context).
    pub(super) fn helper(&mut self, name: &'static str, n: usize, float: bool) -> SigRef {
        if let Some(s) = self.sigs.get(name) {
            return *s;
        }
        let mut sig = self.b.func.signature.clone();
        sig.params.clear();
        sig.returns.clear();
        if float {
            sig.params.extend((0..n).map(|_| AbiParam::new(types::F64)));
            sig.returns.push(AbiParam::new(types::F64));
        } else {
            sig.params.push(AbiParam::new(self.ptr));
            sig.params.extend((0..n).map(|_| AbiParam::new(types::I64)));
            sig.returns.push(AbiParam::new(types::I64));
        }
        let r = self.b.import_signature(sig);
        self.sigs.insert(name, r);
        r
    }

    /// Call helper `f` (context first unless `float`), then leave if it reported failure.
    pub(super) fn call(&mut self, name: &'static str, f: usize, args: &[V], float: bool, fallible: bool) -> V {
        let sig = self.helper(name, args.len(), float);
        let callee = self.b.ins().iconst(self.ptr, f as i64);
        let mut all = Vec::with_capacity(args.len() + 1);
        if !float {
            all.push(self.ctx);
        }
        all.extend_from_slice(args);
        let call = self.b.ins().call_indirect(sig, callee, &all);
        let r = self.b.inst_results(call)[0];
        if fallible {
            let fl = self.b.ins().load(types::I64, MemFlags::trusted(), self.ctx, rt::FAIL);
            self.bail_if(fl);
        }
        r
    }

    fn unit_loop(&mut self, cond: E, body: &'static [S]) -> Option<()> {
        self.check_block(body)?;
        let head = self.b.create_block();
        self.b.ins().jump(head, &[]);
        let entry = self.b.current_block()?;
        self.b.seal_block(entry);
        self.b.switch_to_block(head);
        let tys0 = self.tys();
        // a failed check in the condition resumes at the head (the closure engine evaluates it)
        self.stmt_exit.push(None);
        let c = self.cond(cond)?;
        let (bb, out) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(c, bb, &[], out, &[]);
        self.stmt_exit.pop();
        self.b.switch_to_block(out);
        self.b.seal_block(out);
        self.exit_now(true);
        self.b.switch_to_block(bb);
        self.b.seal_block(bb);
        self.loops.push(LoopCtx { label: self.unit_label, cont: head, brk: None, tys: tys0.clone(), cont_used: false });
        let ended = self.block(body)?;
        self.loops.pop();
        if !ended {
            if !self.same_tys(&tys0) {
                return None;
            }
            self.b.ins().jump(head, &[]);
        }
        self.b.seal_block(head);
        Some(())
    }

    /// `break` / `continue` (with an optional label): a jump when the target loop is being lowered
    /// here, an exit (the closure engine runs the statement) when it is the unit's own `break`.
    fn jump_out(&mut self, label: Option<&str>, is_break: bool) -> Option<bool> {
        let k = match label {
            None => self.loops.len().checked_sub(1)?,
            Some(l) => self.loops.iter().rposition(|c| c.label == Some(l))?,
        };
        if is_break && self.loops[k].brk.is_none() {
            self.exit_now(true);
            return Some(true);
        }
        let tys = self.loops[k].tys.clone();
        if !self.same_tys(&tys) {
            return None;
        }
        let target = if is_break { self.loops[k].brk? } else { self.loops[k].cont };
        if !is_break {
            self.loops[k].cont_used = true;
        }
        self.b.ins().jump(target, &[]);
        Some(true)
    }

    /// Every statement of `list` can be resumed at (registered code, a location) and the block's scope is known.
    fn check_block(&mut self, list: &'static [S]) -> Option<()> {
        let ok = self.reg.block(list).is_some() && list.iter().all(|s| self.reg.code(s).is_some() && loc(s).is_some());
        if !ok {
            self.refused(|| "a block the closure engine cannot resume in (an island or no location)".into());
        }
        ok.then_some(())
    }

    /// A statement list in its own name scope; `Some(true)`: control always leaves through an exit.
    fn block(&mut self, list: &'static [S]) -> Option<bool> {
        self.check_block(list)?;
        self.scopes.push(Vec::new());
        let mut ended = false;
        for (i, s) in list.iter().enumerate() {
            self.path.push(Level { list, idx: i, for_: None });
            self.stmt_exit.push(None);
            let r = self.stmt(s);
            self.stmt_exit.pop();
            self.path.pop();
            if r? {
                ended = true;
                break;
            }
        }
        self.scopes.pop();
        Some(ended)
    }

    /// One statement; `Some(true)` when control leaves through an exit.
    fn stmt(&mut self, s: S) -> Option<bool> {
        let r = self.stmt_in(s);
        if r.is_none() {
            self.refused(|| format!("{s:?}"));
        }
        r
    }

    /// Note what refused the loop (the first, innermost construct).
    pub(super) fn refused(&mut self, what: impl FnOnce() -> String) {
        if self.why.is_none() && self.it.jit_debug {
            let mut w = what();
            w.truncate(300);
            self.why = Some(w);
        }
    }

    fn stmt_in(&mut self, s: S) -> Option<bool> {
        match s {
            Statement::Let { pattern: Pattern::Identifier(n) | Pattern::MutBinding(n), type_, value, else_block: None, .. } => {
                let slot = self.reg.let_slot(s)?;
                self.reg.kind(slot)?;
                let mut v = self.expr(value)?;
                if let Some(t) = type_ {
                    if matches!(value, Expression::Call { .. } | Expression::MethodCall { .. }) {
                        return None; // the walker hands such a value an expected type
                    }
                    v = self.coerce(v, t)?;
                }
                let id = self.new_var(slot, v.ty, true);
                self.b.def_var(self.vars[id].var, v.v);
                self.scopes.last_mut()?.push((n.as_str(), Bind::Var(id)));
            }
            Statement::Assignment { target, value, compound_op, .. } => self.assign(target, value, *compound_op)?,
            Statement::Expression { expr, .. } => self.effect(expr)?,
            Statement::If { condition, then_block, else_block, .. } => return self.if_stmt(condition, then_block, else_block.as_deref()),
            Statement::While { condition, body, label, .. } => self.while_stmt(condition, body, label.as_deref())?,
            Statement::For { pattern, iterable: Expression::Range { start, end, inclusive, .. }, body, label, .. } => {
                self.for_stmt(s, pattern, (start, end, *inclusive), body, label.as_deref())?
            }
            Statement::Break { label, value: None, .. } => return self.jump_out(label.as_deref(), true),
            Statement::Continue { label, .. } => return self.jump_out(label.as_deref(), false),
            Statement::Break { .. } | Statement::Return { .. } => {
                self.exit_now(true);
                return Some(true);
            }
            _ => return None,
        }
        Some(false)
    }

    fn if_stmt(&mut self, cond: E, then_b: &'static [S], else_b: Option<&'static [S]>) -> Option<bool> {
        let c = self.cond(cond)?;
        let (tb, eb, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
        self.b.ins().brif(c, tb, &[], eb, &[]);
        let tys0 = self.tys();
        self.b.switch_to_block(tb);
        self.b.seal_block(tb);
        let t_end = self.block(then_b)?;
        if !t_end {
            self.b.ins().jump(join, &[]);
        }
        let tys_t = self.tys();
        self.set_tys(&tys0);
        self.b.switch_to_block(eb);
        self.b.seal_block(eb);
        let e_end = match else_b {
            Some(list) => self.block(list)?,
            None => false,
        };
        if !e_end {
            self.b.ins().jump(join, &[]);
        }
        match (t_end, e_end) {
            (true, true) => return Some(true),
            (false, true) => self.set_tys(&tys_t),
            (false, false) if tys_t[..tys0.len()] != self.tys()[..tys0.len()] => return None,
            _ => {}
        }
        self.b.switch_to_block(join);
        self.b.seal_block(join);
        Some(false)
    }

    fn while_stmt(&mut self, cond: E, body: &'static [S], label: Option<&'static str>) -> Option<()> {
        let head = self.b.create_block();
        self.b.ins().jump(head, &[]);
        self.b.switch_to_block(head);
        let tys0 = self.tys();
        let c = self.cond(cond)?;
        let (bb, out) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(c, bb, &[], out, &[]);
        self.b.switch_to_block(bb);
        self.b.seal_block(bb);
        self.loops.push(LoopCtx { label, cont: head, brk: Some(out), tys: tys0.clone(), cont_used: false });
        let ended = self.block(body);
        self.loops.pop();
        if !ended? {
            if !self.same_tys(&tys0) {
                return None;
            }
            self.b.ins().jump(head, &[]);
        }
        self.b.seal_block(head);
        self.b.switch_to_block(out);
        self.b.seal_block(out);
        self.set_tys(&tys0);
        Some(())
    }

    /// A hidden variable (a `for` counter or end), bound in the innermost scope so exits spill it.
    fn hidden(&mut self, name: &'static str, ty: Ty, v: V) -> usize {
        let id = self.new_var(NO_SLOT, ty, true);
        self.b.def_var(self.vars[id].var, v);
        self.scopes.last_mut().expect("scope").push((name, Bind::Var(id)));
        id
    }

    /// The loop variable of an iteration (a fresh binding of the counter's value), in a new scope.
    fn loop_var(&mut self, s: S, pattern: &'static Pattern<'static>, ty: Ty, c: V) -> Option<()> {
        let info = self.reg.for_info(s)?;
        self.scopes.push(Vec::new());
        match (pattern, info.slot) {
            (Pattern::Identifier(n) | Pattern::MutBinding(n), Some(slot)) => {
                self.reg.kind(slot)?;
                let id = self.new_var(slot, ty, true);
                self.b.def_var(self.vars[id].var, c);
                self.scopes.last_mut()?.push((n.as_str(), Bind::Var(id)));
            }
            (Pattern::Wildcard, None) => {}
            _ => return None,
        }
        Some(())
    }

    /// `c < e` (`c <= e` when inclusive) in the counter's representation.
    fn below(&mut self, c: V, e: V, ty: IntTy, inclusive: bool) -> V {
        let cc = match (unsigned_repr(ty), inclusive) {
            (false, false) => IntCC::SignedLessThan,
            (false, true) => IntCC::SignedLessThanOrEqual,
            (true, false) => IntCC::UnsignedLessThan,
            (true, true) => IntCC::UnsignedLessThanOrEqual,
        };
        self.b.ins().icmp(cc, c, e)
    }

    /// A `for` unit: the counter and end arrive in the spill buffer (after the live-ins), the
    /// exclusive end already applied; reaching the end exits at the head with the counter.
    fn unit_for(&mut self, s: S, pattern: &'static Pattern<'static>, body: &'static [S], ty: IntTy) -> Option<()> {
        self.check_block(body)?;
        let t = Ty::Int(ty);
        let (c0, e0) = (self.vars.len(), self.vars.len() + 1);
        let c0v = self.b.ins().load(types::I64, MemFlags::trusted(), self.buf, (c0 * 8) as i32);
        let e0v = self.b.ins().load(types::I64, MemFlags::trusted(), self.buf, (e0 * 8) as i32);
        let c = self.hidden("\0counter", t, c0v);
        let e = self.hidden("\0end", t, e0v);
        let head = self.b.create_block();
        self.b.ins().jump(head, &[]);
        let entry = self.b.current_block()?;
        self.b.seal_block(entry);
        self.b.switch_to_block(head);
        let tys0 = self.tys();
        let (cv, ev) = (self.b.use_var(self.vars[c].var), self.b.use_var(self.vars[e].var));
        let go = self.below(cv, ev, ty, false);
        let (bb, out) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(go, bb, &[], out, &[]);
        self.b.switch_to_block(out);
        self.b.seal_block(out);
        self.exit_now(true);
        self.b.switch_to_block(bb);
        self.b.seal_block(bb);
        self.loop_var(s, pattern, t, cv)?;
        let latch = self.b.create_block();
        self.loops.push(LoopCtx { label: self.unit_label, cont: latch, brk: None, tys: tys0.clone(), cont_used: false });
        let ended = self.block(body);
        let ctx = self.loops.pop()?;
        self.scopes.pop();
        if !ended? {
            if !self.same_tys(&tys0) {
                return None;
            }
            self.b.ins().jump(latch, &[]);
        }
        if !ended? || ctx.cont_used {
            self.b.switch_to_block(latch);
            self.b.seal_block(latch);
            let cv = self.b.use_var(self.vars[c].var);
            let next = self.b.ins().iadd_imm(cv, 1);
            self.b.def_var(self.vars[c].var, next);
            self.b.ins().jump(head, &[]);
        }
        self.b.seal_block(head);
        Some(())
    }

    /// `for x in a..b` / `a..=b` inside a unit.
    fn for_stmt(&mut self, s: S, pattern: &'static Pattern<'static>, (start, end, inclusive): (E, E, bool), body: &'static [S], label: Option<&'static str>) -> Option<()> {
        let (a, z) = (self.expr(start)?, self.expr(end)?);
        let (Ty::Int(ta), Ty::Int(tz)) = (a.ty, z.ty) else { return None };
        let ty = IntTy::unify(ta, tz);
        if !int_ok(ty) {
            return None;
        }
        // the range holds the bounds' values with the unified width: they must be representable
        for (v, t) in [(a, ta), (z, tz)] {
            if unsigned_repr(t) != unsigned_repr(ty) {
                let neg = self.b.ins().icmp_imm(IntCC::SignedLessThan, v.v, 0);
                self.bail_if(neg);
            }
        }
        let t = Ty::Int(ty);
        self.scopes.push(Vec::new());
        let c = self.hidden("\0counter", t, a.v);
        let e = self.hidden("\0end", t, z.v);
        self.path.last_mut()?.for_ = Some(ForState { counter: c, end: e, ty, inclusive });
        let head = self.b.create_block();
        self.b.ins().jump(head, &[]);
        self.b.switch_to_block(head);
        let tys0 = self.tys();
        let (cv, ev) = (self.b.use_var(self.vars[c].var), self.b.use_var(self.vars[e].var));
        let go = self.below(cv, ev, ty, inclusive);
        let (bb, out) = (self.b.create_block(), self.b.create_block());
        self.b.ins().brif(go, bb, &[], out, &[]);
        self.b.switch_to_block(bb);
        self.b.seal_block(bb);
        self.loop_var(s, pattern, t, cv)?;
        let latch = self.b.create_block();
        self.loops.push(LoopCtx { label, cont: latch, brk: Some(out), tys: tys0.clone(), cont_used: false });
        let ended = self.block(body);
        let ctx = self.loops.pop()?;
        self.scopes.pop();
        if !ended? {
            if !self.same_tys(&tys0) {
                return None;
            }
            self.b.ins().jump(latch, &[]);
        }
        if !ended? || ctx.cont_used {
            self.b.switch_to_block(latch);
            self.b.seal_block(latch);
            let (cv, ev) = (self.b.use_var(self.vars[c].var), self.b.use_var(self.vars[e].var));
            let next = self.b.ins().iadd_imm(cv, 1);
            self.b.def_var(self.vars[c].var, next);
            if inclusive {
                // the last value is the end itself: stop before the counter could pass it
                let last = self.b.ins().icmp(IntCC::Equal, cv, ev);
                self.b.ins().brif(last, out, &[], head, &[]);
            } else {
                self.b.ins().jump(head, &[]);
            }
        }
        self.b.seal_block(head);
        self.b.switch_to_block(out);
        self.b.seal_block(out);
        self.set_tys(&tys0);
        self.scopes.pop();
        self.path.last_mut()?.for_ = None;
        Some(())
    }

    /// Bind a new value to scalar variable `id` (its type may change within its class).
    pub(super) fn set_var(&mut self, id: usize, v: Val) -> Option<()> {
        if cty(v.ty) != cty(self.vars[id].ty) {
            return None;
        }
        self.b.def_var(self.vars[id].var, v.v);
        self.vars[id].ty = v.ty;
        Some(())
    }
}

pub(super) fn cty(t: Ty) -> CTy {
    match t {
        Ty::Int(_) => types::I64,
        Ty::Flt(_) => types::F64,
        Ty::Bool => types::I8,
    }
}

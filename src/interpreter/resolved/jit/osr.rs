//! Entering a hot `while` loop in machine code at its head (on-stack replacement) and coming back.
//!
//! At the head of an armed loop (`OsrSite::armed`), the locals the loop names are classified
//! (`types::Class`: a scalar of some width, a `Vec`, a struct...). Machine code is compiled once per
//! classification (`lower.rs`) and cached on the site. It runs with the scalars in registers and
//! stops at an exit; the exit's variables go back to their slots, and the closure engine finishes
//! the iteration from the exit's statement (`resume`), using the statement codes the compiled
//! function registered (`resolved/osr.rs`). Nothing the loop does is visible before that point, so
//! the closure engine's run of the statement is the run the program would have had.

use super::lower::{self, Level, Path, Plan, StructSpill, NO_SLOT, S};
use super::rt::Ctx;
use super::kinds::{bits, class_of, class_of_is, int_value, same_name, ty_of, value, value_class, value_class_is, Class, Ty};
use crate::interpreter::int::IntTy;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::{Ctl, R};
use crate::interpreter::eval_stmt::loop_exit;
use crate::interpreter::resolved::osr::{close_block, loc, ForInfo, OsrSite, Reg, SlotKind};
use crate::interpreter::value::Value;
use crate::parser::{Expression, Statement};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::Module;
use std::cell::Cell;
use std::rc::Rc;

/// Classifications compiled per loop before it is given up.
const MAX_SIGS: usize = 4;
/// Failed checks (not `break`/`return`/the loop's end) after which a unit is given up.
const MAX_UNPLANNED: u32 = 64;

type UnitFn = unsafe extern "C" fn(*mut Ctx, *mut i64) -> i64;

pub(crate) struct Unit {
    f: UnitFn,
    plan: Plan,
    unplanned: Cell<u32>,
    dead: Cell<bool>,
    /// The spill buffer and row-view tables of the last run, kept for the next one (a loop entered once per
    /// call of its function would otherwise allocate them on every call). Taken while a run uses them, so a
    /// nested run of the same unit allocates its own.
    spare: std::cell::RefCell<Spare>,
}

#[derive(Default)]
struct Spare {
    buf: Vec<i64>,
    tbls: [Vec<i64>; super::rows::ROWS],
}

impl Unit {
    /// Keep `buf` for the next run.
    fn give_back(&self, buf: Vec<i64>) {
        if let Ok(mut s) = self.spare.try_borrow_mut() {
            s.buf = buf;
        }
    }
}

/// One classification of the loop's locals and its machine code (`None`: outside the subset).
pub(crate) struct Entry {
    sig: Vec<Class>,
    unit: Option<Rc<Unit>>,
}

#[derive(Default)]
pub(crate) struct Units {
    refs: Option<Rc<Vec<(&'static str, usize)>>>,
    list: Vec<Entry>,
}

fn debug(it: &Interpreter, site: &OsrSite, what: &str) {
    if it.jit_debug {
        let (Statement::While { condition: head, .. } | Statement::For { iterable: head, .. }) = site.stmt else { return };
        let at = crate::interpreter::eval_expr::expr_loc(head).map(|l| format!("{}:{}", l.file.display(), l.line)).unwrap_or_default();
        eprintln!("[jit] loop {at}: {what}");
    }
}

/// Try a `while` loop's machine code at its head. `Ok(true)`: it ran (the locals are back in their
/// slots and the iteration it stopped in has been finished); `Ok(false)`: it did not run.
pub(crate) fn enter(it: &mut Interpreter, b: usize, site: &OsrSite) -> R<bool> {
    let Some((reg, unit)) = unit_for(it, b, site, None) else { return Ok(false) };
    let Some((exit, buf, raised)) = execute(it, b, site, &reg, &unit, &[]) else { return Ok(false) };
    let path = &unit.plan.exits[exit].path;
    if !path.is_empty() {
        let r = resume(it, b, &reg, path, &buf, raised);
        unit.give_back(buf);
        r?;
    } else {
        unit.give_back(buf);
        close_body(it, b, &reg, site.stmt);
    }
    Ok(true)
}

/// The machine code stopped at the loop head: the last iteration's body locals are still in their slots (each
/// iteration overwrote the previous one). Release them as the closure engine does after every iteration, so an
/// `Rc` (a crate handle, a shared value) dies when the loop ends, not when the function returns.
fn close_body(it: &mut Interpreter, b: usize, reg: &Reg, s: S) {
    let body: &'static [S] = match s {
        Statement::While { body, .. } | Statement::For { body, .. } | Statement::Loop { body, .. } => &body[..],
        _ => return,
    };
    let _ = close_block(it, b, reg.block(body).as_deref(), Ok(()));
}

/// Try a range `for` loop's machine code with its counter at `i` (the end `hi` exclusive).
/// `Ok(Some(next))`: it ran, and the loop goes on from `next` (`hi` after a `break`).
pub(crate) fn enter_for(it: &mut Interpreter, b: usize, site: &OsrSite, i: i128, hi: i128, ty: IntTy) -> R<Option<i128>> {
    let t = Ty::Int(ty);
    let (Some(ib), Some(hb)) = (bits(&Value::Int(i, ty), t), bits(&Value::Int(hi, ty), t)) else {
        site.cool();
        return Ok(None);
    };
    let Some((reg, unit)) = unit_for(it, b, site, Some(ty)) else { return Ok(None) };
    let c = unit.plan.counter.expect("a for unit has a counter");
    let Some((exit, buf, raised)) = execute(it, b, site, &reg, &unit, &[(c, ib), (c + 1, hb)]) else { return Ok(None) };
    let at = int_value(buf[c], ty);
    let path = &unit.plan.exits[exit].path;
    if path.is_empty() {
        unit.give_back(buf);
        close_body(it, b, &reg, site.stmt);
        return Ok(Some(at)); // stopped at the head: the counter reached the end
    }
    let Statement::For { label, .. } = site.stmt else { unreachable!("a for site") };
    let info = reg.for_info(site.stmt).ok_or_else(internal)?;
    let r = resume(it, b, &reg, path, &buf, raised);
    unit.give_back(buf);
    let r = close_block(it, b, Some(&info.scope), r);
    Ok(Some(match loop_exit(r.map(|_| Value::Unit), label.as_deref())? {
        Some(_) => hi,
        None => at + 1,
    }))
}

fn internal() -> crate::interpreter::error::Ctl {
    crate::interpreter::error::semantic("internal: a machine-code loop resumed at an unregistered statement")
}

/// The machine code for the locals' current classification (`counter`: a `for` unit's counter
/// width), compiled on first use; `None`: not now (outside the subset, or given up).
fn unit_for(it: &mut Interpreter, b: usize, site: &OsrSite, counter: Option<IntTy>) -> Option<(Rc<Reg>, Rc<Unit>)> {
    if it.type_hint.is_some() {
        site.cool();
        return None;
    }
    let reg = site.reg.upgrade()?;
    let refs = site.units.borrow_mut().refs.get_or_insert_with(|| Rc::new(collect(site))).clone();
    // the classification a unit was compiled for, checked in place (no class is built on the way in)
    let found = site.units.borrow().list.iter().find(|e| sig_is(it, b, &reg, &refs, counter, &e.sig)).map(|e| e.unit.clone());
    let unit = match found {
        Some(u) => {
            // debug builds and `WJ_JIT_DEBUG` runs (the jit and fast-path gates) prove the in-place check is classify's
            if cfg!(debug_assertions) || it.jit_debug {
                assert!(site.units.borrow().list.iter().any(|e| e.sig == sig_of(it, b, &reg, &refs, counter)), "sig_is and classify disagree");
            }
            u
        }
        None => {
            let sig = sig_of(it, b, &reg, &refs, counter);
            if site.units.borrow().list.len() >= MAX_SIGS {
                it.fallback.jitrefused += 1;
                site.dead.set(true);
                return None;
            }
            let u = match compile(it, site, &reg, &refs, &sig) {
                Ok(u) => {
                    if u.plan.inlines {
                        it.fast.inline += 1;
                        debug(it, site, "inlines method calls");
                    }
                    debug(it, site, "compiled");
                    Some(Rc::new(u))
                }
                Err(why) => {
                    it.fallback.jitrefused += 1;
                    debug(it, site, &format!("outside the subset: {why}"));
                    None
                }
            };
            site.units.borrow_mut().list.push(Entry { sig, unit: u.clone() });
            u
        }
    };
    match unit.filter(|u| !u.dead.get()) {
        Some(u) => Some((reg, u)),
        None => {
            site.cool();
            None
        }
    }
}

/// The classification of the loop's locals now (and a `for` unit's counter width).
fn sig_of(it: &Interpreter, b: usize, reg: &Reg, refs: &[(&'static str, usize)], counter: Option<IntTy>) -> Vec<Class> {
    let mut sig: Vec<Class> = refs.iter().map(|(_, s)| classify(&it.rstack[b + s], reg.kind(*s))).collect();
    if let Some(t) = counter {
        sig.push(Class::Scalar(Ty::Int(t)));
    }
    sig
}

/// `sig_of(..) == *sig`, without building the classification.
fn sig_is(it: &Interpreter, b: usize, reg: &Reg, refs: &[(&'static str, usize)], counter: Option<IntTy>, sig: &[Class]) -> bool {
    if sig.len() != refs.len() + counter.is_some() as usize {
        return false;
    }
    if let Some(t) = counter {
        if sig[refs.len()] != Class::Scalar(Ty::Int(t)) {
            return false;
        }
    }
    refs.iter().zip(sig).all(|((_, s), c)| classify_is(&it.rstack[b + s], reg.kind(*s), c))
}

/// Run the machine code: fill the spill buffer (the live-ins, then `init`), call, write the exit's
/// variables back. `None`: a live-in no longer has the classified type (nothing ran). The third
/// value is what a closure-run statement raised, when the exit follows one.
fn execute(it: &mut Interpreter, b: usize, site: &OsrSite, reg: &Reg, unit: &Unit, init: &[(usize, i64)]) -> Option<(usize, Vec<i64>, Option<Ctl>)> {
    let n = unit.plan.nbuf.max(1);
    let rows = !unit.plan.row_shapes.is_empty();
    let (mut buf, tbls) = match unit.spare.try_borrow_mut() {
        Ok(mut s) if rows => (std::mem::take(&mut s.buf), std::mem::take(&mut s.tbls)),
        Ok(mut s) => (std::mem::take(&mut s.buf), Default::default()),
        Err(_) => Default::default(),
    };
    buf.clear();
    buf.resize(n, 0);
    for (i, slot, t) in &unit.plan.live {
        buf[*i] = read_bits(it, b + slot, reg.kind(*slot), *t)?;
    }
    for sp in &unit.plan.live_structs {
        load_struct(it, b + sp.slot, reg.kind(sp.slot), sp, &mut buf)?;
    }
    if unit.plan.inlines && !it.stack_room(INLINE_STACK) {
        return None; // the frames inlined calls stand for might not fit: the closures check
    }
    for (i, x) in init {
        buf[*i] = *x;
    }
    let mut ctx = Ctx::new(it as *mut Interpreter, b, &unit.plan, reg);
    ctx.row_tbls = tbls;
    // SAFETY: `f` is finalized code of the module `it.jit` keeps alive, with the `UnitFn` signature;
    // `ctx` and `buf` (`nbuf` slots) outlive the call.
    let exit = unsafe { (unit.f)(&mut ctx, buf.as_mut_ptr()) } as usize;
    let raised = ctx.raised.take();
    if rows {
        if let Ok(mut s) = unit.spare.try_borrow_mut() {
            s.tbls = std::mem::take(&mut ctx.row_tbls);
        }
    }
    it.fast.packedstore += ctx.view_stores as u64;
    it.fast.rowview += ctx.row_hits as u64;
    let e = &unit.plan.exits[exit];
    for sp in e.spills.iter().filter(|sp| sp.slot != NO_SLOT && !e.after) {
        write(it, b + sp.slot, reg.kind(sp.slot), value(buf[sp.buf], sp.ty), sp.fresh);
    }
    for sp in e.structs.iter().filter(|_| !e.after) {
        write(it, b + sp.slot, reg.kind(sp.slot), make_struct(sp, &buf), sp.fresh);
    }
    if !e.planned {
        it.fallback.jitexit += 1;
        let n = unit.unplanned.get() + 1;
        unit.unplanned.set(n);
        if n >= MAX_UNPLANNED {
            it.fallback.jitdead += 1;
            unit.dead.set(true);
            debug(it, site, "too many failed checks: back to the closures");
        }
    }
    Some((exit, buf, raised))
}

/// Stack bytes a unit with inlined calls must have left before it runs (far more than the frames).
const INLINE_STACK: usize = 1 << 20;

/// Read struct local `i` field by field into the spill buffer; `None`: it no longer has the shape.
pub(super) fn load_struct(it: &Interpreter, i: usize, kind: Option<SlotKind>, sp: &StructSpill, buf: &mut [i64]) -> Option<()> {
    let v = read(it, i, kind)?;
    let Value::Struct(s) = &v else { return None };
    if !same_name(&s.name, &sp.name) || s.fields.len() != sp.fields.len() || s.uid != 0 {
        return None;
    }
    for ((n, x), (m, k, t)) in s.fields.iter().zip(&sp.fields) {
        if !same_name(n, m) {
            return None;
        }
        buf[*k] = bits(x, *t)?;
    }
    Some(())
}

/// The struct value a struct local's fields make.
pub(super) fn make_struct(sp: &StructSpill, buf: &[i64]) -> Value {
    let fields = sp.fields.iter().map(|(n, k, t)| (n.clone(), value(buf[*k], *t))).collect();
    Value::Struct(Rc::new(crate::interpreter::value::StructVal { name: sp.name.clone(), fields, copy: sp.copy, uid: 0 }))
}

/// The bits of a scalar local of type `t` (`read` then `bits`, without cloning the value).
fn read_bits(it: &Interpreter, i: usize, kind: Option<SlotKind>, t: Ty) -> Option<i64> {
    match (kind?, &it.rstack[i]) {
        (SlotKind::Boxed, Value::Ref(p)) => p.slot.try_borrow().ok().and_then(|v| bits(&v, t)),
        (SlotKind::Boxed, _) => None,
        (_, v) => bits(v, t),
    }
}

/// A scalar local's value (a boxed one's walker slot holds it).
pub(super) fn read(it: &Interpreter, i: usize, kind: Option<SlotKind>) -> Option<Value> {
    match (kind?, &it.rstack[i]) {
        (SlotKind::Boxed, Value::Ref(p)) => p.slot.try_borrow().ok().map(|v| v.clone()),
        (SlotKind::Boxed, _) => None,
        (_, v) => Some(v.clone()),
    }
}

/// Write a variable back; a `fresh` boxed one (bound in the loop) gets a new walker slot.
pub(super) fn write(it: &mut Interpreter, i: usize, kind: Option<SlotKind>, v: Value, fresh: bool) {
    match kind {
        Some(SlotKind::Boxed) if fresh => it.rstack[i] = crate::interpreter::resolved::island::boxed(v),
        Some(SlotKind::Boxed) => {
            if let Value::Ref(p) = &it.rstack[i] {
                *p.slot.borrow_mut() = v;
            }
        }
        _ => crate::interpreter::resolved::fast::set(it, i, v),
    }
}

/// What a local holds: a scalar machine code keeps in a register (directly in its slot, or in its
/// walker slot when boxed), or an aggregate it reaches through helpers.
fn classify(v: &Value, kind: Option<SlotKind>) -> Class {
    let direct = match (kind, v) {
        (Some(SlotKind::Boxed), Value::Ref(p)) if p.path.is_empty() => p.slot.try_borrow().ok().and_then(|g| ty_of(&g)),
        (Some(SlotKind::Plain | SlotKind::Typed(_)), v) => ty_of(v),
        _ => None,
    };
    if let Some(t) = direct {
        return Class::Scalar(t);
    }
    let held = match (kind, v) {
        (Some(SlotKind::Boxed), Value::Ref(p)) if p.path.is_empty() => p.slot.try_borrow().ok().and_then(|g| value_class(&g)),
        (Some(SlotKind::Plain), v) => value_class(v),
        _ => None,
    };
    if let Some(c) = held {
        return c;
    }
    match super::rt::peek(v, |x| Some(class_of(x))) {
        Some(Class::Scalar(_)) | None => Class::Opaque,
        Some(c) => c,
    }
}

/// `classify(v, kind) == *c`, without building the class.
fn classify_is(v: &Value, kind: Option<SlotKind>, c: &Class) -> bool {
    let direct = match (kind, v) {
        (Some(SlotKind::Boxed), Value::Ref(p)) if p.path.is_empty() => p.slot.try_borrow().ok().and_then(|g| ty_of(&g)),
        (Some(SlotKind::Plain | SlotKind::Typed(_)), v) => ty_of(v),
        _ => None,
    };
    if let Some(t) = direct {
        return *c == Class::Scalar(t);
    }
    let held = match (kind, v) {
        (Some(SlotKind::Boxed), Value::Ref(p)) if p.path.is_empty() => p.slot.try_borrow().ok().and_then(|g| value_class_is(&g, c)),
        (Some(SlotKind::Plain), v) => value_class_is(v, c),
        _ => None,
    };
    if let Some(yes) = held {
        return yes;
    }
    super::rt::peek(v, |x| Some(class_of_is(x, c))).unwrap_or(*c == Class::Opaque)
}

/// The locals the loop names (its own `let`s shadow them inside; that is resolved when lowering).
fn collect(site: &OsrSite) -> Vec<(&'static str, usize)> {
    let mut names = Vec::new();
    stmt_names(site.stmt, &mut names);
    let mut out: Vec<(&'static str, usize)> = Vec::new();
    for n in names {
        if let (Some(s), false) = (site.slot_of(n), out.iter().any(|(m, _)| *m == n)) {
            out.push((n, s));
        }
    }
    out
}

fn stmt_names(s: S, out: &mut Vec<&'static str>) {
    match s {
        Statement::Let { value, .. } => expr_names(value, out),
        Statement::Assignment { target, value, .. } => {
            expr_names(target, out);
            expr_names(value, out);
        }
        Statement::Expression { expr, .. } | Statement::Return { value: Some(expr), .. } => expr_names(expr, out),
        Statement::If { condition, then_block, else_block, .. } => {
            expr_names(condition, out);
            then_block.iter().chain(else_block.iter().flatten()).for_each(|s| stmt_names(s, out));
        }
        Statement::While { condition, body, .. } | Statement::For { iterable: condition, body, .. } => {
            expr_names(condition, out);
            body.iter().for_each(|s| stmt_names(s, out));
        }
        Statement::Match { value, arms, .. } => {
            expr_names(value, out);
            arms.iter().for_each(|a| expr_names(a.body, out));
        }
        _ => {}
    }
}

fn expr_names(e: &'static Expression<'static>, out: &mut Vec<&'static str>) {
    match e {
        Expression::Identifier { name, .. } => out.push(name.as_str()),
        Expression::Binary { left, right, .. } | Expression::Index { object: left, index: right, .. } | Expression::Range { start: left, end: right, .. } => {
            expr_names(left, out);
            expr_names(right, out);
        }
        Expression::Unary { operand: x, .. } | Expression::Cast { expr: x, .. } | Expression::FieldAccess { object: x, .. } => expr_names(x, out),
        Expression::MethodCall { object, arguments, .. } => {
            expr_names(object, out);
            arguments.iter().for_each(|(_, a)| expr_names(a, out));
        }
        Expression::Block { statements, .. } => statements.iter().for_each(|s| stmt_names(s, out)),
        _ => {}
    }
}

fn compile(it: &mut Interpreter, site: &OsrSite, reg: &Reg, refs: &[(&'static str, usize)], sig: &[Class]) -> Result<Unit, String> {
    let mut refs: Vec<(&'static str, usize, Class)> = refs.iter().zip(sig).map(|((n, s), c)| (*n, *s, c.clone())).collect();
    if sig.len() > refs.len() {
        refs.push(("\0for", NO_SLOT, sig[refs.len()].clone())); // a `for` unit's counter width
    }
    let mut jit = super::take_jit(it).ok_or("no Cranelift backend")?;
    let r = (|| {
        let module = &mut jit.module;
        let ptr = module.target_config().pointer_type();
        let mut ctx = module.make_context();
        ctx.func.signature = lower::signature(ptr, module.target_config().default_call_conv);
        let mut fctx = FunctionBuilderContext::new();
        let plan = lower::lower(&mut ctx.func, &mut fctx, it, site.stmt, reg, &refs, ptr)?;
        let id = module.declare_anonymous_function(&ctx.func.signature).map_err(|e| format!("{e:?}"))?;
        module.define_function(id, &mut ctx).map_err(|e| format!("cranelift: {e:?}"))?;
        module.clear_context(&mut ctx);
        module.finalize_definitions().map_err(|e| format!("{e:?}"))?;
        let p = module.get_finalized_function(id);
        // SAFETY: the function was built with `lower::signature`, which is `UnitFn`'s.
        let f = unsafe { std::mem::transmute::<*const u8, UnitFn>(p) };
        Ok(Unit { f, plan, unplanned: Cell::new(0), dead: Cell::new(false), spare: Default::default() })
    })();
    it.jit = Some(jit);
    r
}

/// Finish the iteration the machine code stopped in, from the statement `path` names (`buf`: the
/// exit's spill buffer, for the counters of `for` loops on the path).
fn resume(it: &mut Interpreter, b: usize, reg: &Reg, path: &Path, buf: &[i64], raised: Option<Ctl>) -> R<()> {
    level(it, b, reg, path, buf, 0, raised)
}

/// Run block `path[l]` from its statement on (the one at `path[l + 1]`'s container is partly done:
/// its nested block is finished first), then release the block's locals as `scoped` does.
fn level(it: &mut Interpreter, b: usize, reg: &Reg, path: &Path, buf: &[i64], l: usize, raised: Option<Ctl>) -> R<()> {
    let Level { list, idx, for_ } = path[l];
    let r = (|| {
        let mut start = idx;
        if l + 1 < path.len() {
            let s = list[idx];
            let r = level(it, b, reg, path, buf, l + 1, raised);
            match s {
                Statement::While { label, .. } => {
                    if loop_exit(r.map(|_| Value::Unit), label.as_deref())?.is_none() {
                        // the inner loop goes on from its condition
                        code(reg, s)?(it, b)?;
                    }
                }
                Statement::For { label, .. } => {
                    let info = reg.for_info(s).ok_or_else(internal)?;
                    let r = close_block(it, b, Some(&info.scope), r);
                    if loop_exit(r.map(|_| Value::Unit), label.as_deref())?.is_none() {
                        let st = for_.ok_or_else(internal)?;
                        let hi = int_value(buf[st.end], st.ty) + st.inclusive as i128;
                        iterations(it, b, reg, s, &info, int_value(buf[st.counter], st.ty) + 1, hi, st.ty)?;
                    }
                }
                _ => r?,
            }
            start = idx + 1;
        } else if let Some(e) = raised {
            return Err(e); // the closure-run statement before `idx` raised: the block ends with it
        }
        statements(it, b, reg, &list[start..])
    })();
    close_block(it, b, reg.block(list).as_deref(), r)
}

fn statements(it: &mut Interpreter, b: usize, reg: &Reg, list: &[S]) -> R<()> {
    for s in list {
        it.mark(loc(s));
        code(reg, s)?(it, b)?;
    }
    Ok(())
}

/// The remaining iterations `i..hi` of a `for` loop whose body machine code left (the closure
/// engine's loop: bind the variable, run the body, release its scope).
#[allow(clippy::too_many_arguments)]
fn iterations(it: &mut Interpreter, b: usize, reg: &Reg, s: S, info: &ForInfo, mut i: i128, hi: i128, ty: IntTy) -> R<()> {
    let Statement::For { body, label, .. } = s else { return Err(internal()) };
    while i < hi {
        if let Some(slot) = info.slot {
            let v = Value::Int(i, ty);
            match reg.kind(slot) {
                Some(SlotKind::Boxed) => it.rstack[b + slot] = crate::interpreter::resolved::island::boxed(v),
                _ => crate::interpreter::resolved::fast::set(it, b + slot, v),
            }
        }
        let r = statements(it, b, reg, body);
        let r = close_block(it, b, reg.block(body).as_deref(), r);
        let r = close_block(it, b, Some(&info.scope), r);
        if loop_exit(r.map(|_| Value::Unit), label.as_deref())?.is_some() {
            break;
        }
        i += 1;
    }
    Ok(())
}

fn code(reg: &Reg, s: S) -> R<crate::interpreter::resolved::osr::SCode> {
    reg.code(s).ok_or_else(internal)
}

//! Runtime helpers machine-code loops call for what lives in `Value`s: elements of a `Vec`, fields
//! of a struct, `push`, `len`, and integer width combinations without an inline fast path.
//!
//! Each helper takes the happy path or reports failure (`Ctx::fail`) before it changes anything;
//! the loop then stops at the statement and the closure engine runs it (and produces the walker's
//! panic, if there is one). Stores follow the walker's `Place::with_mut` (copy-on-write along the
//! path, `retag` for `=`); reads follow references without cloning.

use super::kinds::{bits, from_code, int_value, mutant, value, Ty};
use crate::interpreter::engine::Interpreter;
use crate::interpreter::eval_stmt::retag;
use crate::interpreter::ops::int_binary;
use crate::interpreter::place::descend;
use crate::interpreter::value::{Proj, Value};
use crate::parser::BinaryOp;
use std::rc::Rc;

/// What machine code passes every helper: the interpreter, the frame base, and the failure flag.
#[repr(C)]
pub(crate) struct Ctx {
    pub(crate) it: *mut Interpreter,
    pub(crate) base: usize,
    pub(crate) fail: i64,
    /// The running unit's plan and registry (for closure-run statements).
    pub(crate) plan: *const super::lower::Plan,
    pub(crate) reg: *const crate::interpreter::resolved::osr::Reg,
    /// What a closure-run statement raised (the unit then leaves after it).
    pub(crate) raised: Option<crate::interpreter::error::Ctl>,
    /// Locals reached through references, resolved to the cell that holds their value (`with_cell`): the chain
    /// is walked once per unit run instead of on every element access. Cleared before every closure-run
    /// statement, the only statements that can rebind a reference.
    /// `None`: the local holds its value directly (also remembered, so a plain local is looked at once).
    pub(crate) cells: Vec<(i64, Option<crate::interpreter::value::Slot>)>,
    /// `cells` found again without a scan: by `slot % CELL_HITS`, the slot plus one (0: empty) and its index in `cells`
    /// (`NO_CELL`: the local holds its value directly). Cleared with `cells`, never filled with anything but what `cells` holds.
    pub(crate) cell_hit: [(i64, u32); CELL_HITS],
    /// Packed vectors machine code stores into without a call (`View`), by `slot % VIEWS`.
    pub(crate) views: [View; VIEWS],
    /// Stores machine code made through a view (added to `FastHits::packedstore` when the unit leaves).
    pub(crate) view_stores: i64,
    /// Vectors of structs whose fields machine code reads and writes in place (`rows.rs`), by `slot % ROWS`.
    pub(crate) rows: [super::rows::RowView; super::rows::ROWS],
    /// The element tables the row views point into.
    pub(crate) row_tbls: [Vec<i64>; super::rows::ROWS],
    /// Elements a row view build may still scan (`rows.rs`).
    pub(crate) row_credit: i64,
    /// Field accesses machine code made through a row view (added to `FastHits::rowview`).
    pub(crate) row_hits: i64,
    /// Versioned loops entered through their fast copy / their slow copy (added to `FastHits::hoisted` / `hoistfail`).
    pub(crate) hoisted: i64,
    pub(crate) hoistfail: i64,
    /// Whether the last `map_get` found its key (read by machine code after the call).
    pub(crate) found: i64,
    /// Map lookups / inserts the lean helpers served (added to `FastHits::mapleanget` / `mapleanins` when the unit leaves).
    pub(crate) lean_gets: i64,
    /// Element reads machine code made through a view (added to `FastHits::packedread` when the unit leaves).
    pub(crate) view_reads: i64,
    pub(crate) lean_inss: i64,
    /// The lookups among `lean_gets` a typed array (`IntDense`) answered.
    pub(crate) lean_typed: i64,
}

impl Ctx {
    pub(crate) fn new(it: *mut Interpreter, base: usize, plan: *const super::lower::Plan, reg: *const crate::interpreter::resolved::osr::Reg) -> Ctx {
        let (rows, row_tbls) = (Default::default(), Default::default());
        Ctx { it, base, fail: 0, plan, reg, raised: None, cells: Vec::new(), cell_hit: [(0, 0); CELL_HITS], views: Default::default(), view_stores: 0, rows, row_tbls, row_credit: super::rows::CREDIT_START, row_hits: 0, hoisted: 0, hoistfail: 0, found: 0, lean_gets: 0, view_reads: 0, lean_inss: 0, lean_typed: 0 }
    }
}

/// The storage of a packed vector a store reached (`index_set`): machine code checks `key` and `len`
/// and writes the element at `ptr` itself. Valid until anything else could move, share or unpack that
/// storage: closure code (`run_stmt`), `push`, or a store the packing does not take clears every view.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct View {
    /// `view_key(slot, element type)`; 0: empty.
    pub(crate) key: i64,
    pub(crate) ptr: i64,
    pub(crate) len: i64,
}

pub(crate) const VIEWS: usize = 4;
const CELL_HITS: usize = 8;
const NO_CELL: u32 = u32::MAX;
pub(crate) const FAIL: i32 = std::mem::offset_of!(Ctx, fail) as i32;
pub(crate) const VIEW_AT: i32 = std::mem::offset_of!(Ctx, views) as i32;
pub(crate) const VIEW_READS: i32 = std::mem::offset_of!(Ctx, view_reads) as i32;
pub(crate) const VIEW_STORES: i32 = std::mem::offset_of!(Ctx, view_stores) as i32;
pub(crate) const ROW_AT: i32 = std::mem::offset_of!(Ctx, rows) as i32;
pub(crate) const ROW_HITS: i32 = std::mem::offset_of!(Ctx, row_hits) as i32;
pub(crate) const HOISTED: i32 = std::mem::offset_of!(Ctx, hoisted) as i32;
pub(crate) const HOISTFAIL: i32 = std::mem::offset_of!(Ctx, hoistfail) as i32;
pub(crate) const FOUND: i32 = std::mem::offset_of!(Ctx, found) as i32;

/// The key of a view: the local's slot and the stored element type (never 0).
pub(crate) fn view_key(slot: usize, t: Ty) -> i64 {
    ((slot as i64 + 1) << 12) | super::kinds::code(t)
}

/// Set in the key of a view built by a READ (`index_get`): its storage was uniquely owned when it was built, but a
/// store through it would not have made it unique, so only reads may use it (a store compares the key exactly).
pub(crate) const READ_VIEW: i64 = 1 << 11;

/// Element types machine code can store into a packed vector as their bits: a bool (one byte), an
/// `i64` (the packing's own width, so no normalization) and a float.
pub(crate) fn view_elem(t: Ty) -> Option<i64> {
    match t {
        Ty::Bool => Some(1),
        Ty::Int(crate::interpreter::int::IntTy::I64) | Ty::Flt(_) => Some(8),
        _ => None,
    }
}

/// Run `f` with the local at `slot`; a `None` sets the failure flag.
unsafe fn with_slot(c: *mut Ctx, slot: i64, f: impl FnOnce(&mut Value) -> Option<i64>) -> i64 {
    // SAFETY: `c` is the context `osr::enter` built for this call; the interpreter outlives it and
    // nothing else touches it while machine code runs.
    let ctx = unsafe { &mut *c };
    let it = unsafe { &mut *ctx.it };
    let i = ctx.base + slot as usize;
    match it.rstack.get_mut(i).and_then(f) {
        Some(x) => x,
        None => {
            ctx.fail = 1;
            0
        }
    }
}

/// Read what `v` holds, following references.
pub(crate) fn peek<T>(v: &Value, f: impl FnOnce(&Value) -> Option<T>) -> Option<T> {
    match v {
        Value::Ref(p) => {
            let g = p.slot.try_borrow().ok()?;
            let mut cur: &Value = &g;
            for pr in &p.path {
                cur = step(cur, pr)?;
            }
            peek(cur, f)
        }
        Value::Rc(_) => None,
        v => f(v),
    }
}

fn step<'v>(v: &'v Value, p: &Proj) -> Option<&'v Value> {
    match (v, p) {
        (Value::Struct(s), Proj::Field(n)) => s.fields.iter().find(|(m, _)| **m == **n).map(|(_, v)| v),
        (Value::Vec(xs), Proj::Index(i)) => xs.get(*i),
        (Value::Tuple(t), Proj::Index(i)) => t.get(*i),
        _ => None,
    }
}

/// Mutate what `v` holds, following references (the walker's `Place::with_mut`).
fn poke<T>(v: &mut Value, f: impl FnOnce(&mut Value) -> Option<T>) -> Option<T> {
    match v {
        Value::Ref(p) => {
            let mut g = p.slot.try_borrow_mut().ok()?;
            let mut cur: &mut Value = &mut g;
            for pr in &p.path {
                cur = descend(cur, pr).ok()?;
            }
            poke(cur, f)
        }
        Value::Rc(_) => None,
        v => f(v),
    }
}

/// An index value (`i` at type code `t`) as the walker checks it: negative is out of bounds.
fn index(i: i64, t: i64) -> Option<usize> {
    match from_code(t)? {
        Ty::Int(t) => usize::try_from(int_value(i, t)).ok(),
        _ => None,
    }
}

fn name(p: i64, n: i64) -> &'static str {
    // SAFETY: `p`/`n` are the pointer and length of a field name in the program's AST, which lives
    // for the whole run.
    unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(p as *const u8, n as usize)) }
}

/// A field name as machine code names it: the text, the position it had at entry and the address
/// of the interned name it had there (equal addresses are equal names: no text comparison).
#[derive(Clone, Copy)]
struct FieldRef {
    name: &'static str,
    hint: usize,
    id: usize,
}

impl FieldRef {
    fn new(p: i64, n: i64, hint: i64, id: i64) -> FieldRef {
        FieldRef { name: name(p, n), hint: hint as usize, id: id as usize }
    }
    fn is(&self, n: &Rc<str>) -> bool {
        n.as_ptr() as usize == self.id || **n == *self.name
    }
}

fn field(v: &Value, f: FieldRef) -> Option<&Value> {
    let Value::Struct(s) = v else { return None };
    match s.fields.get(f.hint) {
        Some((n, x)) if f.is(n) => Some(x),
        _ => s.fields.iter().find(|(n, _)| **n == *f.name).map(|(_, x)| x),
    }
}

fn field_mut(v: &mut Value, f: FieldRef) -> Option<&mut Value> {
    let Value::Struct(s) = v else { return None };
    let s = Rc::make_mut(s);
    let i = match s.fields.get(f.hint) {
        Some((n, _)) if f.is(n) => f.hint,
        _ => s.fields.iter().position(|(n, _)| **n == *f.name)?,
    };
    Some(&mut s.fields[i].1)
}

/// `=` into `slot` (the walker's `retag`: a literal keeps the old value's width).
fn store(slot: &mut Value, x: i64, t: i64) -> Option<i64> {
    let new = value(x, from_code(t)?);
    let old = std::mem::replace(slot, Value::Unit);
    *slot = retag(old, new);
    Some(0)
}

/// `v[i]` (an element of type `want`).
pub(crate) unsafe extern "C" fn index_get(c: *mut Ctx, slot: i64, i: i64, it: i64, want: i64) -> i64 {
    // SAFETY: as in `with_slot`.
    let off_by_one = unsafe { mutant(&*(*c).it, "bounds") };
    let direct_off = unsafe { (*(*c).it).fast_off("packedidx") };
    let mut direct = false;
    let view_off = unsafe { (*(*c).it).fast_off("packedread") };
    // test-only mutant `readviewshared` (debug builds): a read view of a body someone else shares
    let shared_ok = unsafe { mutant(&*(*c).it, "readviewshared") };
    let mut fill = None;
    let r = unsafe {
        with_cell(c, slot, |v| {
            let (k, want) = (index(i, it)?, from_code(want)?);
            match v {
                Value::Vec(xs) if off_by_one => bits(&xs.get_value(k).or_else(|| xs.get_value(xs.len().checked_sub(1)?))?, want),
                Value::Vec(xs) => match packed_get(xs, k, want).filter(|_| !direct_off) {
                    Some(r) => {
                        direct = true;
                        if !view_off {
                            fill = read_view(xs, want, shared_ok).map(|(p, n)| (want, p, n));
                        }
                        r
                    }
                    None => bits(&xs.get_value(k)?, want),
                },
                _ => None,
            }
        })
    };
    if direct {
        unsafe { (*(*c).it).fast.packedidx += 1 };
    }
    // SAFETY: as above.
    let ctx = unsafe { &mut *c };
    if let (Some((t, ptr, len)), 0) = (fill, ctx.fail) {
        // a store view of this vector (unique, written through) serves reads too: keep it
        let (at, key) = (slot as usize % VIEWS, view_key(slot as usize, t));
        if ctx.views[at].key & !READ_VIEW != key {
            ctx.views[at] = View { key: key | READ_VIEW, ptr, len };
        }
    }
    r
}

/// The storage of a packed vector that machine code may READ without a call (`View`, `READ_VIEW`): only when the body
/// is uniquely owned (nothing else can share it, so nothing writes it but this local, whose writes keep it in place),
/// packed with exactly type `want`, and of a width `view_elem` reads. `None`: no view.
fn read_view(xs: &Rc<crate::interpreter::vecbody::VecBody>, want: Ty, shared_ok: bool) -> Option<(i64, i64)> {
    use crate::interpreter::vecbody::Packed;
    if !shared_ok && (Rc::strong_count(xs) != 1 || Rc::weak_count(xs) != 0) {
        return None;
    }
    view_elem(want)?;
    match (xs.packed()?, want) {
        (Packed::Bool(v), Ty::Bool) => Some((v.as_ptr() as i64, v.len() as i64)),
        (Packed::Int(v, t), Ty::Int(w)) if *t == w => Some((v.as_ptr() as i64, v.len() as i64)),
        (Packed::Float(v, t), Ty::Flt(w)) if *t == w => Some((v.as_ptr() as i64, v.len() as i64)),
        _ => None,
    }
}

/// `v[i].f`.
pub(crate) unsafe extern "C" fn index_field_get(c: *mut Ctx, slot: i64, i: i64, it: i64, fp: i64, fl: i64, hint: i64, id: i64, want: i64) -> i64 {
    unsafe {
        with_cell(c, slot, |v| {
            let (k, want, f) = (index(i, it)?, from_code(want)?, FieldRef::new(fp, fl, hint, id));
            match v {
                Value::Vec(xs) => bits(field(xs.get(k)?, f)?, want),
                _ => None,
            }
        })
    }
}

/// `s.f`.
pub(crate) unsafe extern "C" fn field_get(c: *mut Ctx, slot: i64, fp: i64, fl: i64, hint: i64, id: i64, want: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let (want, f) = (from_code(want)?, FieldRef::new(fp, fl, hint, id));
            peek(v, |v| bits(field(v, f)?, want))
        })
    }
}

/// `v[i] = x`.
pub(crate) unsafe extern "C" fn index_set(c: *mut Ctx, slot: i64, i: i64, it: i64, x: i64, xt: i64) -> i64 {
    // SAFETY: as in `with_slot`; the interpreter is not borrowed across the call below.
    // an element replaced through the helper may not be the struct a row view points into
    unsafe { (*c).clear_rows() };
    let (direct_off, view_off) = unsafe { ((*(*c).it).fast_off("packedidx"), (*(*c).it).fast_off("packedstore")) };
    let mut direct = None;
    let r = unsafe {
        with_cell_mut(c, slot, |v| {
            let k = index(i, it)?;
            let t = from_code(xt)?;
            match v {
                Value::Vec(xs) => {
                    if !direct_off {
                        if let Some(view) = packed_set(xs, k, x, t) {
                            direct = Some((t, view));
                            return Some(0);
                        }
                    }
                    if k >= xs.len() {
                        return None;
                    }
                    // the walker's `retag` against the old element, then a plain replacement
                    let new = retag(xs.get_value(k)?, value(x, t));
                    Rc::make_mut(xs).set_value(k, new);
                    Some(0)
                }
                _ => None,
            }
        })
    };
    // SAFETY: as above.
    let ctx = unsafe { &mut *c };
    match direct {
        Some((t, (ptr, len))) => {
            unsafe { (*ctx.it).fast.packedidx += 1 };
            if !view_off && view_elem(t).is_some() {
                ctx.views[slot as usize % VIEWS] = View { key: view_key(slot as usize, t), ptr, len };
            }
        }
        // the element may have unpacked the vector or replaced its storage
        None => ctx.views = Default::default(),
    }
    r
}

/// `v[i].f = x`.
pub(crate) unsafe extern "C" fn index_field_set(c: *mut Ctx, slot: i64, i: i64, it: i64, fp: i64, fl: i64, hint: i64, id: i64, x: i64, xt: i64) -> i64 {
    // the store may retag the field (or copy what a view points into)
    unsafe { (*c).clear_rows() };
    unsafe {
        with_cell_mut(c, slot, |v| {
            let (k, f) = (index(i, it)?, FieldRef::new(fp, fl, hint, id));
            match v {
                // the field must exist before anything is made unique (as `descend` checks it)
                Value::Vec(xs) if k < xs.len() && field(&xs[k], f).is_some() => store(field_mut(&mut Rc::make_mut(xs)[k], f)?, x, xt),
                _ => None,
            }
        })
    }
}

/// `s.f = x`.
pub(crate) unsafe extern "C" fn field_set(c: *mut Ctx, slot: i64, fp: i64, fl: i64, hint: i64, id: i64, x: i64, xt: i64) -> i64 {
    // the struct may be an element a row view points into (through a reference)
    unsafe { (*c).clear_rows() };
    unsafe {
        with_slot(c, slot, |v| {
            let f = FieldRef::new(fp, fl, hint, id);
            poke(v, |v| {
                field(v, f)?;
                store(field_mut(v, f)?, x, xt)
            })
        })
    }
}

/// `push` onto a packed body whose packing has exactly the pushed type (`xt`): what `push_value` does for that case
/// (`Packed::put` at the end, the `Value` cache dropped), after the same `Rc::make_mut`. `false`: nothing pushed (an
/// unpacked or empty body, another type, a value the packing cannot hold) and the general helper decides.
fn lean_push(xs: &mut Rc<crate::interpreter::vecbody::VecBody>, x: i64, xt: i64) -> bool {
    use crate::interpreter::vecbody::{Packed, VecBody};
    let Some(t) = from_code(xt) else { return false };
    let fits = matches!((xs.packed(), t), (Some(Packed::Bool(_)), Ty::Bool))
        || matches!((xs.packed(), t), (Some(Packed::Int(_, w)), Ty::Int(u)) if *w == u)
        || matches!((xs.packed(), t), (Some(Packed::Float(_, w)), Ty::Flt(u)) if *w == u);
    if !fits {
        return false;
    }
    let VecBody::Packed(p, cache) = Rc::make_mut(xs) else { return false };
    match (p, t) {
        (Packed::Bool(v), _) => v.push(x != 0),
        (Packed::Int(v, _), Ty::Int(u)) => match i64::try_from(int_value(x, u)) {
            Ok(n) => v.push(n),
            Err(_) => return false,
        },
        (Packed::Float(v, _), _) => v.push(f64::from_bits(x as u64)),
        _ => return false,
    }
    cache.take();
    true
}

/// `v.push(x)`.
pub(crate) unsafe extern "C" fn push(c: *mut Ctx, slot: i64, x: i64, xt: i64) -> i64 {
    // SAFETY: as in `with_slot`; the interpreter is not borrowed across the call below.
    // a push may move a packed vector's storage
    unsafe {
        (*c).views = Default::default();
        (*c).clear_rows();
    }
    let off = unsafe { (*(*c).it).fast_off("packed") };
    let mut packed = false;
    // a packed vector of the pushed element's type: the element appended to the packing, no `Value` (`lean_push`)
    if !off && !unsafe { (*(*c).it).fast_off("leanpush") } {
        if let Some(Some(true)) = unsafe { lean_value(c, slot, |v| match v {
            Value::Vec(xs) => Some(lean_push(xs, x, xt)),
            _ => None,
        }) } {
            unsafe { (*(*c).it).fast.packed += 1 };
            return 0;
        }
    }
    let r = unsafe {
        with_slot(c, slot, |v| {
            let x = value(x, from_code(xt)?);
            poke(v, |v| match v {
                Value::Vec(xs) => {
                    let body = Rc::make_mut(xs);
                    if off {
                        body.push(x);
                    } else {
                        body.push_value(x);
                        packed = body.packed().is_some();
                    }
                    Some(0)
                }
                _ => None,
            })
        })
    };
    if packed {
        unsafe { (*(*c).it).fast.packed += 1 };
    }
    r
}

/// The integer type of the key machine code passes a map helper (its value is `int_value` at that type).
fn int_ty(kt: i64) -> Option<crate::interpreter::int::IntTy> {
    match from_code(kt)? {
        super::kinds::Ty::Int(t) => Some(t),
        _ => None,
    }
}

/// The map the local at `slot` holds, through at most plain references (`local_cell`, resolved once per unit run
/// and cleared before every closure-run statement, the only statements that can rebind a reference): the lean map
/// helpers' shape guard. The cell's CONTENT is read on every call (nothing of the map is cached), so a map replaced,
/// cleared or mutated by closure-run code is seen by the next call. A local that is not a map reached this way (a
/// projection, a lock guard, `Rc`, a non-map) takes the general helper.
#[inline(always)]
unsafe fn lean_map<R>(c: *mut Ctx, slot: i64, f: impl FnOnce(&mut Rc<crate::interpreter::kmap::KMap>) -> R) -> Option<R> {
    // SAFETY: as in `with_slot`; `f` sees only the map, never the context.
    if let Some(i) = unsafe { local_cell(c, slot) } {
        let ctx = unsafe { &mut *c };
        let cell = ctx.cells[i].1.as_ref().expect("resolved");
        let mut g = cell.try_borrow_mut().ok()?;
        return match &mut *g {
            Value::Map(km) => Some(f(km)),
            _ => None,
        };
    }
    let ctx = unsafe { &*c };
    let it = unsafe { &mut *ctx.it };
    match it.rstack.get_mut(ctx.base + slot as usize) {
        Some(Value::Map(km)) => Some(f(km)),
        _ => None,
    }
}

/// The local at `slot` (directly or through plain references, `local_cell`) handed to `f`, which answers `None` for a
/// value of a shape it does not take. `None` when the local cannot be reached this way.
#[inline(always)]
unsafe fn lean_value<R>(c: *mut Ctx, slot: i64, f: impl FnOnce(&mut Value) -> Option<R>) -> Option<Option<R>> {
    // SAFETY: as in `with_slot`; `f` sees only the map, never the context.
    if let Some(i) = unsafe { local_cell(c, slot) } {
        let ctx = unsafe { &mut *c };
        let cell = ctx.cells[i].1.as_ref().expect("resolved");
        let mut g = cell.try_borrow_mut().ok()?;
        return Some(f(&mut g));
    }
    let ctx = unsafe { &*c };
    let it = unsafe { &mut *ctx.it };
    Some(f(it.rstack.get_mut(ctx.base + slot as usize)?))
}

/// The bits of a scalar map value at type `want`: `bits`, inlined (the lean helpers call it once per lookup).
#[inline(always)]
fn lean_bits(x: &Value, want: i64) -> Option<i64> {
    match (x, from_code(want)?) {
        (Value::Int(i, vt), Ty::Int(t)) if *vt == t => {
            if super::kinds::unsigned_repr(t) {
                u64::try_from(*i).ok().map(|u| u as i64)
            } else {
                i64::try_from(*i).ok()
            }
        }
        (x, want) => bits(x, want),
    }
}

/// `m.get(&k)` of a map local (`mapfast::MapGet`'s lookup): the value's bits at type `want` and `Ctx::found`.
/// Fails, changing nothing, when the local holds no map or the value is not a scalar of type `want`.
/// A map the local reaches (directly or through plain references) is read without the `with_slot`/`peek` machinery (`lean_map`).
pub(crate) unsafe extern "C" fn map_get(c: *mut Ctx, slot: i64, k: i64, kt: i64, want: i64) -> i64 {
    if let (Some(t), false) = (int_ty(kt), cfg!(debug_assertions) && unsafe { (*(*c).it).fast_off("mapleanget") }) {
        // SAFETY: as in `with_slot`.
        let miss = mutant(unsafe { &*(*c).it }, "mapleanfound");
        let r = unsafe {
            lean_map(c, slot, |km| match km.typed_get(int_value(k, t)) {
                // a typed array (`KMap::IntDense`): the bits are the stored word, the value is built only to be checked
                Some(None) => Some((0, miss as i64, 1)),
                // the array's own type is the wanted one (signed, so the bits are the word): no `Value` built
                Some(Some((x, vt))) if from_code(want) == Some(Ty::Int(vt)) => Some((x, 1, 1)),
                Some(Some((x, vt))) => lean_bits(&Value::Int(x as i128, vt), want).map(|b| (b, 1, 1)),
                None => match km.get_int(int_value(k, t), t) {
                    // `miss` is the test-only mutant `mapleanfound`: an absent key reads as present (value 0)
                    None => Some((0, miss as i64, 0)),
                    Some(x) => lean_bits(&x, want).map(|b| (b, 1, 0)),
                },
            })
        };
        // SAFETY: as in `with_slot`.
        let ctx = unsafe { &mut *c };
        match r {
            Some(Some((b, found, typed))) => {
                ctx.found = found;
                ctx.lean_gets += 1;
                ctx.lean_typed += typed;
                return b;
            }
            // a map whose value is not a scalar of `want`: fails, nothing changed
            Some(None) => {
                ctx.found = 0;
                ctx.fail = 1;
                return 0;
            }
            None => {}
        }
    }
    unsafe { map_get_general(c, slot, k, kt, want) }
}

/// `map_get` for a map behind references (or a local that holds no map: fails).
unsafe fn map_get_general(c: *mut Ctx, slot: i64, k: i64, kt: i64, want: i64) -> i64 {
    let mut found = 0;
    let r = unsafe {
        with_slot(c, slot, |v| {
            peek(v, |v| match v {
                Value::Map(km) => match int_ty(kt).map(|t| km.get_int(int_value(k, t), t))? {
                    None => Some(0),
                    Some(x) => {
                        let b = bits(&x, from_code(want)?)?;
                        found = 1;
                        Some(b)
                    }
                },
                _ => None,
            })
        })
    };
    // SAFETY: as in `with_slot`.
    unsafe { (*c).found = found };
    r
}

/// `m.insert(k, x)` statement of a map local (`mapfast::insert_stmt`): the map made unique, the key and the
/// value stored as the walker's values. Fails, changing nothing, when the local holds no map.
/// A map the local reaches (directly or through plain references) is written without the `with_slot`/`poke` machinery (`lean_map`).
pub(crate) unsafe extern "C" fn map_insert(c: *mut Ctx, slot: i64, k: i64, kt: i64, x: i64, xt: i64) -> i64 {
    if let (Some(t), Some(xty), false) = (int_ty(kt), from_code(xt), cfg!(debug_assertions) && unsafe { (*(*c).it).fast_off("mapleanins") }) {
        let done = unsafe {
            lean_map(c, slot, |km| {
                let km = Rc::make_mut(km);
                // a typed array of this value type takes the bits in place (a signed 64-bit type: the bits are the word)
                if let Ty::Int(xi) = xty {
                    if km.set_int_bits(int_value(k, t), t, x, xi) {
                        return;
                    }
                }
                km.set_int(int_value(k, t), t, value(x, xty))
            })
        };
        if done.is_some() {
            unsafe { (*c).lean_inss += 1 };
            return 0;
        }
    }
    unsafe { map_insert_general(c, slot, k, kt, x, xt) }
}

/// `map_insert` for a map behind references (or a local that holds no map: fails).
unsafe fn map_insert_general(c: *mut Ctx, slot: i64, k: i64, kt: i64, x: i64, xt: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            let t = int_ty(kt)?;
            let x = value(x, from_code(xt)?);
            poke(v, |v| match v {
                Value::Map(km) => {
                    Rc::make_mut(km).set_int(int_value(k, t), t, x);
                    Some(0)
                }
                _ => None,
            })
        })
    }
}

/// `v.len()` of a `Vec`.
pub(crate) unsafe extern "C" fn len(c: *mut Ctx, slot: i64) -> i64 {
    unsafe {
        with_slot(c, slot, |v| {
            peek(v, |v| match v {
                Value::Vec(xs) => Some(xs.len() as i64),
                _ => None,
            })
        })
    }
}

pub(crate) const OPS: [BinaryOp; 8] =
    [BinaryOp::Add, BinaryOp::Sub, BinaryOp::Mul, BinaryOp::Div, BinaryOp::Mod, BinaryOp::BitAnd, BinaryOp::BitOr, BinaryOp::BitXor];

/// `a op b` for width combinations without an inline path: the walker's `int_binary`.
pub(crate) unsafe extern "C" fn int_op(c: *mut Ctx, op: i64, a: i64, at: i64, b: i64, bt: i64, want: i64) -> i64 {
    let r = (|| {
        let (Ty::Int(at), Ty::Int(bt)) = (from_code(at)?, from_code(bt)?) else { return None };
        let v = int_binary(*OPS.get(op as usize)?, int_value(a, at), at, int_value(b, bt), bt).ok()?;
        bits(&v, from_code(want)?)
    })();
    match r {
        Some(x) => x,
        None => {
            // SAFETY: as in `with_slot`.
            unsafe { (*c).fail = 1 };
            0
        }
    }
}

/// Run closure-run statement `k` of the plan: its variables go to their slots, its code runs (marked
/// as `seq` marks it), and they come back to the spill buffer. Nonzero: leave after the statement (it
/// raised, `raised` holds what; or a variable no longer has its type).
pub(crate) unsafe extern "C" fn run_stmt(c: *mut Ctx, buf: i64, k: i64) -> i64 {
    // SAFETY: as in `with_slot`; `plan`/`reg` outlive the call, `buf` has `nbuf` slots.
    let ctx = unsafe { &mut *c };
    // closure code may rebind a reference the unit resolved (`r = &mut b`, `Ctx::cells`): resolve again after it
    // (test-only mutant `vecstale`, debug builds: keep the stale resolution)
    if !mutant(unsafe { &*ctx.it }, "vecstale") {
        ctx.cells.clear();
        // (test-only mutant `cellhitstale`: the hit table keeps pointing at the old resolution)
        if !mutant(unsafe { &*ctx.it }, "cellhitstale") {
            ctx.cell_hit = [(0, 0); CELL_HITS];
        }
    }
    ctx.views = Default::default();
    ctx.clear_rows();
    let (it, plan, reg, b) = unsafe { (&mut *ctx.it, &*ctx.plan, &*ctx.reg, ctx.base) };
    let buf = buf as *mut i64;
    let st = &plan.stmts[k as usize];
    let at = |i: usize| unsafe { buf.add(i) };
    for sp in st.spills.iter().filter(|sp| sp.slot != super::lower::NO_SLOT) {
        let v = value(unsafe { *at(sp.buf) }, sp.ty);
        super::osr::write(it, b + sp.slot, reg.kind(sp.slot), v, false);
    }
    {
        // SAFETY: `buf` has `nbuf` slots, every struct field index is one of them.
        let all = unsafe { std::slice::from_raw_parts_mut(buf, plan.nbuf.max(1)) };
        // test-only mutant `box`: the first write after a struct `let` fills the previous binding's box
        let reuse = mutant(it, "box");
        for sp in &st.structs {
            super::osr::write(it, b + sp.slot, reg.kind(sp.slot), super::osr::make_struct(sp, all), sp.fresh && !reuse);
        }
    }
    it.mark(crate::interpreter::resolved::osr::loc(st.stmt));
    let r = match reg.code(st.stmt) {
        Some(code) => code(it, b),
        None => Err(crate::interpreter::error::semantic("internal: a machine-code loop ran an unregistered statement")),
    };
    if let Err(e) = r {
        ctx.raised = Some(e);
        return 1;
    }
    for sp in st.spills.iter().filter(|sp| sp.slot != super::lower::NO_SLOT) {
        match super::osr::read(it, b + sp.slot, reg.kind(sp.slot)).and_then(|v| bits(&v, sp.ty)) {
            Some(x) => unsafe { *at(sp.buf) = x },
            None => return 1,
        }
    }
    let all = unsafe { std::slice::from_raw_parts_mut(buf, plan.nbuf.max(1)) };
    for sp in &st.structs {
        if super::osr::load_struct(it, b + sp.slot, reg.kind(sp.slot), sp, all).is_none() {
            return 1;
        }
    }
    0
}

pub(crate) extern "C" fn fmod(a: f64, b: f64) -> f64 {
    a % b
}

pub(crate) extern "C" fn round(x: f64) -> f64 {
    x.round()
}

/// `v[k]` read straight from a packed body holding exactly type `want` (`Some(None)`: out of bounds,
/// which fails as the element read does); `None`: not packed with that type, the element path decides.
/// The packing holds what `bits` would make of the element.
fn packed_get(xs: &crate::interpreter::vecbody::VecBody, k: usize, want: Ty) -> Option<Option<i64>> {
    use crate::interpreter::vecbody::Packed;
    Some(match (xs.packed()?, want) {
        (Packed::Bool(v), Ty::Bool) => v.get(k).map(|b| *b as i64),
        (Packed::Int(v, t), Ty::Int(w)) if *t == w => v.get(k).copied(),
        (Packed::Float(v, t), Ty::Flt(w)) if *t == w => v.get(k).map(|f| f.to_bits() as i64),
        _ => return None,
    })
}

/// `v[k] = x` written straight into a packed body when `x` (bits at type `xt`) has exactly the packing's
/// type: then the walker's `retag` keeps `x` as it is and the plain replacement packs it. `false`: the
/// element path decides (another type, a literal width to retag, an unpacked body).
/// Write a packed element in place; `Some`: the storage written (its data pointer and length).
fn packed_set(xs: &mut Rc<crate::interpreter::vecbody::VecBody>, k: usize, x: i64, xt: Ty) -> Option<(i64, i64)> {
    use crate::interpreter::vecbody::{Packed, VecBody};
    let fits = matches!((xs.packed(), xt), (Some(Packed::Bool(_)), Ty::Bool))
        || matches!((xs.packed(), xt), (Some(Packed::Int(_, t)), Ty::Int(w)) if *t == w)
        || matches!((xs.packed(), xt), (Some(Packed::Float(_, t)), Ty::Flt(w)) if *t == w);
    if !fits || k >= xs.len() {
        return None;
    }
    let VecBody::Packed(p, cache) = Rc::make_mut(xs) else { return None };
    cache.take();
    Some(match (p, xt) {
        (Packed::Bool(v), _) => {
            v[k] = x != 0;
            (v.as_mut_ptr() as i64, v.len() as i64)
        }
        (Packed::Int(v, _), Ty::Int(t)) => {
            v[k] = int_value(x, t) as i64;
            (v.as_mut_ptr() as i64, v.len() as i64)
        }
        (Packed::Float(v, _), _) => {
            v[k] = f64::from_bits(x as u64);
            (v.as_mut_ptr() as i64, v.len() as i64)
        }
        _ => return None,
    })
}

/// The cell a local's value lives in when the local reaches it through plain references (no projection,
/// no lock guard), resolved once per unit run; `None`: the local holds its value directly, or the chain
/// has a projection (the element path walks it as before).
#[inline(always)]
unsafe fn local_cell(c: *mut Ctx, slot: i64) -> Option<usize> {
    // SAFETY: as in `with_slot`.
    let h = unsafe { (*c).cell_hit[slot as usize % CELL_HITS] };
    if h.0 == slot + 1 {
        return (h.1 != NO_CELL).then_some(h.1 as usize);
    }
    unsafe { local_cell_miss(c, slot) }
}

/// `local_cell` when the slot is not in the hit table: the scan of `cells`, or the resolution (and remembering it).
#[inline(never)]
unsafe fn local_cell_miss(c: *mut Ctx, slot: i64) -> Option<usize> {
    // SAFETY: as in `with_slot`.
    let ctx = unsafe { &mut *c };
    let found = match ctx.cells.iter().position(|(s, _)| *s == slot) {
        Some(i) => i,
        None => {
            let it = unsafe { &*ctx.it };
            if it.fast_off("vecresolve") {
                return None;
            }
            let cell = resolve_cell(it.rstack.get(ctx.base + slot as usize)?);
            ctx.cells.push((slot, cell));
            ctx.cells.len() - 1
        }
    };
    let i = ctx.cells[found].1.is_some().then_some(found);
    ctx.cell_hit[slot as usize % CELL_HITS] = (slot + 1, i.map_or(NO_CELL, |i| i as u32));
    i
}

/// The innermost cell behind a chain of plain references, or `None` (a plain value, a projection or a guard).
fn resolve_cell(v: &Value) -> Option<crate::interpreter::value::Slot> {
    let Value::Ref(p) = v else { return None };
    if !p.path.is_empty() || p.guard.is_some() {
        return None;
    }
    let mut cell = p.slot.clone();
    loop {
        let next = match &*cell.try_borrow().ok()? {
            Value::Ref(q) if q.path.is_empty() && q.guard.is_none() => q.slot.clone(),
            Value::Ref(_) => return None,
            _ => break,
        };
        cell = next;
    }
    Some(cell)
}

/// `with_slot` for a read of a local that may sit behind references: through its resolved cell when it has one.
unsafe fn with_cell(c: *mut Ctx, slot: i64, f: impl FnOnce(&Value) -> Option<i64>) -> i64 {
    match unsafe { local_cell(c, slot) } {
        Some(i) => {
            // SAFETY: as in `with_slot`; `f` sees only the value, never the context.
            let ctx = unsafe { &mut *c };
            unsafe { (*ctx.it).fast.vecresolve += 1 };
            let cell = ctx.cells[i].1.as_ref().expect("resolved");
            match cell.try_borrow().ok().and_then(|g| f(&g)) {
                Some(x) => x,
                None => {
                    ctx.fail = 1;
                    0
                }
            }
        }
        None => unsafe { with_slot(c, slot, |v| peek(v, f)) },
    }
}

/// `with_cell_mut` that never fails the unit: `f`'s answer, `false` when the local cannot be reached.
pub(super) unsafe fn with_value_mut(c: *mut Ctx, slot: i64, f: impl FnOnce(&mut Value) -> bool) -> bool {
    let mut r = false;
    // SAFETY: as in `with_slot`.
    let fail = unsafe { (*c).fail };
    unsafe {
        with_cell_mut(c, slot, |v| {
            r = f(v);
            Some(0)
        });
        (*c).fail = fail;
    }
    r
}

/// `with_slot` for a write through a local that may sit behind references (see `with_cell`).
unsafe fn with_cell_mut(c: *mut Ctx, slot: i64, f: impl FnOnce(&mut Value) -> Option<i64>) -> i64 {
    match unsafe { local_cell(c, slot) } {
        Some(i) => {
            // SAFETY: as in `with_slot`; `f` sees only the value, never the context.
            let ctx = unsafe { &mut *c };
            unsafe { (*ctx.it).fast.vecresolve += 1 };
            let cell = ctx.cells[i].1.as_ref().expect("resolved");
            match cell.try_borrow_mut().ok().and_then(|mut g| f(&mut g)) {
                Some(x) => x,
                None => {
                    ctx.fail = 1;
                    0
                }
            }
        }
        None => unsafe { with_slot(c, slot, |v| poke(v, f)) },
    }
}

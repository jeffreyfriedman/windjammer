//! Row views: machine code reads and writes the scalar fields of a `Vec` of structs in place (`v[i].f`)
//! instead of calling `index_field_get` / `index_field_set` for every access.
//!
//! After a helper reached such a vector (`row_build`), the context keeps, per view, the address of
//! every element's field array, validated against the shape the unit was compiled for: every element
//! a struct of that type (no destructor identity) whose typed fields hold exactly the shape's types,
//! and the vector and every element uniquely owned, so a store in place is the walker's
//! copy-on-write store with nothing to copy. Machine code then checks the view's key and the bound
//! and loads or stores the field's payload at a fixed offset (an `f64`, or an `i64` held as `i128`).
//!
//! A view is valid until anything could share, replace or retype an element: closure code
//! (`run_stmt`), `push`, and every helper store clear them all (`clear`). Building scans the whole
//! vector, so it is paid for by the helper calls it saves: each costs credit, each slow-path access
//! earns some (`CREDIT_*`), so a loop that keeps invalidating its views never scans more than a
//! constant factor of the helper work it already does.

use super::kinds::{same_name, ty_is, Shape, Ty};
use super::rt::Ctx;
use crate::interpreter::int::IntTy;
use crate::interpreter::value::{FloatTy, Value};
use std::rc::Rc;

/// One row view (`Ctx::rows`), read by machine code.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct RowView {
    /// `row_key(slot, shape)`; 0: empty.
    pub(crate) key: i64,
    pub(crate) len: i64,
    /// Address of the table of element field-array addresses (`Ctx::row_tbls`).
    pub(crate) tbl: i64,
}

pub(crate) const ROWS: usize = 2;
pub(crate) const CREDIT_START: i64 = 32;
pub(crate) const CREDIT_PER_MISS: i64 = 8;

/// The key of a view: the local's slot and the plan's shape index (never 0).
pub(crate) fn row_key(slot: usize, shape: usize) -> i64 {
    (((shape as i64) + 1) << 24) | (slot as i64 + 1)
}

/// Where a field's payload lives: the field array's stride, and the payload's offset inside an
/// element of it for an `f64` field (`Value::Float`) and an `i64` field (`Value::Int`, low word).
pub(crate) struct Layout {
    pub(crate) stride: i64,
    pub(crate) float: i64,
    pub(crate) int: i64,
}

pub(crate) fn layout() -> Layout {
    let f: (Rc<str>, Value) = (Rc::from(""), Value::Float(0.0, FloatTy::F64));
    let i: (Rc<str>, Value) = (Rc::from(""), Value::Int(0, IntTy::I64));
    let at = |base: &(Rc<str>, Value), p: *const u8| p as i64 - base as *const _ as i64;
    let float = match &f.1 {
        Value::Float(x, _) => at(&f, x as *const f64 as *const u8),
        _ => unreachable!("a float"),
    };
    let int = match &i.1 {
        Value::Int(x, _) => at(&i, x as *const i128 as *const u8),
        _ => unreachable!("an int"),
    };
    Layout { stride: std::mem::size_of::<(Rc<str>, Value)>() as i64, float, int }
}

/// Field types machine code accesses through a view: `f64` and `i64`.
pub(crate) fn row_field(t: Ty) -> bool {
    matches!(t, Ty::Flt(FloatTy::F64) | Ty::Int(IntTy::I64))
}

impl Ctx {
    /// Forget every row view (their tables are kept for reuse).
    pub(crate) fn clear_rows(&mut self) {
        for r in &mut self.rows {
            r.key = 0;
        }
    }
}

/// Build the view of the local at `slot` for `shape` (the plan's `idx`-th) if it has none, the
/// vector qualifies and the credit covers the scan. Never fails the unit.
pub(crate) unsafe extern "C" fn row_build(c: *mut Ctx, slot: i64, shape: i64, idx: i64) -> i64 {
    // SAFETY: as in `rt::with_slot`; `shape` is a `Shape` the unit's plan keeps alive.
    let ctx = unsafe { &mut *c };
    let at = slot as usize % ROWS;
    let key = row_key(slot as usize, idx as usize);
    ctx.row_credit += CREDIT_PER_MISS;
    if ctx.rows[at].key == key || unsafe { (*ctx.it).fast_off("rowview") } {
        return 0;
    }
    let shape = unsafe { &*(shape as *const Shape) };
    let mut tbl = std::mem::take(&mut ctx.row_tbls[at]);
    tbl.clear();
    let credit = ctx.row_credit;
    let built = unsafe { super::rt::with_value_mut(c, slot, |v| fill(v, shape, credit, &mut tbl)) };
    // SAFETY: as above (the helper call is over).
    let ctx = unsafe { &mut *c };
    if built {
        ctx.row_credit -= tbl.len() as i64;
        ctx.rows[at] = RowView { key, len: tbl.len() as i64, tbl: tbl.as_ptr() as i64 };
    }
    ctx.row_tbls[at] = tbl;
    0
}

/// The field-array address of every element of the vector `v`, or `false` when it does not qualify.
fn fill(v: &mut Value, shape: &Shape, credit: i64, tbl: &mut Vec<i64>) -> bool {
    let Value::Vec(xs) = v else { return false };
    if xs.len() as i64 > credit {
        return false;
    }
    let Some(crate::interpreter::vecbody::VecBody::Values(items)) = Rc::get_mut(xs) else { return false };
    for x in items.iter_mut() {
        let Value::Struct(s) = x else { return false };
        let Some(s) = Rc::get_mut(s) else { return false };
        if s.uid != 0 || !same_name(&s.name, &shape.name) || s.fields.len() != shape.fields.len() {
            return false;
        }
        let same = s.fields.iter().zip(&shape.fields).all(|((n, x), (m, t))| same_name(n, m) && ty_is(x, t));
        if !same {
            return false;
        }
        tbl.push(s.fields.as_mut_ptr() as i64);
    }
    true
}

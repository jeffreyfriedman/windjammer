//! `v[i].f` through a row view (`rows.rs`): machine code checks the view's key and the bound and
//! reads or writes the field's payload itself; otherwise it calls the helper as before, then lets
//! `row_build` build the view for the next access.

use super::kinds::{Shape, Ty};
use super::lower::{Lo, Mode, Val};
use super::rows::{self, row_field, row_key, RowView, ROWS};
use super::rt;
use crate::interpreter::int::IntTy;
use crate::interpreter::value::FloatTy;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, InstBuilder, MemFlags, Value as V};

/// A field machine code reaches through a row view.
pub(super) struct Row<'s> {
    pub(super) slot: usize,
    pub(super) shape: &'s Shape,
    pub(super) hint: usize,
    pub(super) ty: Ty,
    /// The index when it is an identifier bound to a loop variable: (variable id, name).
    pub(super) index: Option<(usize, &'static str)>,
}

impl Lo<'_, '_> {
    /// Can `x` (of type `xt`) be stored into `row` as its bits (the walker's `retag` keeps the field's type)?
    pub(super) fn row_store_ok(&self, row: &Row, xt: Ty) -> bool {
        self.row_ok(row)
            && match row.ty {
                Ty::Flt(FloatTy::F64) => matches!(xt, Ty::Flt(FloatTy::F64 | FloatTy::Lit)),
                Ty::Int(IntTy::I64) => matches!(xt, Ty::Int(IntTy::I64 | IntTy::Lit)),
                _ => false,
            }
    }

    /// Does machine code read `row` through a view?
    pub(super) fn row_ok(&self, row: &Row) -> bool {
        row_field(row.ty) && !self.it.fast_off("rowview")
    }

    /// The plan's index of the view shape for `slot`.
    pub(super) fn row_shape(&mut self, slot: usize, shape: &Shape) -> usize {
        match self.row_shapes.iter().position(|(s, sh)| *s == slot && **sh == *shape) {
            Some(i) => i,
            None => {
                self.row_shapes.push((slot, Box::new(shape.clone())));
                self.row_shapes.len() - 1
            }
        }
    }

    /// Branch on the view: in the fast block the payload's address is passed on; `slow` runs the
    /// helper, after which the view is built. Returns the value both paths give (`load`: the bits read).
    fn row_access(&mut self, row: &Row, i: &Val, fast: impl FnOnce(&mut Self, V) -> Option<V>, slow: impl FnOnce(&mut Self) -> Option<V>) -> Option<V> {
        match self.mode {
            Mode::Fast => return self.row_access_fast(row, i, fast),
            Mode::Analyze => self.note_row(row),
            _ => {}
        }
        let idx = self.row_shape(row.slot, row.shape);
        let shape_at = &*self.row_shapes[idx].1 as *const Shape as i64;
        let at = rt::ROW_AT + (row.slot % ROWS * std::mem::size_of::<RowView>()) as i32;
        let flags = MemFlags::trusted();
        let key = self.b.ins().load(types::I64, flags, self.ctx, at);
        let len = self.b.ins().load(types::I64, flags, self.ctx, at + 8);
        let same = self.b.ins().icmp_imm(IntCC::Equal, key, row_key(row.slot, idx));
        // unsigned: a negative index is out of bounds
        let inside = self.b.ins().icmp(IntCC::UnsignedLessThan, i.v, len);
        let hit = self.b.ins().band(same, inside);
        let (fb, sb, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
        let out = self.b.declare_var(types::I64);
        self.b.ins().brif(hit, fb, &[], sb, &[]);
        self.b.switch_to_block(fb);
        self.b.seal_block(fb);
        let lay = rows::layout();
        let off = row.hint as i64 * lay.stride + if matches!(row.ty, Ty::Flt(_)) { lay.float } else { lay.int };
        let tbl = self.b.ins().load(types::I64, flags, self.ctx, at + 16);
        let k = self.b.ins().imul_imm(i.v, 8);
        let entry = self.b.ins().iadd(tbl, k);
        let fields = self.b.ins().load(types::I64, flags, entry, 0);
        let addr = self.b.ins().iadd_imm(fields, off);
        let n = self.b.ins().load(types::I64, flags, self.ctx, rt::ROW_HITS);
        let n = self.b.ins().iadd_imm(n, 1);
        self.b.ins().store(flags, n, self.ctx, rt::ROW_HITS);
        let r = fast(self, addr)?;
        self.b.def_var(out, r);
        self.b.ins().jump(join, &[]);
        self.b.switch_to_block(sb);
        self.b.seal_block(sb);
        let was = std::mem::replace(&mut self.in_row_slow, true);
        let r = slow(self);
        let a = [row.slot as i64, shape_at, idx as i64].map(|c| self.b.ins().iconst(types::I64, c));
        self.call("row_build", rows::row_build as *const () as usize, &a, false, false);
        self.in_row_slow = was;
        let r = r?;
        self.b.def_var(out, r);
        self.b.ins().jump(join, &[]);
        self.b.switch_to_block(join);
        self.b.seal_block(join);
        Some(self.b.use_var(out))
    }

    /// Read `row`'s field of element `i` (its bits); `slow` calls the helper.
    pub(super) fn row_load(&mut self, row: &Row, i: &Val, slow: impl FnOnce(&mut Self) -> Option<V>) -> Option<V> {
        // an `i64` field's low word is its value (it fits: the view checked the type)
        self.row_access(row, i, |lo, addr| Some(lo.b.ins().load(types::I64, MemFlags::trusted(), addr, 0)), slow)
    }

    /// Store `bits` into `row`'s field of element `i`; `slow` calls the helper.
    pub(super) fn row_store(&mut self, row: &Row, i: &Val, bits: V, slow: impl FnOnce(&mut Self)) -> Option<()> {
        let int = matches!(row.ty, Ty::Int(_));
        let fast = |lo: &mut Self, addr: V| {
            let flags = MemFlags::trusted();
            lo.b.ins().store(flags, bits, addr, 0);
            if int {
                // the `i128` payload's high word: the sign
                let hi = lo.b.ins().sshr_imm(bits, 63);
                lo.b.ins().store(flags, hi, addr, 8);
            }
            Some(bits)
        };
        self.row_access(
            row,
            i,
            fast,
            |lo| {
                slow(lo);
                Some(bits)
            },
        )?;
        Some(())
    }
}

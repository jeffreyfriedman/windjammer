//! Hoisted row views by loop versioning (SOUNDNESS-hoist.md). Pass A (`Mode::Analyze`) records per statement loop
//! whether it can run without any helper that clears views or mutates vec/struct storage, which rows it accesses
//! and which index variables it never assigns. Pass B (`Mode::Version`) lowers such a loop twice behind one
//! preheader that builds and checks every row view: a fast copy (`Mode::Fast`) whose row accesses need no key check,
//! no slow path and, for an invariant index, no table walk; and the ordinary code as the slow copy.

use super::kinds::{Shape, Ty};
use super::lower::{Bind, HIdx, HRow, Lo, Mode, S};
use super::lower_rows::Row;
use super::rows::{self, row_key, RowView, ROWS};
use super::rt;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, InstBuilder, MemFlags, Value as V};

impl Lo<'_, '_> {
    /// Pass A: a variable (walker slot `slot`) is defined here: every active loop assigns it.
    pub(super) fn note_def(&mut self, slot: usize) {
        if self.mode == Mode::Analyze && slot != super::lower::NO_SLOT {
            for k in &self.active {
                self.info.entry(*k).or_default().defs.insert(slot);
            }
        }
    }

    /// A helper is called: pass A disqualifies the active loops unless it is a read-only one; in a fast copy
    /// anything else abandons pass B.
    pub(super) fn note_call(&mut self, name: &str) {
        if super::lower::SAFE_CALLS.contains(&name) || self.in_row_slow {
            return;
        }
        match self.mode {
            Mode::Analyze => {
                for k in &self.active {
                    self.info.entry(*k).or_default().unsafe_ = true;
                }
            }
            Mode::Fast => self.broken = true,
            _ => {}
        }
    }

    /// Pass A: a row access of the active loops.
    pub(super) fn note_row(&mut self, row: &Row) {
        for k in self.active.clone() {
            let l = self.info.entry(k).or_default();
            if !l.rows.iter().any(|(s, sh)| *s == row.slot && sh == row.shape) {
                l.rows.push((row.slot, row.shape.clone()));
            }
            if let Some((id, name)) = row.index {
                let islot = self.vars[id].slot;
                if islot == super::lower::NO_SLOT {
                    continue; // hidden, match-arm and inlined-parameter variables have no walker slot to compare
                }
                if !l.cands.iter().any(|(s, sh, n, is)| *s == row.slot && sh == row.shape && *n == name && *is == islot) {
                    l.cands.push((row.slot, row.shape.clone(), name, islot));
                }
            }
        }
    }

    /// Lower a statement loop with `once` (its ordinary lowering): once, recording what it does (pass A), or, when pass A
    /// found it view-stable, twice behind a preheader that checks the row views.
    pub(super) fn loop_versioned(&mut self, body: &'static [S], mut once: impl FnMut(&mut Self) -> Option<()>) -> Option<()> {
        let key = body.as_ptr() as usize;
        match self.mode {
            Mode::Analyze => {
                self.active.push(key);
                self.info.entry(key).or_default();
                let r = once(self);
                self.active.pop();
                return r;
            }
            Mode::Fast => {
                let mark = self.hidx.len();
                self.hoist_idx(key);
                let r = once(self);
                self.hidx.truncate(mark);
                return r;
            }
            Mode::Version if self.info.get(&key).is_some_and(|l| !l.unsafe_ && !l.rows.is_empty()) => {}
            _ => return once(self),
        }
        let rows: Vec<(usize, Shape)> = self.info[&key].rows.clone();
        let flags = MemFlags::trusted();
        let plan: Vec<(usize, usize, i64, i32)> = rows
            .iter()
            .map(|(slot, shape)| {
                let idx = self.row_shape(*slot, shape);
                let shape_at = &*self.row_shapes[idx].1 as *const Shape as i64;
                (*slot, idx, shape_at, rt::ROW_AT + (*slot % ROWS * std::mem::size_of::<RowView>()) as i32)
            })
            .collect();
        // build every missing view, then check them all (a later build may evict an earlier view sharing its table entry)
        for (slot, idx, shape_at, at) in &plan {
            let kv = self.b.ins().load(types::I64, flags, self.ctx, *at);
            let same = self.b.ins().icmp_imm(IntCC::Equal, kv, row_key(*slot, *idx));
            let (build, have) = (self.b.create_block(), self.b.create_block());
            self.b.ins().brif(same, have, &[], build, &[]);
            self.b.switch_to_block(build);
            self.b.seal_block(build);
            let a = [*slot as i64, *shape_at, *idx as i64].map(|c| self.b.ins().iconst(types::I64, c));
            self.call("row_build", rows::row_build as *const () as usize, &a, false, false);
            self.b.ins().jump(have, &[]);
            self.b.switch_to_block(have);
            self.b.seal_block(have);
        }
        let mut ok: Option<V> = None;
        for (slot, idx, _, at) in &plan {
            let kv = self.b.ins().load(types::I64, flags, self.ctx, *at);
            let same = self.b.ins().icmp_imm(IntCC::Equal, kv, row_key(*slot, *idx));
            ok = Some(match ok {
                Some(o) => self.b.ins().band(o, same),
                None => same,
            });
        }
        let ok = ok?;
        let (fast, slow, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
        self.b.ins().brif(ok, fast, &[], slow, &[]);
        // the fast copy
        self.b.switch_to_block(fast);
        self.b.seal_block(fast);
        self.count(rt::HOISTED, 1);
        self.count(rt::ROW_HITS, plan.len() as i64);
        let natives = self.natives;
        let saved = (self.hrows.len(), self.hidx.len());
        for (slot, idx, _, at) in &plan {
            let len = self.b.ins().load(types::I64, flags, self.ctx, *at + 8);
            let tbl = self.b.ins().load(types::I64, flags, self.ctx, *at + 16);
            self.hrows.push(HRow { slot: *slot, idx: *idx, len, tbl });
        }
        self.mode = Mode::Fast;
        self.hoist_idx(key);
        let r = once(self);
        self.mode = Mode::Version;
        self.hrows.truncate(saved.0);
        self.hidx.truncate(saved.1);
        self.natives = natives;
        r?;
        self.b.ins().jump(join, &[]);
        // the slow copy: the ordinary code
        self.b.switch_to_block(slow);
        self.b.seal_block(slow);
        self.count(rt::HOISTFAIL, 1);
        self.mode = Mode::Plain;
        let r = once(self);
        self.mode = Mode::Version;
        r?;
        self.b.ins().jump(join, &[]);
        self.b.switch_to_block(join);
        self.b.seal_block(join);
        Some(())
    }

    /// `ctx.field += n` (a counter).
    fn count(&mut self, at: i32, n: i64) {
        let flags = MemFlags::trusted();
        let x = self.b.ins().load(types::I64, flags, self.ctx, at);
        let x = self.b.ins().iadd_imm(x, n);
        self.b.ins().store(flags, x, self.ctx, at);
    }

    /// At the preheader of loop `key` inside a fast copy: for each index variable its rows use and the loop never
    /// defines, the field-array address of the element it names (when in range).
    fn hoist_idx(&mut self, key: usize) {
        let (cands, defs) = match self.info.get(&key) {
            Some(l) => (l.cands.clone(), l.defs.clone()),
            None => return,
        };
        let flags = MemFlags::trusted();
        for (slot, shape, name, islot) in cands {
            if islot == super::lower::NO_SLOT || defs.contains(&islot) {
                continue;
            }
            let idx = self.row_shape(slot, &shape);
            let Some((len, tbl)) = self.hrows.iter().find(|h| h.slot == slot && h.idx == idx).map(|h| (h.len, h.tbl)) else { continue };
            let Some(Bind::Var(id)) = self.lookup(name).cloned() else { continue };
            if self.vars[id].slot != islot || !matches!(self.vars[id].ty, Ty::Int(_)) {
                continue;
            }
            if self.hidx.iter().any(|h| h.slot == slot && h.idx == idx && h.var == id) {
                continue;
            }
            let v = self.b.use_var(self.vars[id].var);
            let bad = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, v, len);
            let out = self.b.declare_var(types::I64);
            let (yes, no, join) = (self.b.create_block(), self.b.create_block(), self.b.create_block());
            self.b.ins().brif(bad, no, &[], yes, &[]);
            self.b.switch_to_block(yes);
            self.b.seal_block(yes);
            let k = self.b.ins().imul_imm(v, 8);
            let entry = self.b.ins().iadd(tbl, k);
            let fields = self.b.ins().load(types::I64, flags, entry, 0);
            self.b.def_var(out, fields);
            self.b.ins().jump(join, &[]);
            self.b.switch_to_block(no);
            self.b.seal_block(no);
            let zero = self.b.ins().iconst(types::I64, 0);
            self.b.def_var(out, zero);
            self.b.ins().jump(join, &[]);
            self.b.switch_to_block(join);
            self.b.seal_block(join);
            let fields = self.b.use_var(out);
            self.hidx.push(HIdx { slot, idx, var: id, bad, fields });
        }
    }

    /// A row access in a fast copy: the preheader validated the view; the access checks the bound and loads the payload's address.
    pub(super) fn row_access_fast(&mut self, row: &Row, i: &super::lower::Val, fast: impl FnOnce(&mut Self, V) -> Option<V>) -> Option<V> {
        let idx = self.row_shape(row.slot, row.shape);
        let Some((len, tbl)) = self.hrows.iter().find(|h| h.slot == row.slot && h.idx == idx).map(|h| (h.len, h.tbl)) else {
            self.broken = true;
            return None;
        };
        let flags = MemFlags::trusted();
        let hoisted = row.index.map(|(v, _)| v).and_then(|v| self.hidx.iter().rev().find(|h| h.slot == row.slot && h.idx == idx && h.var == v)).map(|h| (h.bad, h.fields));
        let fields = match hoisted {
            Some((bad, fields)) => {
                self.bail_if(bad);
                fields
            }
            None => {
                // unsigned: a negative index is out of bounds
                let oob = self.b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, i.v, len);
                self.bail_if(oob);
                let k = self.b.ins().imul_imm(i.v, 8);
                let entry = self.b.ins().iadd(tbl, k);
                self.b.ins().load(types::I64, flags, entry, 0)
            }
        };
        let lay = rows::layout();
        let off = row.hint as i64 * lay.stride + if matches!(row.ty, Ty::Flt(_)) { lay.float } else { lay.int };
        let addr = self.b.ins().iadd_imm(fields, off);
        fast(self, addr)
    }
}

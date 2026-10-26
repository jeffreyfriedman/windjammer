//! Go lowering of the enumerable surface (ADR-0018 feature 1) to `wj_core.go` helpers.
//! Which methods exist is decided by `crate::stdlib_surface::enumerable`; this file only spells the calls.

use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;
use crate::stdlib_surface::enumerable::lookup;

impl<'m> Go<'m> {
    /// `Some(go)` when `recv.name(args)` is an enumerable surface method (or a keyed std method it relies on).
    pub(super) fn enumerable_method(
        &mut self,
        recv: &PExpr,
        name: &str,
        a: &[String],
    ) -> PResult<Option<String>> {
        if !matches!(recv.ty, Ty::Vec(_) | Ty::Iter(_) | Ty::Range(_)) {
            return Ok(None);
        }
        let keyed = matches!(
            (name, a.len()),
            ("sort_by_key" | "min_by_key" | "max_by_key" | "partition", 1)
        );
        if !keyed && lookup(name, a.len()).is_none() {
            return Ok(None);
        }
        let r = self.expr(recv)?;
        Ok(Some(match name {
            "sort_by_key" => format!("wjSortByKeyInPlace({r}, {}, false)", a[0]),
            "sort_by_key_desc" => format!("wjSortByKeyInPlace({r}, {}, true)", a[0]),
            "min_by_key" => format!("wjMinByKey({r}, {})", a[0]),
            "max_by_key" => format!("wjMaxByKey({r}, {})", a[0]),
            "partition" | "partition_by" => format!("wjPartitionBy({r}, {})", a[0]),
            "group_by" => format!("wjGroupBy({r}, {})", a[0]),
            "count_by" => format!("wjCountBy({r}, {})", a[0]),
            "tally" | "frequencies" => format!("wjTally({r})"),
            "sum_by" => format!("wjSumBy({r}, {})", a[0]),
            "unique" => format!("wjUnique({r})"),
            "unique_by" => format!("wjUniqueBy({r}, {})", a[0]),
            "chunk_by" => format!("wjChunkBy({r}, {})", a[0]),
            "each_slice" => format!("wjChunks({r}, {})", a[0]),
            "intersperse" => format!("wjIntersperse({r}, {})", a[0]),
            "zip_with" => format!("wjZipWith({r}, {}, {})", a[0], a[1]),
            "index_of" => format!("wjIndexOf({r}, {})", a[0]),
            "sorted" => format!("wjSorted({r})"),
            "sorted_by_key" => format!("wjSortedByKey({r}, {}, false)", a[0]),
            "sorted_by_key_desc" => format!("wjSortedByKey({r}, {}, true)", a[0]),
            "to_list" => format!("wjClone({r})"),
            other => return self.unsupported(format!("enumerable method `{other}`")),
        }))
    }
}

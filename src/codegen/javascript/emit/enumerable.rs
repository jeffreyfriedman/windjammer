//! JavaScript/TypeScript lowering of the enumerable surface (ADR-0018 feature 1) to `wj_core.mjs` helpers.
//! Which methods exist is decided by `crate::stdlib_surface::enumerable`; this file only spells the calls.

use super::Js;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;
use crate::stdlib_surface::enumerable::lookup;

impl<'m> Js<'m> {
    /// `Some(js)` when `recv.name(args)` is an enumerable surface method (or a keyed std method it relies on).
    pub(super) fn enumerable_method(
        &mut self,
        recv: &PExpr,
        name: &str,
        r: &str,
        a: &[String],
        args: &[PExpr],
    ) -> PResult<Option<String>> {
        let seq = match &recv.ty {
            Ty::Vec(_) | Ty::Iter(_) | Ty::Range(_) => r.to_string(),
            Ty::Set(_) | Ty::Map(..) => format!("Array.from({r})"),
            _ => return Ok(None),
        };
        let keyed = match (name, a.len()) {
            ("sort_by_key", 1) => Some(format!("$sortByKeyInPlace({r}, {}, false)", a[0])),
            ("min_by_key", 1) => Some(format!("$minByKey({seq}, {})", a[0])),
            ("max_by_key", 1) => Some(format!("$maxByKey({seq}, {})", a[0])),
            ("partition", 1) => Some(format!("$partitionBy({seq}, {})", a[0])),
            _ => None,
        };
        if keyed.is_some() {
            return Ok(keyed);
        }
        if lookup(name, a.len()).is_none() {
            return Ok(None);
        }
        Ok(Some(match name {
            "group_by" => format!("$groupBy({seq}, {})", a[0]),
            "count_by" => format!("$countBy({seq}, {})", a[0]),
            "tally" | "frequencies" => format!("$countBy({seq}, (x) => x)"),
            "sum_by" => {
                let ret = match args.first().map(|e| &e.ty) {
                    Some(Ty::Fn(_, r)) => (**r).clone(),
                    _ => return self.unsupported("sum_by with an untyped closure"),
                };
                let (f, z) = self.sum_op(&ret, "sum")?;
                format!("{seq}.map((x) => ({})(x)).reduce({f}, {z})", a[0])
            }
            "unique" => format!("$uniqueBy({seq}, (x) => x)"),
            "unique_by" => format!("$uniqueBy({seq}, {})", a[0]),
            "chunk_by" => format!("$chunkBy({seq}, {})", a[0]),
            "each_slice" => format!("$chunks({seq}, {})", a[0]),
            "partition_by" => format!("$partitionBy({seq}, {})", a[0]),
            "intersperse" => format!("$intersperse({seq}, {})", a[0]),
            "zip_with" => format!("$zipWith({seq}, Array.from({}), {})", a[0], a[1]),
            "index_of" => format!("$indexOf({seq}, {})", a[0]),
            "sorted" => format!("$sortedByKey({seq}, (x) => x, false)"),
            "sorted_by_key" => format!("$sortedByKey({seq}, {}, false)", a[0]),
            "sorted_by_key_desc" => format!("$sortedByKey({seq}, {}, true)", a[0]),
            "sort_by_key_desc" => format!("$sortByKeyInPlace({r}, {}, true)", a[0]),
            "to_list" => format!("Array.from({seq})"),
            other => return self.unsupported(format!("enumerable method `{other}`")),
        }))
    }
}

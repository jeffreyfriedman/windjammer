//! `std::arena` on the Go/JS/TS backends (ADR-0025).
//!
//! The arena is the Windjammer-source prelude `portable/prelude/arena.wj`, lowered like user code. The one thing
//! user code cannot express is a returned `&mut T`: `arena.get_mut(h)` must alias the slot. It lowers to the
//! place `arena.items[arena.checked(h)]` (the check panics on a stale or foreign handle exactly like the Rust
//! runtime), and `let p = arena.get_mut(h)` binds `p` to that place with `PK::BorrowMut`.

use super::{Lowerer, PResult};
use crate::codegen::portable::env::ItemEnv;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{IntKind, Ty};
use crate::parser::Expression;

type Args<'a> = [(Option<String>, &'a Expression<'a>)];

/// True when `owner` is the prelude's `Arena` (a user type of the same name has other fields).
pub(super) fn is_prelude_arena(env: &ItemEnv, owner: &str) -> bool {
    owner == "Arena"
        && env.structs.get(owner).is_some_and(|s| {
            ["items", "generation", "id"].iter().all(|f| s.fields.iter().any(|(n, _)| n == f))
        })
}

/// True when `owner` is the prelude's `Scratch`.
pub(super) fn is_prelude_scratch(env: &ItemEnv, owner: &str) -> bool {
    owner == "Scratch" && env.structs.get(owner).is_some_and(|s| s.fields.len() == 1 && s.fields[0].0 == "generation")
}

/// The slot place built by [`Lowerer::arena_slot`]: `Index(Field(_, "items"), Method "checked")`.
pub(super) fn is_slot_place(e: &PExpr) -> bool {
    matches!(&*e.kind, PK::Index(items, idx)
        if matches!(&*items.kind, PK::Field(_, f) if f == "items")
        && matches!(&*idx.kind, PK::Method { name, .. } if name == "checked"))
}

impl Lowerer<'_> {
    pub(super) fn arena_slot(&mut self, recv: PExpr, targs: &[Ty], args: &Args) -> PResult<PExpr> {
        let elem = targs.first().cloned().unwrap_or(Ty::Unknown);
        let handle_ty = Ty::Adt("Handle".into(), vec![elem.clone()]);
        let h = self.expr(args[0].1, Some(&handle_ty))?;
        let h = self.own(h);
        let line = recv.line;
        let idx = PExpr::new(
            Ty::Int(IntKind::I64),
            PK::Method { recv: recv.clone(), name: "checked".into(), args: vec![h], res: Resolution::User { owner: "Arena".into() } },
        )
        .at(line);
        let items = PExpr::new(Ty::Vec(Box::new(elem.clone())), PK::Field(recv, "items".into())).at(line);
        Ok(PExpr::new(elem, PK::Index(items, idx)).at(line))
    }
}

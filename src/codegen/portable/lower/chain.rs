//! Optional chaining `a?.b` (ADR-0018 feature 5) for the portable backends (Go, JS, TS).
//!
//! The parser desugars `a?.b` to `a.__opt_chain(|_oc| _oc.b)`. With the receiver's static type known here this
//! becomes `match a { Some(_oc) => <_oc.b lifted to an option>, None => None }`: the member result is wrapped in
//! `Some` unless it already is an option (then it is returned as is, so `a?.b?.c` stays one optional).

use super::{Lowerer, PResult};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;
use crate::parser::optional_chain::OPT_CHAIN_PARAM;
use crate::parser::Expression;

impl<'e> Lowerer<'e> {
    pub(super) fn optional_chain(&mut self, recv: PExpr, args: &[(Option<String>, &Expression)]) -> PResult<PExpr> {
        let Some((_, Expression::Closure { body, .. })) = args.first() else {
            return self.err("malformed `?.`");
        };
        let Ty::Option(inner) = self.resolve(&recv.ty) else {
            return self.err(format!("`?.` needs an optional (`T?`) receiver, found `{}`", recv.ty.describe()));
        };
        self.push();
        let bound = self.declare(OPT_CHAIN_PARAM, *inner, false);
        let member = self.expr(body, None);
        self.pop();
        let member = member?;
        let member_ty = self.resolve(&member.ty);
        let (ty, some_value) = match member_ty {
            Ty::Option(_) => (member_ty, member),
            other => {
                let opt = Ty::Option(Box::new(other));
                (opt.clone(), PExpr::new(opt, PK::Some(member)))
            }
        };
        let arms = vec![
            PArm { pat: PPat::Some(Box::new(PPat::Bind { name: bound, mutable: false })), guard: None, body: PBlock { stmts: vec![], tail: Some(some_value) } },
            PArm { pat: PPat::None, guard: None, body: PBlock { stmts: vec![], tail: Some(PExpr::new(ty.clone(), PK::None)) } },
        ];
        Ok(PExpr::new(ty, PK::Match { scrutinee: recv, arms }))
    }
}

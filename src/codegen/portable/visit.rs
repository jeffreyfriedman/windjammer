//! Mutable traversal helpers over the portable IR.

use super::ir::*;
use super::ty::Ty;

type TyFn<'a> = &'a mut dyn FnMut(&mut Ty);

pub fn types_in_fn(f: &mut PFn, v: TyFn) {
    for p in f.params.iter_mut() {
        v(&mut p.ty);
    }
    v(&mut f.ret);
    if let Some(b) = &mut f.body {
        types_in_block(b, v);
    }
}

pub fn types_in_block(b: &mut PBlock, v: TyFn) {
    b.stmts.iter_mut().for_each(|s| types_in_stmt(s, v));
    if let Some(t) = &mut b.tail {
        types_in_expr(t, v);
    }
}

pub fn types_in_stmts(ss: &mut [PStmt], v: TyFn) {
    ss.iter_mut().for_each(|s| types_in_stmt(s, v));
}

fn types_in_pat(p: &mut PPat, v: TyFn) {
    match p {
        PPat::Lit(e) => types_in_expr(e, v),
        PPat::Range { lo, hi, .. } => {
            if let Some(e) = lo {
                types_in_expr(e, v);
            }
            if let Some(e) = hi {
                types_in_expr(e, v);
            }
        }
        PPat::Tuple(ps) | PPat::Or(ps) => ps.iter_mut().for_each(|p| types_in_pat(p, v)),
        PPat::Variant { fields, .. } => fields.iter_mut().for_each(|(_, p)| types_in_pat(p, v)),
        PPat::Struct { fields, .. } => fields.iter_mut().for_each(|(_, t, p)| {
            v(t);
            types_in_pat(p, v)
        }),
        PPat::Some(p) | PPat::Ok(p) | PPat::Err(p) => types_in_pat(p, v),
        PPat::Wild | PPat::Bind { .. } | PPat::None => {}
    }
}

pub fn types_in_stmt(s: &mut PStmt, v: TyFn) {
    match s {
        PStmt::Let { ty, init, .. } => {
            v(ty);
            if let Some(e) = init {
                types_in_expr(e, v);
            }
        }
        PStmt::LetPat { pat, init, els } => {
            types_in_pat(pat, v);
            types_in_expr(init, v);
            if let Some(b) = els {
                types_in_stmts(b, v);
            }
        }
        PStmt::Assign { target, value, .. } => {
            types_in_expr(target, v);
            types_in_expr(value, v);
        }
        PStmt::Expr(e) => types_in_expr(e, v),
        PStmt::If { cond, then, els } => {
            types_in_expr(cond, v);
            types_in_stmts(then, v);
            types_in_stmts(els, v);
        }
        PStmt::While { cond, body } => {
            types_in_expr(cond, v);
            types_in_stmts(body, v);
        }
        PStmt::Loop { body } | PStmt::Block(body) => types_in_stmts(body, v),
        PStmt::ForRange { start, end, body, .. } => {
            types_in_expr(start, v);
            types_in_expr(end, v);
            types_in_stmts(body, v);
        }
        PStmt::ForIn { pat, iter, body } => {
            types_in_pat(pat, v);
            types_in_expr(iter, v);
            types_in_stmts(body, v);
        }
        PStmt::Match { scrutinee, arms } => {
            types_in_expr(scrutinee, v);
            arms.iter_mut().for_each(|a| types_in_arm(a, v));
        }
        PStmt::Return(e) => {
            if let Some(e) = e {
                types_in_expr(e, v);
            }
        }
        PStmt::Defer(s) => types_in_stmt(s, v),
        PStmt::Labeled { stmt, .. } => types_in_stmt(stmt, v),
        PStmt::Break | PStmt::Continue | PStmt::BreakTo(_) | PStmt::ContinueTo(_) => {}
    }
}

fn types_in_arm(a: &mut PArm, v: TyFn) {
    types_in_pat(&mut a.pat, v);
    if let Some(g) = &mut a.guard {
        types_in_expr(g, v);
    }
    types_in_block(&mut a.body, v);
}

pub fn children_mut(k: &mut PK) -> Vec<&mut PExpr> {
    match k {
        PK::Unary(_, a) | PK::Cast(a, _) | PK::Field(a, _) | PK::TupleIdx(a, _) | PK::Some(a) | PK::Ok(a) | PK::Err(a) | PK::Copy(a) | PK::BorrowMut(a) | PK::Panic(a) => vec![a],
        PK::Try { expr, .. } => vec![expr],
        PK::Binary(_, a, b) | PK::Index(a, b) | PK::VecRepeat(a, b) => vec![a, b],
        PK::Range { start, end, .. } => vec![start, end],
        PK::Call { args, .. } | PK::StaticCall { args, .. } | PK::StdCall { args, .. } => args.iter_mut().collect(),
        PK::CallValue { f, args } => std::iter::once(f).chain(args.iter_mut()).collect(),
        PK::Method { recv, args, .. } => std::iter::once(recv).chain(args.iter_mut()).collect(),
        PK::Struct { fields, .. } | PK::Variant { fields, .. } => fields.iter_mut().map(|(_, e)| e).collect(),
        PK::VecLit(es) | PK::Tuple(es) => es.iter_mut().collect(),
        PK::MapLit(ps) => ps.iter_mut().flat_map(|(k, v)| [k, v]).collect(),
        PK::Format(ps) | PK::Print { pieces: ps, .. } => ps.iter_mut().filter_map(|p| if let FmtPiece::Arg(e, _) = p { Some(e) } else { None }).collect(),
        _ => vec![],
    }
}

pub fn types_in_expr(e: &mut PExpr, v: TyFn) {
    v(&mut e.ty);
    match &mut *e.kind {
        PK::Cast(_, t) => v(t),
        PK::Closure { params, ret, body } => {
            params.iter_mut().for_each(|(_, t)| v(t));
            v(ret);
            types_in_block(body, v);
        }
        PK::If { cond, then, els } => {
            types_in_expr(cond, v);
            types_in_block(then, v);
            types_in_block(els, v);
        }
        PK::Match { scrutinee, arms } => {
            types_in_expr(scrutinee, v);
            arms.iter_mut().for_each(|a| types_in_arm(a, v));
        }
        PK::Block(b) => types_in_block(b, v),
        PK::Try { fn_ret, .. } => v(fn_ret),
        _ => {}
    }
    for c in children_mut(&mut e.kind) {
        types_in_expr(c, v);
    }
}

/// Visit every expression (pre-order) in a block, including nested statements, arms and closures.
pub fn exprs_in_block(b: &mut PBlock, f: &mut dyn FnMut(&mut PExpr)) {
    for s in b.stmts.iter_mut() {
        exprs_in_stmt(s, f);
    }
    if let Some(t) = &mut b.tail {
        exprs_in_expr(t, f);
    }
}

pub fn exprs_in_stmts(ss: &mut [PStmt], f: &mut dyn FnMut(&mut PExpr)) {
    ss.iter_mut().for_each(|s| exprs_in_stmt(s, f));
}

fn exprs_in_arm(a: &mut PArm, f: &mut dyn FnMut(&mut PExpr)) {
    if let Some(g) = &mut a.guard {
        exprs_in_expr(g, f);
    }
    exprs_in_block(&mut a.body, f);
}

pub fn exprs_in_stmt(s: &mut PStmt, f: &mut dyn FnMut(&mut PExpr)) {
    match s {
        PStmt::Let { init, .. } => {
            if let Some(e) = init {
                exprs_in_expr(e, f);
            }
        }
        PStmt::LetPat { init, els, .. } => {
            exprs_in_expr(init, f);
            if let Some(b) = els {
                exprs_in_stmts(b, f);
            }
        }
        PStmt::Assign { target, value, .. } => {
            exprs_in_expr(target, f);
            exprs_in_expr(value, f);
        }
        PStmt::Expr(e) => exprs_in_expr(e, f),
        PStmt::If { cond, then, els } => {
            exprs_in_expr(cond, f);
            exprs_in_stmts(then, f);
            exprs_in_stmts(els, f);
        }
        PStmt::While { cond, body } => {
            exprs_in_expr(cond, f);
            exprs_in_stmts(body, f);
        }
        PStmt::Loop { body } | PStmt::Block(body) => exprs_in_stmts(body, f),
        PStmt::ForRange { start, end, body, .. } => {
            exprs_in_expr(start, f);
            exprs_in_expr(end, f);
            exprs_in_stmts(body, f);
        }
        PStmt::ForIn { iter, body, .. } => {
            exprs_in_expr(iter, f);
            exprs_in_stmts(body, f);
        }
        PStmt::Match { scrutinee, arms } => {
            exprs_in_expr(scrutinee, f);
            arms.iter_mut().for_each(|a| exprs_in_arm(a, f));
        }
        PStmt::Return(e) => {
            if let Some(e) = e {
                exprs_in_expr(e, f);
            }
        }
        PStmt::Defer(s) => exprs_in_stmt(s, f),
        PStmt::Labeled { stmt, .. } => exprs_in_stmt(stmt, f),
        PStmt::Break | PStmt::Continue | PStmt::BreakTo(_) | PStmt::ContinueTo(_) => {}
    }
}

pub fn exprs_in_expr(e: &mut PExpr, f: &mut dyn FnMut(&mut PExpr)) {
    f(e);
    match &mut *e.kind {
        PK::Closure { body, .. } => exprs_in_block(body, f),
        PK::If { cond, then, els } => {
            exprs_in_expr(cond, f);
            exprs_in_block(then, f);
            exprs_in_block(els, f);
        }
        PK::Match { scrutinee, arms } => {
            exprs_in_expr(scrutinee, f);
            arms.iter_mut().for_each(|a| exprs_in_arm(a, f));
        }
        PK::Block(b) => exprs_in_block(b, f),
        _ => {}
    }
    for c in children_mut(&mut e.kind) {
        exprs_in_expr(c, f);
    }
}

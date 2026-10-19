//! Flatten expression-position control flow into statements.
//!
//! After this pass no `PK::If`, `PK::Match`, `PK::Block` or `PK::Try` remains inside an
//! expression and no `PBlock` carries a tail: emitters only deal with pure expression trees plus
//! statements. Evaluation order is preserved by spilling already-evaluated operands to temporaries
//! whenever a later operand needs hoisted statements.

use super::ir::*;
use super::ty::Ty;
use super::visit::children_mut;

pub fn flatten_module(m: &mut PModule) {
    let mut f = Flatten { n: 0 };
    for func in m.fns.iter_mut() {
        f.func(func);
    }
    for s in m.structs.iter_mut() {
        s.methods.iter_mut().for_each(|func| f.func(func));
    }
    for e in m.enums.iter_mut() {
        e.methods.iter_mut().for_each(|func| f.func(func));
    }
    for t in m.traits.iter_mut() {
        t.methods.iter_mut().for_each(|func| f.func(func));
    }
    for c in m.consts.iter_mut() {
        let mut pre = Vec::new();
        let v = std::mem::replace(&mut c.value, PExpr::unit());
        c.value = f.expr(v, &mut pre);
        if !pre.is_empty() {
            let ty = c.value.ty.clone();
            let tail = std::mem::replace(&mut c.value, PExpr::unit());
            c.value = PExpr::new(ty, PK::Block(PBlock { stmts: pre, tail: Some(tail) }));
        }
    }
}

struct Flatten {
    n: usize,
}

#[derive(Clone)]
enum Sink {
    Return,
    Discard,
    Assign(String),
}

fn local(name: &str, ty: &Ty) -> PExpr {
    PExpr::new(ty.clone(), PK::Local(name.to_string()))
}

fn is_place_or_trivial(e: &PExpr) -> bool {
    match &*e.kind {
        PK::Int(_) | PK::Float(_) | PK::Bool(_) | PK::Str(_) | PK::Char(_) | PK::Unit | PK::Local(_) | PK::FnRef(_) | PK::None => true,
        PK::Field(a, _) | PK::TupleIdx(a, _) => is_place_or_trivial(a),
        PK::Index(a, b) => is_place_or_trivial(a) && is_place_or_trivial(b),
        PK::Copy(a) => is_place_or_trivial(a),
        _ => false,
    }
}

impl Flatten {
    fn fresh(&mut self, base: &str) -> String {
        self.n += 1;
        format!("{base}__{}", self.n)
    }

    fn func(&mut self, f: &mut PFn) {
        if let Some(body) = f.body.take() {
            let sink = if f.ret == Ty::Unit { Sink::Discard } else { Sink::Return };
            f.body = Some(PBlock { stmts: self.block(body, sink), tail: None });
        }
    }

    fn block(&mut self, b: PBlock, sink: Sink) -> Vec<PStmt> {
        let mut out = Vec::new();
        for s in b.stmts {
            out.extend(self.stmt(s));
        }
        if let Some(t) = b.tail {
            let mut pre = Vec::new();
            let e = self.expr(t, &mut pre);
            out.extend(pre);
            out.extend(self.finish(e, sink));
        }
        out
    }

    fn finish(&mut self, e: PExpr, sink: Sink) -> Vec<PStmt> {
        let valued = e.ty != Ty::Unit && !matches!(&*e.kind, PK::Panic(_));
        match sink {
            Sink::Return if valued => vec![PStmt::Return(Some(e))],
            Sink::Assign(t) if valued => vec![PStmt::Assign { target: local(&t, &e.ty), op: None, value: e }],
            _ => {
                if matches!(&*e.kind, PK::Unit) {
                    vec![]
                } else {
                    vec![PStmt::Expr(e)]
                }
            }
        }
    }

    fn stmts(&mut self, ss: Vec<PStmt>) -> Vec<PStmt> {
        ss.into_iter().flat_map(|s| self.stmt(s)).collect()
    }

    fn stmt(&mut self, s: PStmt) -> Vec<PStmt> {
        let mut pre = Vec::new();
        let main = match s {
            PStmt::Let { name, ty, mutable, init } => PStmt::Let { name, ty, mutable, init: init.map(|e| self.expr(e, &mut pre)) },
            PStmt::LetPat { pat, init, els } => PStmt::LetPat { pat, init: self.expr(init, &mut pre), els: els.map(|b| self.stmts(b)) },
            PStmt::Assign { target, op, value } => {
                let value = self.expr(value, &mut pre);
                PStmt::Assign { target: self.expr(target, &mut pre), op, value }
            }
            PStmt::Expr(e) => {
                let e = self.expr(e, &mut pre);
                if matches!(&*e.kind, PK::Unit) {
                    return pre;
                }
                PStmt::Expr(e)
            }
            PStmt::If { cond, then, els } => PStmt::If { cond: self.expr(cond, &mut pre), then: self.stmts(then), els: self.stmts(els) },
            PStmt::While { cond, body } => return self.while_loop(cond, body),
            PStmt::Loop { body } => PStmt::Loop { body: self.stmts(body) },
            PStmt::Block(b) => PStmt::Block(self.stmts(b)),
            PStmt::ForRange { var, start, end, inclusive, body } => {
                let (start, end) = (self.expr(start, &mut pre), self.expr(end, &mut pre));
                PStmt::ForRange { var, start, end, inclusive, body: self.stmts(body) }
            }
            PStmt::ForIn { pat, iter, body } => PStmt::ForIn { pat, iter: self.expr(iter, &mut pre), body: self.stmts(body) },
            PStmt::Match { scrutinee, arms } => {
                let scrutinee = self.expr(scrutinee, &mut pre);
                PStmt::Match { scrutinee, arms: self.arms(arms, Sink::Discard) }
            }
            PStmt::Return(e) => PStmt::Return(e.map(|e| self.expr(e, &mut pre))),
            PStmt::Defer(inner) => {
                let mut v = self.stmt(*inner);
                PStmt::Defer(Box::new(if v.len() == 1 { v.remove(0) } else { PStmt::Block(v) }))
            }
            PStmt::Labeled { label, stmt } => {
                // Hoisted pre-statements stay outside; the label stays on the loop itself.
                let mut inner = self.stmt(*stmt);
                let last = inner.pop().expect("labeled statement lowers to at least one statement");
                inner.push(PStmt::Labeled { label, stmt: Box::new(last) });
                return inner;
            }
            other @ (PStmt::Break | PStmt::Continue | PStmt::BreakTo(_) | PStmt::ContinueTo(_)) => other,
        };
        pre.push(main);
        pre
    }

    fn while_loop(&mut self, cond: PExpr, body: Vec<PStmt>) -> Vec<PStmt> {
        let mut pre = Vec::new();
        let c = self.expr(cond, &mut pre);
        let body = self.stmts(body);
        if pre.is_empty() {
            return vec![PStmt::While { cond: c, body }];
        }
        let not = PExpr::new(Ty::Bool, PK::Unary(UnKind::Not, c));
        let mut inner = pre;
        inner.push(PStmt::If { cond: not, then: vec![PStmt::Break], els: vec![] });
        inner.extend(body);
        vec![PStmt::Loop { body: inner }]
    }

    fn arms(&mut self, arms: Vec<PArm>, sink: Sink) -> Vec<PArm> {
        arms.into_iter()
            .map(|a| PArm { pat: a.pat, guard: a.guard, body: PBlock { stmts: self.block(a.body, sink.clone()), tail: None } })
            .collect()
    }

    /// Flatten operands left to right, spilling earlier ones when a later one hoists statements.
    fn operands(&mut self, mut kids: Vec<&mut PExpr>, pre: &mut Vec<PStmt>) {
        for i in 0..kids.len() {
            let taken = std::mem::replace(&mut *kids[i], PExpr::unit());
            let mut local_pre = Vec::new();
            let v = self.expr(taken, &mut local_pre);
            if !local_pre.is_empty() {
                for k in kids.iter_mut().take(i) {
                    if !is_place_or_trivial(k) {
                        let t = self.fresh("t");
                        let ty = k.ty.clone();
                        let old = std::mem::replace(&mut **k, local(&t, &ty));
                        pre.push(PStmt::Let { name: t, ty, mutable: false, init: Some(old) });
                    }
                }
                pre.extend(local_pre);
            }
            *kids[i] = v;
        }
    }

    fn expr(&mut self, mut e: PExpr, pre: &mut Vec<PStmt>) -> PExpr {
        let line = e.line;
        let ty = e.ty.clone();
        let kind = std::mem::replace(&mut *e.kind, PK::Unit);
        match kind {
            PK::If { cond, then, els } => self.if_expr(cond, then, els, &ty, pre),
            PK::Match { scrutinee, arms } => self.match_expr(scrutinee, arms, &ty, pre),
            PK::Block(b) => {
                for s in b.stmts {
                    pre.extend(self.stmt(s));
                }
                match b.tail {
                    Some(t) => self.expr(t, pre),
                    None => PExpr::unit(),
                }
            }
            PK::Try { expr, fn_ret, conv } => self.try_expr(expr, fn_ret, conv, &ty, pre),
            PK::Binary(op @ (BinKind::And | BinKind::Or), l, r) => self.short_circuit(op, l, r, pre),
            PK::Closure { params, ret, body } => {
                let sink = if ret == Ty::Unit { Sink::Discard } else { Sink::Return };
                let stmts = self.block(body, sink);
                PExpr::new(ty, PK::Closure { params, ret, body: PBlock { stmts, tail: None } }).at(line)
            }
            mut other => {
                let kids = children_mut(&mut other);
                self.operands(kids, pre);
                PExpr::new(ty, other).at(line)
            }
        }
    }

    fn temp_decl(&mut self, ty: &Ty, pre: &mut Vec<PStmt>) -> Option<String> {
        if *ty == Ty::Unit || ty.is_unknown() {
            return None;
        }
        let t = self.fresh("v");
        pre.push(PStmt::Let { name: t.clone(), ty: ty.clone(), mutable: true, init: None });
        Some(t)
    }

    fn if_expr(&mut self, cond: PExpr, then: PBlock, els: PBlock, ty: &Ty, pre: &mut Vec<PStmt>) -> PExpr {
        let c = self.expr(cond, pre);
        let tmp = self.temp_decl(ty, pre);
        let sink = tmp.clone().map(Sink::Assign).unwrap_or(Sink::Discard);
        let (t, e) = (self.block(then, sink.clone()), self.block(els, sink));
        pre.push(PStmt::If { cond: c, then: t, els: e });
        tmp.map(|t| local(&t, ty)).unwrap_or_else(PExpr::unit)
    }

    fn match_expr(&mut self, scrutinee: PExpr, arms: Vec<PArm>, ty: &Ty, pre: &mut Vec<PStmt>) -> PExpr {
        let s = self.expr(scrutinee, pre);
        let tmp = self.temp_decl(ty, pre);
        let sink = tmp.clone().map(Sink::Assign).unwrap_or(Sink::Discard);
        let arms = self.arms(arms, sink);
        pre.push(PStmt::Match { scrutinee: s, arms });
        tmp.map(|t| local(&t, ty)).unwrap_or_else(PExpr::unit)
    }

    fn short_circuit(&mut self, op: BinKind, l: PExpr, r: PExpr, pre: &mut Vec<PStmt>) -> PExpr {
        let l = self.expr(l, pre);
        let mut local_pre = Vec::new();
        let r = self.expr(r, &mut local_pre);
        if local_pre.is_empty() {
            return PExpr::new(Ty::Bool, PK::Binary(op, l, r));
        }
        let t = self.fresh("b");
        pre.push(PStmt::Let { name: t.clone(), ty: Ty::Bool, mutable: true, init: Some(l) });
        let guard = if op == BinKind::And { local(&t, &Ty::Bool) } else { PExpr::new(Ty::Bool, PK::Unary(UnKind::Not, local(&t, &Ty::Bool))) };
        local_pre.push(PStmt::Assign { target: local(&t, &Ty::Bool), op: None, value: r });
        pre.push(PStmt::If { cond: guard, then: local_pre, els: vec![] });
        local(&t, &Ty::Bool)
    }

    fn try_expr(&mut self, inner: PExpr, fn_ret: Ty, conv: Option<(String, String)>, ty: &Ty, pre: &mut Vec<PStmt>) -> PExpr {
        let v = self.expr(inner, pre);
        let t = self.fresh("q");
        let vty = v.ty.clone();
        pre.push(PStmt::Let { name: t.clone(), ty: vty.clone(), mutable: false, init: Some(v) });
        let err_name = self.fresh("e");
        let (pat, ret) = match &vty {
            Ty::Option(_) => (PPat::None, PExpr::new(fn_ret.clone(), PK::None)),
            Ty::Result(_, et) => {
                let mut e = local(&err_name, et);
                if let (Some((owner, name)), Ty::Result(_, to)) = (conv, &fn_ret) {
                    e = PExpr::new((**to).clone(), PK::StaticCall { owner, name, args: vec![e] });
                }
                (PPat::Err(Box::new(PPat::Bind { name: err_name.clone(), mutable: false })), PExpr::new(fn_ret.clone(), PK::Err(e)))
            }
            _ => (PPat::Wild, PExpr::unit()),
        };
        let early = PArm { pat, guard: None, body: PBlock::of(vec![PStmt::Return(Some(ret))]) };
        let rest = PArm { pat: PPat::Wild, guard: None, body: PBlock::default() };
        pre.push(PStmt::Match { scrutinee: local(&t, &vty), arms: vec![early, rest] });
        PExpr::new(ty.clone(), PK::Method { recv: local(&t, &vty), name: "unwrap".into(), args: vec![], res: Resolution::Builtin })
    }
}

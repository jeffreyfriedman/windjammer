//! Statement lowering, including value-producing tails.

use super::{Lowerer, PResult};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{from_ast, Ty};
use crate::parser::{CompoundOp, Expression, MatchArm, Pattern, Statement, UnaryOp};

fn compound_kind(op: &CompoundOp) -> BinKind {
    match op {
        CompoundOp::Add => BinKind::Add,
        CompoundOp::Sub => BinKind::Sub,
        CompoundOp::Mul => BinKind::Mul,
        CompoundOp::Div => BinKind::Div,
        CompoundOp::Mod => BinKind::Rem,
        CompoundOp::BitAnd => BinKind::BitAnd,
        CompoundOp::BitOr => BinKind::BitOr,
        CompoundOp::BitXor => BinKind::BitXor,
        CompoundOp::Shl => BinKind::Shl,
        CompoundOp::Shr => BinKind::Shr,
    }
}

impl<'e> Lowerer<'e> {
    /// Statements with an optional value tail. When `need_value`, a trailing expression (or
    /// trailing `if`/`match`) becomes the block's value.
    pub(super) fn stmts_with_tail(&mut self, stmts: &[&Statement], want: Option<&Ty>, need_value: bool) -> PResult<PBlock> {
        let mut out = Vec::new();
        let mut tail = None;
        for (i, s) in stmts.iter().enumerate() {
            let last = i + 1 == stmts.len();
            if last && need_value {
                match s {
                    Statement::Expression { expr, .. } => {
                        let e = self.expr(expr, want)?;
                        tail = Some(e);
                        continue;
                    }
                    Statement::If { .. } | Statement::Match { .. } => {
                        tail = Some(self.stmt_as_expr(s, want)?);
                        continue;
                    }
                    _ => {}
                }
            }
            out.extend(self.stmt(s)?);
        }
        Ok(PBlock { stmts: out, tail })
    }

    pub(super) fn stmts(&mut self, stmts: &[&Statement]) -> PResult<Vec<PStmt>> {
        self.push();
        let r = self.stmts_with_tail(stmts, None, false).map(|b| b.stmts);
        self.pop();
        r
    }

    fn stmt_as_expr(&mut self, s: &Statement, want: Option<&Ty>) -> PResult<PExpr> {
        match s {
            Statement::If { condition, then_block, else_block, .. } => {
                let cond = self.expr(condition, Some(&Ty::Bool))?;
                let then = self.block_value(then_block, want)?;
                let els = match else_block {
                    Some(b) => self.block_value(b, want.or(then.tail.as_ref().map(|t| &t.ty)))?,
                    None => PBlock::default(),
                };
                let ty = then.tail.as_ref().or(els.tail.as_ref()).map(|t| t.ty.clone()).unwrap_or(Ty::Unit);
                Ok(PExpr::new(ty, PK::If { cond, then, els }))
            }
            Statement::Match { value, arms, .. } => self.match_expr(value, arms, want, true),
            _ => self.err("statement used as expression"),
        }
    }

    pub(super) fn match_expr(&mut self, value: &Expression, arms: &[MatchArm], want: Option<&Ty>, need_value: bool) -> PResult<PExpr> {
        let scrut = self.expr(value, None)?;
        let mut out = Vec::new();
        let mut result_ty: Option<Ty> = want.cloned();
        for arm in arms {
            self.push();
            let pat = self.pattern(&arm.pattern, &scrut.ty)?;
            let guard = arm.guard.map(|g| self.expr(g, Some(&Ty::Bool))).transpose()?;
            let body = self.arm_body(arm.body, result_ty.as_ref(), need_value)?;
            self.pop();
            if let Some(t) = &body.tail {
                if result_ty.as_ref().map(|r| r.is_unknown()).unwrap_or(true) && !t.ty.is_unknown() {
                    result_ty = Some(t.ty.clone());
                }
                if let Some(r) = result_ty.clone() {
                    self.constrain(&r, &t.ty);
                }
            }
            out.push(PArm { pat, guard, body });
        }
        let ty = if need_value { result_ty.unwrap_or(Ty::Unit) } else { Ty::Unit };
        Ok(PExpr::new(ty, PK::Match { scrutinee: scrut, arms: out }))
    }

    fn arm_body(&mut self, body: &Expression, want: Option<&Ty>, need_value: bool) -> PResult<PBlock> {
        let want = want.filter(|t| !t.is_unknown());
        match body {
            Expression::Block { statements, .. } => self.stmts_with_tail(statements, want, need_value),
            other if need_value => Ok(PBlock { stmts: vec![], tail: Some(self.expr(other, want)?) }),
            other => Ok(PBlock::of(vec![PStmt::Expr(self.expr(other, want)?)])),
        }
    }

    pub(super) fn stmt(&mut self, s: &Statement) -> PResult<Vec<PStmt>> {
        if let Some(l) = s.location() {
            self.line = l.line as u32;
        }
        match s {
            Statement::Let { pattern, mutable, type_, value, else_block, .. } => self.let_stmt(pattern, *mutable, type_.as_ref(), value, else_block.as_deref()),
            Statement::Const { name, type_, value, .. } | Statement::Static { name, type_, value, .. } => {
                let ty = from_ast(type_, &self.generics);
                let init = self.expr(value, Some(&ty))?;
                let out = self.declare(name, ty.clone(), false);
                Ok(vec![PStmt::Let { name: out, ty, mutable: false, init: Some(init) }])
            }
            Statement::Assignment { target, value, compound_op, .. } => self.assignment(target, value, compound_op.as_ref()),
            Statement::Return { value, .. } => {
                let want = self.rets.last().cloned();
                let v = value.map(|e| self.expr(e, want.as_ref().filter(|t| !t.is_unknown()))).transpose()?;
                if let (Some(w), Some(v)) = (&want, &v) {
                    self.constrain(w, &v.ty);
                }
                Ok(vec![PStmt::Return(v.map(|e| self.own_for_return(e)))])
            }
            Statement::Expression { expr, .. } => Ok(vec![PStmt::Expr(self.expr(expr, None)?)]),
            Statement::If { condition, then_block, else_block, .. } => {
                let cond = self.expr(condition, Some(&Ty::Bool))?;
                let then = self.stmts(then_block)?;
                let els = match else_block {
                    Some(b) => self.stmts(b)?,
                    None => vec![],
                };
                Ok(vec![PStmt::If { cond, then, els }])
            }
            Statement::Match { value, arms, .. } => {
                let m = self.match_expr(value, arms, None, false)?;
                match *m.kind {
                    PK::Match { scrutinee, arms } => Ok(vec![PStmt::Match { scrutinee, arms }]),
                    _ => self.err("match lowering"),
                }
            }
            Statement::For { pattern, iterable, body, .. } => self.for_stmt(pattern, iterable, body),
            Statement::Loop { body, .. } => Ok(vec![PStmt::Loop { body: self.stmts(body)? }]),
            Statement::While { condition, body, .. } => {
                let cond = self.expr(condition, Some(&Ty::Bool))?;
                Ok(vec![PStmt::While { cond, body: self.stmts(body)? }])
            }
            Statement::Break { .. } => Ok(vec![PStmt::Break]),
            Statement::Continue { .. } => Ok(vec![PStmt::Continue]),
            Statement::Defer { statement, .. } => {
                let inner = self.stmt(statement)?;
                Ok(inner.into_iter().map(|s| PStmt::Defer(Box::new(s))).collect())
            }
            Statement::Use { .. } => Ok(vec![]),
            Statement::Thread { .. } => self.err("thread blocks"),
            Statement::Async { .. } => self.err("async blocks"),
        }
    }

    fn let_stmt(&mut self, pattern: &Pattern, mutable: bool, ty: Option<&crate::parser::Type>, value: &Expression, els: Option<&[&Statement]>) -> PResult<Vec<PStmt>> {
        let declared = ty.map(|t| from_ast(t, &self.generics)).map(|t| self.freshen(&t));
        let init = self.expr(value, declared.as_ref())?;
        if let Some(d) = &declared {
            self.constrain(d, &init.ty);
        }
        let ty = declared.unwrap_or_else(|| init.ty.clone());
        let init = self.own(init);
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => {
                let out = self.declare(name, ty.clone(), false);
                let m = mutable || matches!(pattern, Pattern::MutBinding(_));
                if m {
                    self.set_mutable(name);
                }
                Ok(vec![PStmt::Let { name: out, ty, mutable: m, init: Some(init) }])
            }
            Pattern::Wildcard => Ok(vec![PStmt::Expr(init)]),
            other => {
                let els = els.map(|b| self.stmts(b)).transpose()?;
                let pat = self.pattern(other, &ty)?;
                Ok(vec![PStmt::LetPat { pat, init, els }])
            }
        }
    }

    fn assignment(&mut self, target: &Expression, value: &Expression, op: Option<&CompoundOp>) -> PResult<Vec<PStmt>> {
        if let Some(stmts) = self.try_entry_assignment(target, value, op)? {
            return Ok(stmts);
        }
        let t = self.expr(target, None)?;
        let want = t.ty.clone();
        let v = self.expr(value, Some(&want).filter(|w| !w.is_unknown()))?;
        self.constrain(&t.ty, &v.ty);
        let v = if op.is_none() { self.own(v) } else { v };
        Ok(vec![PStmt::Assign { target: t, op: op.map(compound_kind), value: v }])
    }

    /// `*map.entry(k).or_insert(d) += v` => `if !map.contains_key(k) { map.insert(k, d) }; map[k] += v`.
    fn try_entry_assignment(&mut self, target: &Expression, value: &Expression, op: Option<&CompoundOp>) -> PResult<Option<Vec<PStmt>>> {
        let inner = match target {
            Expression::Unary { op: UnaryOp::Deref, operand, .. } => *operand,
            other => other,
        };
        let Expression::MethodCall { object, method, arguments, .. } = inner else { return Ok(None) };
        if method != "or_insert" && method != "or_default" {
            return Ok(None);
        }
        let Expression::MethodCall { object: map_e, method: m2, arguments: kargs, .. } = *object else { return Ok(None) };
        if m2 != "entry" || kargs.len() != 1 {
            return Ok(None);
        }
        let map = self.expr(map_e, None)?;
        let Ty::Map(kt, vt) = map.ty.clone() else { return self.err("entry() on non-map") };
        let key = self.expr(kargs[0].1, Some(&kt))?;
        let default = match arguments.first() {
            Some((_, d)) => self.expr(d, Some(&vt))?,
            None => return self.err("or_default() entries are not supported"),
        };
        let tmp = self.fresh("k");
        let key_ty = (*kt).clone();
        let key_local = PExpr::new(key_ty.clone(), PK::Local(tmp.clone()));
        let has = PExpr::new(Ty::Bool, PK::Method { recv: map.clone(), name: "contains_key".into(), args: vec![key_local.clone()], res: Resolution::Builtin });
        let not = PExpr::new(Ty::Bool, PK::Unary(UnKind::Not, has));
        let ins = PExpr::new(Ty::Unit, PK::Method { recv: map.clone(), name: "insert".into(), args: vec![key_local.clone(), default], res: Resolution::Builtin });
        let slot = PExpr::new((*vt).clone(), PK::Index(map, key_local));
        let v = self.expr(value, Some(&vt))?;
        Ok(Some(vec![
            PStmt::Let { name: tmp, ty: key_ty, mutable: false, init: Some(key) },
            PStmt::If { cond: not, then: vec![PStmt::Expr(ins)], els: vec![] },
            PStmt::Assign { target: slot, op: op.map(compound_kind), value: v },
        ]))
    }

    fn for_stmt(&mut self, pattern: &Pattern, iterable: &Expression, body: &[&Statement]) -> PResult<Vec<PStmt>> {
        if let Expression::Range { start, end, inclusive, .. } = iterable {
            if let Pattern::Identifier(var) | Pattern::MutBinding(var) = pattern {
                return self.for_range(var, start, end, *inclusive, body);
            }
            if matches!(pattern, Pattern::Wildcard) {
                return self.for_range("_", start, end, *inclusive, body);
            }
        }
        let it = self.expr(iterable, None)?;
        let elem = match &it.ty {
            Ty::Map(k, v) => Ty::Tuple(vec![(**k).clone(), (**v).clone()]),
            Ty::Vec(t) | Ty::Iter(t) | Ty::Set(t) | Ty::Range(t) | Ty::Option(t) => (**t).clone(),
            other => return self.err(format!("iterating over `{}`", other.describe())),
        };
        self.push();
        let pat = self.pattern(pattern, &elem)?;
        let body = self.stmts(body);
        self.pop();
        Ok(vec![PStmt::ForIn { pat, iter: it, body: body? }])
    }

    fn for_range(&mut self, var: &str, start: &Expression, end: &Expression, inclusive: bool, body: &[&Statement]) -> PResult<Vec<PStmt>> {
        let r = self.operand_pair(start, end, None)?;
        let (s, e) = r;
        let ty = s.ty.join(&e.ty);
        self.push();
        let out = if var == "_" { self.fresh("_i") } else { self.declare(var, ty.clone(), false) };
        let body = self.stmts(body);
        self.pop();
        Ok(vec![PStmt::ForRange { var: out, start: s, end: e, inclusive, body: body? }])
    }
}

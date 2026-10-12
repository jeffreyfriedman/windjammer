//! Statement lowering, including value-producing tails.

use super::{LoopCtx, Lowerer, PResult};
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
                    // `loop { .. break v .. }` as the value of a block: a result local receives
                    // each `break v`, and the block's value is that local.
                    Statement::Loop { body, label, .. } if loop_has_break_value(body, label.as_deref()) => {
                        let result = self.fresh("loop_val");
                        let ty = want.cloned().filter(|t| !t.is_unknown()).unwrap_or_else(|| self.new_var());
                        out.push(PStmt::Let { name: result.clone(), ty: ty.clone(), mutable: true, init: None });
                        self.loop_stack.push(LoopCtx { label: label.clone(), used: false, result: Some((result.clone(), ty.clone())) });
                        let r = self.stmts(body).map(|b| vec![PStmt::Loop { body: b }]);
                        out.extend(self.finish_loop(r)?);
                        let ty = self.resolve(&ty);
                        tail = Some(PExpr::new(ty, PK::Local(result)));
                        continue;
                    }
                    _ => {}
                }
            }
            out.extend(self.stmt(s)?);
        }
        Ok(PBlock { stmts: out, tail })
    }

    /// Pop the loop context pushed for a loop whose statements were lowered to `r`; wrap in
    /// `Labeled` when a labeled jump targeted it.
    fn finish_loop(&mut self, r: PResult<Vec<PStmt>>) -> PResult<Vec<PStmt>> {
        let ctx = self.loop_stack.pop().expect("loop context");
        let mut stmts = r?;
        if let (Some(label), true) = (ctx.label, ctx.used) {
            if let Some(last) = stmts.pop() {
                stmts.push(PStmt::Labeled { label, stmt: Box::new(last) });
            }
        }
        Ok(stmts)
    }

    fn mark_label_used(&mut self, label: &str) -> PResult<()> {
        match self.loop_stack.iter_mut().rev().find(|c| c.label.as_deref() == Some(label)) {
            Some(ctx) => {
                ctx.used = true;
                Ok(())
            }
            None => self.err(format!("`'{label}` does not name an enclosing loop")),
        }
    }

    fn break_stmt(&mut self, label: Option<&str>, value: Option<&Expression>) -> PResult<Vec<PStmt>> {
        let idx = match label {
            Some(l) => {
                self.mark_label_used(l)?;
                self.loop_stack.iter().rposition(|c| c.label.as_deref() == Some(l))
            }
            None => self.loop_stack.len().checked_sub(1),
        };
        let jump = match label {
            Some(l) => PStmt::BreakTo(l.to_string()),
            None => PStmt::Break,
        };
        let Some(v) = value else { return Ok(vec![jump]) };
        let Some((name, ty)) = idx.and_then(|i| self.loop_stack[i].result.clone()) else {
            return self.err("`break` with a value outside a value-producing `loop`");
        };
        let want = Some(&ty).filter(|t| !t.is_unknown()).cloned();
        let e = self.expr(v, want.as_ref())?;
        self.constrain(&ty, &e.ty);
        let target = PExpr::new(ty, PK::Local(name));
        Ok(vec![PStmt::Assign { target, op: None, value: e }, jump])
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
                if result_ty.as_ref().map(|r| matches!(r, Ty::Unknown)).unwrap_or(true) && !matches!(t.ty, Ty::Unknown) {
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
            Statement::For { pattern, iterable, body, label, .. } => {
                self.loop_stack.push(LoopCtx { label: label.clone(), used: false, result: None });
                let r = self.for_stmt(pattern, iterable, body);
                self.finish_loop(r)
            }
            Statement::Loop { body, label, .. } => {
                self.loop_stack.push(LoopCtx { label: label.clone(), used: false, result: None });
                let r = self.stmts(body).map(|b| vec![PStmt::Loop { body: b }]);
                self.finish_loop(r)
            }
            Statement::While { condition, body, label, .. } => {
                self.loop_stack.push(LoopCtx { label: label.clone(), used: false, result: None });
                let r = (|| {
                    let cond = self.expr(condition, Some(&Ty::Bool))?;
                    Ok(vec![PStmt::While { cond, body: self.stmts(body)? }])
                })();
                self.finish_loop(r)
            }
            Statement::Break { label, value, .. } => self.break_stmt(label.as_deref(), *value),
            Statement::Continue { label, .. } => {
                match label {
                    None => Ok(vec![PStmt::Continue]),
                    Some(l) => {
                        self.mark_label_used(l)?;
                        Ok(vec![PStmt::ContinueTo(l.clone())])
                    }
                }
            }
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
        if let (Expression::Unary { op: UnaryOp::MutRef, operand, .. }, Pattern::Identifier(name) | Pattern::MutBinding(name)) = (value, pattern) {
            if let Some(stmts) = self.let_mut_borrow(name, operand)? {
                return Ok(stmts);
            }
        }
        let declared = ty.map(|t| from_ast(t, &self.generics)).map(|t| self.freshen(&t));
        let mut init = self.expr(value, declared.as_ref())?;
        if let Some(d) = &declared {
            self.constrain(d, &init.ty);
        }
        let ty = match declared {
            Some(d) => d,
            None => {
                // `let v = []` / `Vec::new()` / `None`: unknown element types become inference
                // variables that later uses (`v.push(1)`) bind.
                init.ty = self.freshen(&init.ty);
                init.ty.clone()
            }
        };
        let init = self.own(init);
        match pattern {
            Pattern::Identifier(name) | Pattern::MutBinding(name) => {
                let out = self.declare(name, ty.clone(), false);
                let m = mutable || matches!(pattern, Pattern::MutBinding(_));
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

    /// `let r = &mut place`: `r` aliases the place (never a copy). A `&mut` of a temporary is an
    /// unaliased value and takes the ordinary path (`None`).
    fn let_mut_borrow(&mut self, name: &str, operand: &Expression) -> PResult<Option<Vec<PStmt>>> {
        if !matches!(operand, Expression::Identifier { .. } | Expression::FieldAccess { .. } | Expression::Index { .. }) {
            return Ok(None);
        }
        let place = self.expr(operand, None)?;
        if !place.is_place() {
            return self.err("`&mut` of an expression that is not a place");
        }
        let ty = place.ty.clone();
        let out = self.declare(name, ty.clone(), true);
        let line = place.line;
        let init = PExpr::new(ty.clone(), PK::BorrowMut(place)).at(line);
        Ok(Some(vec![PStmt::Let { name: out, ty, mutable: false, init: Some(init) }]))
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

/// Does `body` contain a `break` carrying a value that targets the loop with `label` (or, for an
/// unlabeled break, this loop)? Nested loops shadow unlabeled breaks only.
fn loop_has_break_value(body: &[&Statement], label: Option<&str>) -> bool {
    fn walk(ss: &[&Statement], label: Option<&str>, nested: bool) -> bool {
        ss.iter().any(|s| match s {
            Statement::Break { value: Some(_), label: l, .. } => match l {
                None => !nested,
                Some(l) => Some(l.as_str()) == label,
            },
            Statement::If { then_block, else_block, .. } => {
                walk(then_block, label, nested) || else_block.as_ref().is_some_and(|b| walk(b, label, nested))
            }
            Statement::Match { arms, .. } => arms.iter().any(|a| expr_has(a.body, label, nested)),
            Statement::Loop { body, .. } | Statement::While { body, .. } | Statement::For { body, .. } => walk(body, label, true),
            Statement::Expression { expr, .. } => expr_has(expr, label, nested),
            _ => false,
        })
    }
    fn expr_has(e: &Expression, label: Option<&str>, nested: bool) -> bool {
        match e {
            Expression::Block { statements, .. } => walk(statements, label, nested),
            _ => false,
        }
    }
    walk(body, label, false)
}

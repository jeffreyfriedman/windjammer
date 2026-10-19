//! Statement emission for Go.

use super::names::ident;
use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{IntKind, Ty};

fn has_break(ss: &[PStmt]) -> bool {
    ss.iter().any(|s| match s {
        PStmt::Break | PStmt::BreakTo(_) => true,
        PStmt::Labeled { stmt, .. } => has_break(std::slice::from_ref(stmt)),
        PStmt::If { then, els, .. } => has_break(then) || has_break(els),
        PStmt::Block(b) => has_break(b),
        PStmt::Match { arms, .. } => arms.iter().any(|a| has_break(&a.body.stmts)),
        PStmt::Defer(s) => has_break(std::slice::from_ref(s)),
        _ => false,
    })
}

impl<'m> Go<'m> {
    /// Does the statement list end in a Go "terminating statement" (so no trailing `panic` needed)?
    pub fn terminating(&self, ss: &[PStmt]) -> bool {
        match ss.last() {
            Some(PStmt::Return(_)) => true,
            Some(PStmt::Expr(e)) => matches!(&*e.kind, PK::Panic(_)),
            Some(PStmt::If { then, els, .. }) => !els.is_empty() && self.terminating(then) && self.terminating(els),
            Some(PStmt::Block(b)) => self.terminating(b),
            Some(PStmt::Loop { body }) => !has_break(body),
            Some(PStmt::Match { arms, .. }) => arms.iter().all(|a| self.terminating(&a.body.stmts)) && !arms.iter().any(|a| has_break(&a.body.stmts)),
            _ => false,
        }
    }

    pub fn stmts(&mut self, ss: &[PStmt]) -> PResult<()> {
        for s in ss {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn block(&mut self, head: &str, ss: &[PStmt]) -> PResult<()> {
        self.line(&format!("{head} {{"));
        self.ind += 1;
        self.stmts(ss)?;
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    /// Emit a loop; the label is only declared when a `break`/`continue` needs it.
    fn looped(&mut self, head: &str, body: &[PStmt], prelude: &[String]) -> PResult<()> {
        let label = self.fresh("L");
        self.loops.push(label.clone());
        let user = self.pending_label.take();
        if let Some(u) = &user {
            self.user_labels.push((u.clone(), label.clone()));
        }
        let saved = std::mem::take(&mut self.out);
        let ind = self.ind;
        self.ind += 1;
        for p in prelude {
            self.line(p);
        }
        let r = self.stmts(body);
        self.ind = ind;
        let inner = std::mem::replace(&mut self.out, saved);
        self.loops.pop();
        if user.is_some() {
            self.user_labels.pop();
        }
        r?;
        let used = inner.contains(&format!("break {label}")) || inner.contains(&format!("continue {label}"));
        if used {
            self.line(&format!("{label}:"));
        }
        self.line(&format!("{head} {{"));
        self.out.push_str(&inner);
        self.line("}");
        Ok(())
    }

    pub fn stmt(&mut self, s: &PStmt) -> PResult<()> {
        match s {
            PStmt::Let { name, ty, init: Some(e), .. } if matches!(&*e.kind, PK::BorrowMut(_)) && !self.is_ref_type(ty) => {
                // `let r = &mut place`: a pointer; uses of `r` dereference it (`ptr_locals`).
                let PK::BorrowMut(place) = &*e.kind else { unreachable!() };
                let (t, n) = (self.ty(ty)?, ident(name));
                let p = self.addr(place)?;
                self.line(&format!("var {n} *{t} = {p}"));
                self.line(&format!("_ = {n}"));
                self.ptr_locals.insert(name.clone(), ty.clone());
            }
            PStmt::Let { name, ty, init, .. } => {
                let t = self.ty(ty)?;
                let n = ident(name);
                match init {
                    Some(e) => {
                        let v = self.expr(e)?;
                        self.line(&format!("var {n} {t} = {v}"));
                    }
                    None => self.line(&format!("var {n} {t}")),
                }
                self.line(&format!("_ = {n}"));
            }
            PStmt::LetPat { pat, init, els } => self.let_pat(pat, init, els.as_deref())?,
            PStmt::Assign { target, op, value } => self.assign(target, *op, value)?,
            PStmt::Expr(e) => self.expr_stmt(e)?,
            PStmt::If { cond, then, els } => {
                let c = self.expr(cond)?;
                self.line(&format!("if {c} {{"));
                self.ind += 1;
                self.stmts(then)?;
                self.ind -= 1;
                if els.is_empty() {
                    self.line("}");
                } else {
                    self.line("} else {");
                    self.ind += 1;
                    self.stmts(els)?;
                    self.ind -= 1;
                    self.line("}");
                }
            }
            PStmt::While { cond, body } => {
                let c = self.expr(cond)?;
                self.looped(&format!("for {c}"), body, &[])?;
            }
            PStmt::Loop { body } => self.looped("for", body, &[])?,
            PStmt::Block(b) => self.block("", b)?,
            PStmt::ForRange { var, start, end, inclusive, body } => self.for_range(var, start, end, *inclusive, body)?,
            PStmt::ForIn { pat, iter, body } => self.for_in(pat, iter, body)?,
            PStmt::Match { scrutinee, arms } => self.match_stmt(scrutinee, arms)?,
            PStmt::Labeled { label, stmt } => {
                self.pending_label = Some(label.clone());
                let r = self.stmt(stmt);
                self.pending_label = None;
                r?;
            }
            PStmt::BreakTo(l) | PStmt::ContinueTo(l) => {
                let Some(go) = self.user_labels.iter().rev().find(|(u, _)| u == l).map(|(_, g)| g.clone()) else {
                    return self.unsupported("labeled jump to an unknown loop");
                };
                let kw = if matches!(s, PStmt::BreakTo(_)) { "break" } else { "continue" };
                self.line(&format!("{kw} {go}"));
            }
            PStmt::Break => {
                let l = self.loops.last().cloned();
                match l {
                    Some(l) => self.line(&format!("break {l}")),
                    None => self.line("break"),
                }
            }
            PStmt::Continue => {
                let l = self.loops.last().cloned();
                match l {
                    Some(l) => self.line(&format!("continue {l}")),
                    None => self.line("continue"),
                }
            }
            PStmt::Return(None) => self.line("return"),
            PStmt::Return(Some(e)) => {
                if e.ty == Ty::Unit {
                    self.expr_stmt(e)?;
                    self.line("return");
                } else {
                    let v = self.expr(e)?;
                    self.line(&format!("return {v}"));
                }
            }
            PStmt::Defer(inner) => {
                self.line("defer func() {");
                self.ind += 1;
                self.stmt(inner)?;
                self.ind -= 1;
                self.line("}()");
            }
        }
        Ok(())
    }

    fn expr_stmt(&mut self, e: &PExpr) -> PResult<()> {
        let v = self.expr(e)?;
        let is_call = matches!(&*e.kind, PK::Call { .. } | PK::StaticCall { .. } | PK::StdCall { .. } | PK::CallValue { .. } | PK::Method { .. } | PK::Print { .. } | PK::Panic(_));
        if is_call && e.ty == Ty::Unit || matches!(&*e.kind, PK::Print { .. } | PK::Panic(_)) {
            self.line(&v);
        } else {
            self.line(&format!("_ = {v}"));
        }
        Ok(())
    }

    fn let_pat(&mut self, pat: &PPat, init: &PExpr, els: Option<&[PStmt]>) -> PResult<()> {
        let v = self.expr(init)?;
        let t = self.fresh("p");
        self.line(&format!("{t} := {v}"));
        self.line(&format!("_ = {t}"));
        let (mut conds, mut binds) = (Vec::new(), Vec::new());
        self.pat(pat, &t, &init.ty, &mut conds, &mut binds)?;
        if let (Some(els), false) = (els, conds.is_empty()) {
            self.block(&format!("if !({})", conds.join(" && ")), els)?;
        }
        for (n, e) in binds {
            self.line(&format!("{n} := {e}"));
            self.line(&format!("_ = {n}"));
        }
        Ok(())
    }

    fn assign(&mut self, target: &PExpr, op: Option<BinKind>, value: &PExpr) -> PResult<()> {
        let v = self.expr(value)?;
        let t = self.place(target)?;
        let rhs = match op {
            None => v,
            Some(op) => self.compound(op, target, &t, &v, value)?,
        };
        if t == rhs {
            return Ok(()); // `x = x` (go vet rejects self-assignment; it is a no-op)
        }
        self.line(&format!("{t} = {rhs}"));
        Ok(())
    }

    fn compound(&mut self, op: BinKind, target: &PExpr, t: &str, v: &str, value: &PExpr) -> PResult<String> {
        let _ = value;
        match &target.ty {
            Ty::Int(_) => Ok(super::num::int_binary(op, t, v)),
            Ty::F64 | Ty::F32 => Ok(match op {
                BinKind::Rem => format!("wjFmod({t}, {v})"),
                _ => format!("({t} {} {v})", match op { BinKind::Add => "+", BinKind::Sub => "-", BinKind::Mul => "*", _ => "/" }),
            }),
            Ty::Str if op == BinKind::Add => Ok(format!("({t} + {v})")),
            other => self.unsupported(format!("compound assignment on `{}`", other.describe())),
        }
    }

    fn for_range(&mut self, var: &str, start: &PExpr, end: &PExpr, inclusive: bool, body: &[PStmt]) -> PResult<()> {
        let (s, e) = (self.expr(start)?, self.expr(end)?);
        let v = ident(var);
        let k = start.ty.int_kind().unwrap_or(IntKind::I64);
        let t = self.ty(&start.ty)?;
        let end_name = self.fresh("end");
        self.line("{");
        self.ind += 1;
        self.line(&format!("{end_name} := {e}"));
        if inclusive && k.bits() < 64 {
            let w = self.fresh("w");
            let wide = if k.signed() { "int64" } else { "uint64" };
            self.looped(&format!("for {w} := {wide}({s}); {w} <= {wide}({end_name}); {w}++"), body, &[format!("{v} := {t}({w})"), format!("_ = {v}")])?;
        } else {
            let cmp = if inclusive { "<=" } else { "<" };
            self.looped(&format!("for {v} := {s}; {v} {cmp} {end_name}; {v}++"), body, &[format!("_ = {v}")])?;
        }
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    fn for_in(&mut self, pat: &PPat, iter: &PExpr, body: &[PStmt]) -> PResult<()> {
        let it = self.expr(iter)?;
        let (item, elem_ty) = (self.fresh("it"), self.elem_ty(&iter.ty)?);
        let (mut conds, mut binds) = (Vec::new(), Vec::new());
        self.pat(pat, &item, &elem_ty, &mut conds, &mut binds)?;
        let mut prelude = Vec::new();
        for (n, e) in binds {
            prelude.push(format!("{n} := {e}"));
            prelude.push(format!("_ = {n}"));
        }
        let by_ref = matches!(&*iter.kind, PK::Method { name, .. } if name == "iter_mut" || name == "values_mut");
        match &iter.ty {
            Ty::Map(..) => {
                let (k, v) = (self.fresh("k"), self.fresh("v"));
                let tt = self.ty(&elem_ty)?;
                let m = self.fresh("m");
                self.line(&format!("{m} := {it}"));
                let mut pre = vec![format!("{v} := {m}[{k}]"), format!("{item} := {tt}{{F0: {k}, F1: {v}}}"), format!("_ = {item}")];
                pre.extend(prelude);
                self.looped(&format!("for _, {k} := range wjSortedKeys({m})"), body, &pre)
            }
            Ty::Set(_) => {
                let k = self.fresh("k");
                let mut pre = vec![format!("{item} := {k}"), format!("_ = {item}")];
                pre.extend(prelude);
                self.looped(&format!("for _, {k} := range wjSortedKeys({it})"), body, &pre)
            }
            _ if by_ref => {
                let PPat::Bind { name, .. } = pat else { return self.unsupported("iter_mut with a destructuring pattern") };
                let (i, base) = (self.fresh("i"), self.fresh("s"));
                self.line(&format!("{base} := {it}"));
                let n = ident(name);
                self.ptr_locals.insert(name.clone(), elem_ty.clone());
                let pre = vec![format!("{n} := &{base}[{i}]"), format!("_ = {n}")];
                let r = self.looped(&format!("for {i} := range {base}"), body, &pre);
                self.ptr_locals.remove(name);
                r
            }
            _ => {
                let mut pre = prelude;
                pre.insert(0, format!("_ = {item}"));
                self.looped(&format!("for _, {item} := range {it}"), body, &pre)
            }
        }
    }

    fn elem_ty(&self, t: &Ty) -> PResult<Ty> {
        match t {
            Ty::Vec(e) | Ty::Iter(e) | Ty::Range(e) | Ty::Set(e) | Ty::Option(e) => Ok((**e).clone()),
            Ty::Map(k, v) => Ok(Ty::Tuple(vec![(**k).clone(), (**v).clone()])),
            other => self.unsupported(format!("iteration over `{}`", other.describe())),
        }
    }

    fn match_stmt(&mut self, scrut: &PExpr, arms: &[PArm]) -> PResult<()> {
        let s = self.expr(scrut)?;
        let (label, sv) = (self.fresh("m"), self.fresh("s"));
        self.line(&format!("{label}: switch {{"));
        self.line("default:");
        self.ind += 1;
        self.line(&format!("{sv} := {s}"));
        self.line(&format!("_ = {sv}"));
        let mut all_end_terminating = true;
        for arm in arms {
            let (mut conds, mut binds) = (Vec::new(), Vec::new());
            self.pat(&arm.pat, &sv, &scrut.ty, &mut conds, &mut binds)?;
            let c = if conds.is_empty() { "true".to_string() } else { conds.join(" && ") };
            self.line(&format!("if {c} {{"));
            self.ind += 1;
            for (n, e) in binds {
                self.line(&format!("{n} := {e}"));
                self.line(&format!("_ = {n}"));
            }
            all_end_terminating &= self.arm_body(arm, &label)?;
            self.ind -= 1;
            self.line("}");
        }
        let _ = all_end_terminating;
        self.line("panic(\"no match arm matched\")");
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    /// Returns true when the arm body ends in a terminating statement.
    fn arm_body(&mut self, arm: &PArm, label: &str) -> PResult<bool> {
        let term = self.terminating(&arm.body.stmts);
        if let Some(g) = &arm.guard {
            let gv = self.expr(g)?;
            self.line(&format!("if {gv} {{"));
            self.ind += 1;
            self.stmts(&arm.body.stmts)?;
            if !term {
                self.line(&format!("break {label}"));
            }
            self.ind -= 1;
            self.line("}");
            Ok(false)
        } else {
            self.stmts(&arm.body.stmts)?;
            if !term {
                self.line(&format!("break {label}"));
            }
            Ok(term)
        }
    }
}

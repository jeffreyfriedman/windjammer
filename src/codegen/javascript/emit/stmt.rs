//! Statement emission.

use super::names::ident;
use super::num;
use super::Js;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Js<'m> {
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

    fn ty_ann(&self, t: &Ty) -> String {
        if self.ts { format!(": {}", self.ts_type(t)) } else { String::new() }
    }

    pub fn stmt(&mut self, s: &PStmt) -> PResult<()> {
        match s {
            PStmt::Let { name, ty, init, .. } => {
                let ann = self.ty_ann(ty);
                match init {
                    Some(e) => {
                        let v = self.expr(e)?;
                        self.line(&format!("let {}{ann} = {v};", ident(name)));
                    }
                    None => self.line(&format!("let {}{ann};", ident(name))),
                }
            }
            PStmt::LetPat { pat, init, els } => self.let_pat(pat, init, els.as_deref())?,
            PStmt::Assign { target, op, value } => self.assign(target, *op, value)?,
            PStmt::Expr(e) => {
                let v = self.expr(e)?;
                self.line(&format!("{v};"));
            }
            PStmt::If { cond, then, els } => {
                let c = self.expr(cond)?;
                self.line(&format!("if ({c}) {{"));
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
                self.block(&format!("while ({c})"), body)?;
            }
            PStmt::Loop { body } => self.block("while (true)", body)?,
            PStmt::Block(b) => self.block("", b)?,
            PStmt::ForRange { var, start, end, inclusive, body } => self.for_range(var, start, end, *inclusive, body)?,
            PStmt::ForIn { pat, iter, body } => self.for_in(pat, iter, body)?,
            PStmt::Match { scrutinee, arms } => self.match_stmt(scrutinee, arms)?,
            PStmt::Break => self.line("break;"),
            PStmt::Continue => self.line("continue;"),
            PStmt::Return(None) => self.line("return;"),
            PStmt::Return(Some(e)) => {
                let v = self.expr(e)?;
                self.line(&format!("return {v};"));
            }
            PStmt::Defer(_) => return self.unsupported("defer"),
        }
        Ok(())
    }

    fn let_pat(&mut self, pat: &PPat, init: &PExpr, els: Option<&[PStmt]>) -> PResult<()> {
        let v = self.expr(init)?;
        let t = self.fresh("p");
        self.line(&format!("const {t} = {v};"));
        let (mut conds, mut binds) = (Vec::new(), Vec::new());
        self.pat(pat, &t, &mut conds, &mut binds)?;
        if let (Some(els), false) = (els, conds.is_empty()) {
            self.block(&format!("if (!({}))", conds.join(" && ")), els)?;
        }
        for (n, e) in binds {
            self.line(&format!("let {n} = {e};"));
        }
        Ok(())
    }

    fn assign(&mut self, target: &PExpr, op: Option<BinKind>, value: &PExpr) -> PResult<()> {
        let v = self.expr(value)?;
        if let PK::Index(obj, key) = &*target.kind {
            if let Ty::Map(..) = obj.ty {
                let (m, k) = (self.expr(obj)?, self.expr(key)?);
                let new = match op {
                    Some(op) => self.binary_js(op, &target.ty, &format!("$mget({m}, {k})"), &v, value)?,
                    None => v,
                };
                self.line(&format!("{m}.set({k}, {new});"));
                return Ok(());
            }
        }
        let t = self.place(target)?;
        let rhs = match op {
            Some(op) => self.binary_js(op, &target.ty, &t, &v, value)?,
            None => v,
        };
        self.line(&format!("{t} = {rhs};"));
        Ok(())
    }

    fn for_range(&mut self, var: &str, start: &PExpr, end: &PExpr, inclusive: bool, body: &[PStmt]) -> PResult<()> {
        let (s, e) = (self.expr(start)?, self.expr(end)?);
        let v = ident(var);
        let end_name = self.fresh("end");
        self.line("{");
        self.ind += 1;
        self.line(&format!("const {end_name} = {e};"));
        let cmp = if inclusive { "<=" } else { "<" };
        let inc = match start.ty.int_kind() {
            Some(_) => format!("{v}++"),
            None => return self.unsupported("range over non-integer"),
        };
        self.block(&format!("for (let {v} = {s}; {v} {cmp} {end_name}; {inc})"), body)?;
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    fn for_in(&mut self, pat: &PPat, iter: &PExpr, body: &[PStmt]) -> PResult<()> {
        let it = self.expr(iter)?;
        let (mut conds, mut binds) = (Vec::new(), Vec::new());
        let item = self.fresh("it");
        self.pat(pat, &item, &mut conds, &mut binds)?;
        self.line(&format!("for (const {item} of {it}) {{"));
        self.ind += 1;
        for (n, e) in binds {
            self.line(&format!("let {n} = {e};"));
        }
        self.stmts(body)?;
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    fn match_stmt(&mut self, scrut: &PExpr, arms: &[PArm]) -> PResult<()> {
        let s = self.expr(scrut)?;
        let label = self.fresh("m");
        let sv = self.fresh("s");
        self.line(&format!("{label}: {{"));
        self.ind += 1;
        self.line(&format!("const {sv} = {s};"));
        for arm in arms {
            let (mut conds, mut binds) = (Vec::new(), Vec::new());
            self.pat(&arm.pat, &sv, &mut conds, &mut binds)?;
            let c = if conds.is_empty() { "true".to_string() } else { conds.join(" && ") };
            self.line(&format!("if ({c}) {{"));
            self.ind += 1;
            for (n, e) in binds {
                self.line(&format!("let {n} = {e};"));
            }
            self.arm_body(arm, &label)?;
            self.ind -= 1;
            self.line("}");
        }
        self.line("$panic(\"no match arm matched\");");
        self.ind -= 1;
        self.line("}");
        Ok(())
    }

    fn arm_body(&mut self, arm: &PArm, label: &str) -> PResult<()> {
        if let Some(g) = &arm.guard {
            let gv = self.expr(g)?;
            self.line(&format!("if ({gv}) {{"));
            self.ind += 1;
            self.stmts(&arm.body.stmts)?;
            self.line(&format!("break {label};"));
            self.ind -= 1;
            self.line("}");
        } else {
            self.stmts(&arm.body.stmts)?;
            self.line(&format!("break {label};"));
        }
        Ok(())
    }

    /// Typed binary operation on already-emitted operands (`b_expr` supplies operand types).
    pub fn binary_js(&mut self, op: BinKind, ty: &Ty, a: &str, b: &str, b_expr: &PExpr) -> PResult<String> {
        match ty {
            Ty::Int(k) => Ok(match op {
                BinKind::Shl | BinKind::Shr => num::shift(op == BinKind::Shl, *k, a, b, matches!(b_expr.ty, Ty::Int(bk) if bk.is_wide())),
                _ => num::int_binary(op, *k, a, b),
            }),
            Ty::F64 | Ty::F32 => {
                let sym = match op {
                    BinKind::Add => "+",
                    BinKind::Sub => "-",
                    BinKind::Mul => "*",
                    BinKind::Div => "/",
                    BinKind::Rem => "%",
                    _ => return self.unsupported("float bit operation"),
                };
                let r = format!("({a} {sym} {b})");
                Ok(if *ty == Ty::F32 { format!("Math.fround{r}") } else { r })
            }
            Ty::Str if op == BinKind::Add => Ok(format!("({a} + {b})")),
            other => self.unsupported(format!("compound operator on `{}`", other.describe())),
        }
    }
}

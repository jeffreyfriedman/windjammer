//! Expression lowering.

use super::{Lowerer, PResult};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{from_ast, IntKind, Ty};
use crate::parser::{BinaryOp, Expression, Literal, Statement, Type, UnaryOp};

fn bin_kind(op: &BinaryOp) -> BinKind {
    match op {
        BinaryOp::Add => BinKind::Add,
        BinaryOp::Sub => BinKind::Sub,
        BinaryOp::Mul => BinKind::Mul,
        BinaryOp::Div => BinKind::Div,
        BinaryOp::Mod => BinKind::Rem,
        BinaryOp::Eq => BinKind::Eq,
        BinaryOp::Ne => BinKind::Ne,
        BinaryOp::Lt => BinKind::Lt,
        BinaryOp::Le => BinKind::Le,
        BinaryOp::Gt => BinKind::Gt,
        BinaryOp::Ge => BinKind::Ge,
        BinaryOp::And => BinKind::And,
        BinaryOp::Or => BinKind::Or,
        BinaryOp::BitAnd => BinKind::BitAnd,
        BinaryOp::BitOr => BinKind::BitOr,
        BinaryOp::BitXor => BinKind::BitXor,
        BinaryOp::Shl => BinKind::Shl,
        BinaryOp::Shr => BinKind::Shr,
    }
}

/// Unsuffixed numeric literal (possibly negated): takes its type from context.
fn is_bare_literal(e: &Expression) -> bool {
    match e {
        Expression::Literal { value: Literal::Int(_) | Literal::Float(_), .. } => true,
        Expression::Unary { op: UnaryOp::Neg, operand, .. } => is_bare_literal(operand),
        _ => false,
    }
}

impl<'e> Lowerer<'e> {
    pub(super) fn expr(&mut self, e: &Expression, want: Option<&Ty>) -> PResult<PExpr> {
        if let Some(l) = e.location() {
            self.line = l.line as u32;
        }
        let line = self.line;
        let out = self.expr_inner(e, want)?;
        Ok(out.at(line))
    }

    fn expr_inner(&mut self, e: &Expression, want: Option<&Ty>) -> PResult<PExpr> {
        match e {
            Expression::Literal { value, .. } => self.literal(value, want),
            Expression::Identifier { name, .. } => self.identifier(name, want),
            Expression::Binary { left, op, right, .. } => self.binary(left, op, right, want),
            Expression::Unary { op, operand, .. } => self.unary(op, operand, want),
            Expression::Call { function, arguments, .. } => self.call(function, arguments, want),
            Expression::MethodCall { object, method, type_args, arguments, .. } => self.method_call(object, method, type_args.as_deref(), arguments, want),
            Expression::FieldAccess { object, field, .. } => self.field_access(object, field),
            Expression::StructLiteral { name, fields, .. } => self.struct_literal(name, fields, want),
            Expression::MapLiteral { pairs, .. } => self.map_literal(pairs, want),
            Expression::Range { start, end, inclusive, .. } => self.range(start, end, *inclusive, want),
            Expression::Closure { parameters, body, .. } => self.closure(parameters, body, want),
            Expression::Cast { expr, type_, .. } => {
                let inner = self.expr(expr, None)?;
                let ty = from_ast(type_, &self.generics);
                // Constant integer casts are folded with Rust `as` semantics (wrap to the target
                // width): Go rejects `uint8(-1)` as a constant overflow.
                if let (Some(v), Some(k)) = (const_int(&inner), ty.int_kind()) {
                    return Ok(PExpr::new(ty, PK::Int(wrap_int(v, k.bits(), k.signed()))));
                }
                Ok(PExpr::new(ty.clone(), PK::Cast(inner, ty)))
            }
            Expression::Index { object, index, .. } => self.index(object, index),
            Expression::Tuple { elements, .. } => self.tuple(elements, want),
            Expression::Array { elements, .. } => self.vec_literal(elements, want),
            Expression::MacroInvocation { name, args, is_repeat, .. } => self.macro_call(name, args, *is_repeat, want),
            Expression::TryOp { expr, .. } => self.try_op(expr),
            Expression::Block { statements, .. } => {
                let blk = self.block_value(statements, want)?;
                let ty = blk.tail.as_ref().map(|t| t.ty.clone()).unwrap_or(Ty::Unit);
                Ok(PExpr::new(ty, PK::Block(blk)))
            }
            Expression::Await { .. } | Expression::AsyncCall { .. } | Expression::SpawnCall { .. } => self.err("async/await/spawn"),
            Expression::ChannelSend { .. } | Expression::ChannelRecv { .. } => self.err("channels"),
        }
    }

    pub(super) fn literal(&mut self, v: &Literal, want: Option<&Ty>) -> PResult<PExpr> {
        let want_r = want.map(|w| self.resolve(w));
        let want = want_r.as_ref();
        Ok(match v {
            Literal::Int(n) => match want {
                Some(Ty::Int(k)) => PExpr::new(Ty::Int(*k), PK::Int(*n as i128)),
                Some(t @ (Ty::F32 | Ty::F64)) => PExpr::new(t.clone(), PK::Float(*n as f64)),
                Some(Ty::Var(i)) => {
                    if !self.float_vars.contains(i) {
                        self.int_vars.insert(*i);
                    }
                    PExpr::new(Ty::Var(*i), PK::Int(*n as i128))
                }
                _ => {
                    let t = self.lit_var(false);
                    PExpr::new(t, PK::Int(*n as i128))
                }
            },
            Literal::IntSuffixed(n, suffix) => match IntKind::from_name(suffix) {
                Some(k) => {
                    // u64/usize above i64::MAX are stored as their (negative) bit pattern.
                    let v = if matches!(suffix.as_str(), "u64" | "usize") { *n as u64 as i128 } else { *n as i128 };
                    PExpr::new(Ty::Int(k), PK::Int(v))
                }
                None if suffix == "f64" => PExpr::new(Ty::F64, PK::Float(*n as f64)),
                None if suffix == "f32" => PExpr::new(Ty::F32, PK::Float(*n as f64)),
                None => return self.err(format!("numeric literal suffix `{suffix}`")),
            },
            Literal::Float(f) => match want {
                Some(Ty::F32) => PExpr::new(Ty::F32, PK::Float(*f)),
                Some(Ty::F64) => PExpr::new(Ty::F64, PK::Float(*f)),
                Some(Ty::Var(i)) => {
                    self.float_vars.insert(*i);
                    PExpr::new(Ty::Var(*i), PK::Float(*f))
                }
                _ => {
                    let t = self.lit_var(true);
                    PExpr::new(t, PK::Float(*f))
                }
            },
            Literal::String(s) => PExpr::new(Ty::Str, PK::Str(s.clone())),
            Literal::Char(c) => PExpr::new(Ty::Char, PK::Char(*c)),
            Literal::Bool(b) => PExpr::new(Ty::Bool, PK::Bool(*b)),
        })
    }

    pub(super) fn local_expr(&mut self, name: &str) -> PResult<PExpr> {
        match self.lookup(name) {
            Some(l) => Ok(PExpr::new(l.ty.clone(), PK::Local(l.out.clone()))),
            None => self.err(format!("unknown variable `{name}`")),
        }
    }

    fn identifier(&mut self, name: &str, want: Option<&Ty>) -> PResult<PExpr> {
        if self.lookup(name).is_some() {
            return self.local_expr(name);
        }
        if name == "None" {
            let t = match want {
                Some(Ty::Option(t)) => (**t).clone(),
                _ => Ty::Unknown,
            };
            return Ok(PExpr::new(Ty::Option(Box::new(t)), PK::None));
        }
        if let Some(t) = self.env.consts.get(name) {
            return Ok(PExpr::new(t.clone(), PK::Local(name.to_string())));
        }
        if let Some((owner, member)) = name.rsplit_once("::") {
            return self.path_value(owner, member, want);
        }
        if let Some(sig) = self.env.fns.get(name) {
            let ty = Ty::Fn(sig.params.iter().map(|p| p.1.clone()).collect(), Box::new(sig.ret.clone()));
            return Ok(PExpr::new(ty, PK::FnRef(name.to_string())));
        }
        if let Some(en) = self.env.enum_of_variant(name) {
            let en = en.to_string();
            return self.path_value(&en, name, want);
        }
        self.err(format!("unknown identifier `{name}`"))
    }

    /// `Owner::member` used as a value: unit variant, associated const, or function reference.
    fn path_value(&mut self, owner: &str, member: &str, want: Option<&Ty>) -> PResult<PExpr> {
        let owner = if owner == "Self" { self.owner.clone().unwrap_or_default() } else { owner.to_string() };
        if let Some(v) = self.env.variant(&owner, member) {
            if v.fields.is_empty() {
                let args = self.enum_args_from_want(&owner, want);
                return Ok(PExpr::new(Ty::Adt(owner.clone(), args), PK::Variant { enum_name: owner, variant: member.to_string(), fields: vec![] }));
            }
        }
        if let Some(t) = crate::codegen::portable::builtins::static_ret(&owner, member, &[], want) {
            return Ok(PExpr::new(t, PK::StaticCall { owner, name: member.to_string(), args: vec![] }));
        }
        if let Some(sig) = self.env.method(&owner, member) {
            let ty = Ty::Fn(sig.params.iter().map(|p| p.1.clone()).collect(), Box::new(sig.ret.clone()));
            return Ok(PExpr::new(ty, PK::StaticCall { owner, name: member.to_string(), args: vec![] }));
        }
        self.err(format!("unknown path `{owner}::{member}`"))
    }

    pub(super) fn enum_args_from_want(&self, enum_name: &str, want: Option<&Ty>) -> Vec<Ty> {
        match want {
            Some(Ty::Adt(n, a)) if n == enum_name => a.clone(),
            _ => self.env.enums.get(enum_name).map(|e| vec![Ty::Unknown; e.type_params.len()]).unwrap_or_default(),
        }
    }

    /// Lower an operand pair so unsuffixed literals adopt the other side's type.
    pub(super) fn operand_pair(&mut self, l: &Expression, r: &Expression, want: Option<&Ty>) -> PResult<(PExpr, PExpr)> {
        if is_bare_literal(l) && want.is_none() && !is_bare_literal(r) {
            let rr = self.expr(r, None)?;
            let ll = self.expr(l, Some(&rr.ty))?;
            return Ok((ll, rr));
        }
        let ll = self.expr(l, want)?;
        let rr = self.expr(r, Some(&ll.ty))?;
        Ok((ll, rr))
    }

    fn binary(&mut self, l: &Expression, op: &BinaryOp, r: &Expression, want: Option<&Ty>) -> PResult<PExpr> {
        let kind = bin_kind(op);
        match kind {
            BinKind::And | BinKind::Or => {
                let (a, b) = (self.expr(l, Some(&Ty::Bool))?, self.expr(r, Some(&Ty::Bool))?);
                Ok(PExpr::new(Ty::Bool, PK::Binary(kind, a, b)))
            }
            BinKind::Eq | BinKind::Ne | BinKind::Lt | BinKind::Le | BinKind::Gt | BinKind::Ge => {
                let (a, b) = self.operand_pair(l, r, None)?;
                self.constrain(&a.ty, &b.ty);
                Ok(PExpr::new(Ty::Bool, PK::Binary(kind, a, b)))
            }
            BinKind::Shl | BinKind::Shr => {
                let a = self.expr(l, want)?;
                let b = self.expr(r, None)?;
                Ok(PExpr::new(a.ty.clone(), PK::Binary(kind, a, b)))
            }
            _ => {
                let (a, b) = self.operand_pair(l, r, want)?;
                self.constrain(&a.ty, &b.ty);
                let ty = a.ty.join(&b.ty);
                Ok(PExpr::new(ty, PK::Binary(kind, a, b)))
            }
        }
    }

    fn unary(&mut self, op: &UnaryOp, operand: &Expression, want: Option<&Ty>) -> PResult<PExpr> {
        match op {
            UnaryOp::Ref | UnaryOp::MutRef | UnaryOp::Deref => self.expr(operand, want),
            UnaryOp::Neg => {
                let a = self.expr(operand, want)?;
                match &*a.kind {
                    PK::Int(v) => return Ok(PExpr::new(a.ty.clone(), PK::Int(-*v))),
                    PK::Float(f) => return Ok(PExpr::new(a.ty.clone(), PK::Float(-*f))),
                    _ => {}
                }
                Ok(PExpr::new(a.ty.clone(), PK::Unary(UnKind::Neg, a)))
            }
            UnaryOp::Not => {
                let a = self.expr(operand, want)?;
                Ok(PExpr::new(a.ty.clone(), PK::Unary(UnKind::Not, a)))
            }
        }
    }

    fn field_access(&mut self, object: &Expression, field: &str) -> PResult<PExpr> {
        let obj = self.expr(object, None)?;
        let oty = self.resolve_for_dispatch(&obj.ty);
        match &oty {
            Ty::Tuple(ts) => {
                let idx: usize = field.parse().map_err(|_| self.unsupported(format!("tuple field `{field}`")))?;
                let ty = ts.get(idx).cloned().unwrap_or(Ty::Unknown);
                Ok(PExpr::new(ty, PK::TupleIdx(obj, idx)))
            }
            Ty::Adt(name, args) => {
                let Some(info) = self.env.structs.get(name) else { return self.err(format!("field `{field}` on non-struct `{name}`")) };
                let Some((_, fty)) = info.fields.iter().find(|(n, _)| n == field) else { return self.err(format!("no field `{field}` on `{name}`")) };
                let ty = fty.subst(&info.type_params, args);
                Ok(PExpr::new(ty, PK::Field(obj, field.to_string())))
            }
            other => self.err(format!("field `{field}` on type `{}` (type not inferred)", other.describe())),
        }
    }

    pub(super) fn unsupported(&self, what: String) -> Unsupported {
        Unsupported { what, line: self.line }
    }

    pub(super) fn ensure_usize(&mut self, e: PExpr) -> PExpr {
        self.constrain(&Ty::usize(), &e.ty);
        match self.resolve(&e.ty) {
            Ty::Int(IntKind::Usize) => e,
            Ty::Int(_) => PExpr::new(Ty::usize(), PK::Cast(e, Ty::usize())),
            _ => e,
        }
    }

    fn index(&mut self, object: &Expression, index: &Expression) -> PResult<PExpr> {
        let obj = self.expr(object, None)?;
        let oty = self.resolve_for_dispatch(&obj.ty);
        let obj = PExpr { ty: oty, ..obj };
        if let Expression::Range { .. } = index {
            let idx = self.expr(index, Some(&Ty::Range(Box::new(Ty::usize()))))?;
            let ty = if obj.ty == Ty::Str { Ty::Str } else { obj.ty.clone() };
            return Ok(PExpr::new(ty, PK::Index(obj, idx)));
        }
        match &obj.ty {
            Ty::Vec(t) => {
                let ty = (**t).clone();
                let idx = self.expr(index, Some(&Ty::usize()))?;
                Ok(PExpr::new(ty, PK::Index(obj, self.ensure_usize(idx))))
            }
            Ty::Map(k, v) => {
                let (key_ty, val_ty) = ((**k).clone(), (**v).clone());
                let idx = self.expr(index, Some(&key_ty))?;
                Ok(PExpr::new(val_ty, PK::Index(obj, idx)))
            }
            other => self.err(format!("indexing type `{}`", other.describe())),
        }
    }

    fn range(&mut self, s: &Expression, e: &Expression, inclusive: bool, want: Option<&Ty>) -> PResult<PExpr> {
        let w = match want {
            Some(Ty::Range(t)) => Some((**t).clone()),
            _ => None,
        };
        let (a, b) = self.operand_pair(s, e, w.as_ref())?;
        let t = a.ty.join(&b.ty);
        Ok(PExpr::new(Ty::Range(Box::new(t)), PK::Range { start: a, end: b, inclusive }))
    }

    fn tuple(&mut self, elems: &[&Expression], want: Option<&Ty>) -> PResult<PExpr> {
        if elems.is_empty() {
            return Ok(PExpr::unit());
        }
        let mut out = Vec::new();
        for (i, e) in elems.iter().enumerate() {
            let w = match want {
                Some(Ty::Tuple(ts)) => ts.get(i),
                _ => None,
            };
            let v = self.expr(e, w)?;
            out.push(self.own(v));
        }
        let ty = Ty::Tuple(out.iter().map(|e| e.ty.clone()).collect());
        Ok(PExpr::new(ty, PK::Tuple(out)))
    }

    pub(super) fn vec_literal(&mut self, elems: &[&Expression], want: Option<&Ty>) -> PResult<PExpr> {
        let mut elem_want = match want {
            Some(Ty::Vec(t)) => Some((**t).clone()),
            _ => None,
        };
        let mut out = Vec::new();
        for e in elems {
            let v = self.expr(e, elem_want.as_ref())?;
            if elem_want.as_ref().map(|t| t.is_unknown()).unwrap_or(true) {
                elem_want = Some(v.ty.clone());
            }
            out.push(self.own(v));
        }
        let t = elem_want.unwrap_or(Ty::Unknown);
        Ok(PExpr::new(Ty::Vec(Box::new(t)), PK::VecLit(out)))
    }

    fn map_literal(&mut self, pairs: &[(&Expression, &Expression)], want: Option<&Ty>) -> PResult<PExpr> {
        let (kw, vw) = match want {
            Some(Ty::Map(k, v)) => (Some((**k).clone()), Some((**v).clone())),
            _ => (None, None),
        };
        let mut out = Vec::new();
        for (k, v) in pairs {
            let (k, v) = (self.expr(k, kw.as_ref())?, self.expr(v, vw.as_ref())?);
            out.push((k, self.own(v)));
        }
        let ty = match out.first() {
            Some((k, v)) => Ty::Map(Box::new(k.ty.clone()), Box::new(v.ty.clone())),
            None => want.cloned().unwrap_or(Ty::Map(Box::new(Ty::Unknown), Box::new(Ty::Unknown))),
        };
        Ok(PExpr::new(ty, PK::MapLit(out)))
    }

    fn struct_literal(&mut self, name: &str, fields: &[(String, &Expression)], want: Option<&Ty>) -> PResult<PExpr> {
        let name = if name == "Self" { self.owner.clone().unwrap_or_default() } else { name.to_string() };
        if let Some((en, var)) = name.split_once("::") {
            return self.variant_struct_literal(en, var, fields, want);
        }
        let Some(info) = self.env.structs.get(&name).cloned() else { return self.err(format!("unknown struct `{name}`")) };
        let mut bound: Vec<Ty> = match want {
            Some(Ty::Adt(n, a)) if *n == name => a.clone(),
            _ => vec![Ty::Unknown; info.type_params.len()],
        };
        let mut out = Vec::new();
        for (fname, fexpr) in fields {
            let Some((_, fty)) = info.fields.iter().find(|(n, _)| n == fname) else { return self.err(format!("no field `{fname}` on `{name}`")) };
            let expect = fty.subst(&info.type_params, &bound);
            let v = self.expr(fexpr, Some(&expect))?;
            super::call::unify(fty, &v.ty, &info.type_params, &mut bound);
            self.constrain(&expect, &v.ty);
            out.push((fname.clone(), self.own(v)));
        }
        Ok(PExpr::new(Ty::Adt(name.clone(), bound), PK::Struct { name, fields: out }))
    }

    fn variant_struct_literal(&mut self, en: &str, var: &str, fields: &[(String, &Expression)], want: Option<&Ty>) -> PResult<PExpr> {
        let en = if en == "Self" { self.owner.clone().unwrap_or_default() } else { en.to_string() };
        let Some(info) = self.env.variant(&en, var).cloned() else { return self.err(format!("unknown variant `{en}::{var}`")) };
        let einfo = self.env.enums.get(&en).cloned().unwrap();
        let mut bound = self.enum_args_from_want(&en, want);
        let mut out = Vec::new();
        for (fname, fexpr) in fields {
            let Some((_, fty)) = info.fields.iter().find(|(n, _)| n == fname) else { return self.err(format!("no field `{fname}` on `{en}::{var}`")) };
            let v = self.expr(fexpr, Some(&fty.subst(&einfo.type_params, &bound)))?;
            super::call::unify(fty, &v.ty, &einfo.type_params, &mut bound);
            out.push((fname.clone(), self.own(v)));
        }
        Ok(PExpr::new(Ty::Adt(en.clone(), bound), PK::Variant { enum_name: en, variant: var.to_string(), fields: out }))
    }

    fn closure(&mut self, params: &[String], body: &Expression, want: Option<&Ty>) -> PResult<PExpr> {
        let (ptys, rwant) = match want {
            Some(Ty::Fn(p, r)) => (p.clone(), (**r).clone()),
            _ => (vec![], Ty::Unknown),
        };
        self.push();
        let mut ps = Vec::new();
        let mut prologue = Vec::new();
        for (i, raw) in params.iter().enumerate() {
            let hint = ptys.get(i).filter(|t| !matches!(t, Ty::Unknown)).cloned();
            if raw.starts_with('(') {
                let (param, stmt) = self.tuple_param(raw, hint)?;
                ps.push(param);
                prologue.push(stmt);
                continue;
            }
            let (name, declared) = match raw.split_once(':') {
                Some((n, t)) => (n.trim().to_string(), Some(from_ast(&type_from_str(t), &self.generics))),
                None => (raw.trim().to_string(), None),
            };
            let t = declared.or(hint).unwrap_or_else(|| self.new_var());
            let out = self.declare(&name, t.clone(), false);
            ps.push((out, t));
        }
        self.rets.push(rwant.clone());
        let blk = match body {
            Expression::Block { statements, .. } => self.block_value(statements, Some(&rwant).filter(|t| !t.is_unknown())),
            other => self.expr(other, Some(&rwant).filter(|t| !t.is_unknown())).map(|e| PBlock { stmts: vec![], tail: Some(e) }),
        };
        self.rets.pop();
        self.pop();
        let mut blk = blk?;
        prologue.extend(std::mem::take(&mut blk.stmts));
        blk.stmts = prologue;
        let ret = blk.tail.as_ref().map(|t| t.ty.clone()).unwrap_or(Ty::Unit);
        let ty = Ty::Fn(ps.iter().map(|p| p.1.clone()).collect(), Box::new(ret.clone()));
        let captures = self.closure_captures(&mut blk);
        Ok(PExpr::new(ty, PK::Closure { params: ps, ret, body: blk, captures }))
    }

    /// `|(a, b)|`: a hidden tuple parameter plus a destructuring `let` at the top of the body.
    fn tuple_param(&mut self, raw: &str, hint: Option<Ty>) -> PResult<((String, Ty), PStmt)> {
        let names: Vec<String> = raw.trim_matches(|c| c == '(' || c == ')').split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let elem_tys: Vec<Ty> = match &hint {
            Some(Ty::Tuple(ts)) if ts.len() == names.len() => ts.clone(),
            _ => names.iter().map(|_| self.new_var()).collect(),
        };
        let tup_ty = Ty::Tuple(elem_tys.clone());
        let nm = self.fresh("tup");
        let hidden = self.declare(&nm, tup_ty.clone(), false);
        let mut binds = Vec::new();
        for (n, t) in names.iter().zip(&elem_tys) {
            let out = self.declare(n, t.clone(), false);
            binds.push(PPat::Bind { name: out, mutable: false });
        }
        let init = PExpr::new(tup_ty.clone(), PK::Local(hidden.clone()));
        Ok(((hidden, tup_ty), PStmt::LetPat { pat: PPat::Tuple(binds), init, els: None }))
    }

    /// Outer `mut` locals referenced by the closure body: Rust `move` closures own a snapshot.
    fn closure_captures(&self, body: &mut PBlock) -> Vec<(String, Ty)> {
        let mut seen = std::collections::BTreeSet::new();
        crate::codegen::portable::visit::exprs_in_block(body, &mut |e| {
            if let PK::Local(n) = &*e.kind {
                seen.insert(n.clone());
            }
        });
        let mut out = Vec::new();
        for name in seen {
            let hit = self.scopes.iter().flat_map(|s| s.values()).find(|l| l.out == name && l.mutable && !l.alias);
            if let Some(l) = hit {
                out.push((name, l.ty.clone()));
            }
        }
        out
    }

    fn try_op(&mut self, inner: &Expression) -> PResult<PExpr> {
        let e = self.expr(inner, None)?;
        let fn_ret = self.rets.last().cloned().unwrap_or(Ty::Unknown);
        let ty = match &e.ty {
            Ty::Option(t) => (**t).clone(),
            Ty::Result(t, _) => (**t).clone(),
            other => return self.err(format!("`?` applied to `{}`", other.describe())),
        };
        Ok(PExpr::new(ty, PK::Try { expr: e, fn_ret }))
    }

    /// Lower a statement list as a value-producing block (`let x = { ... }`, arm bodies, closures).
    pub(super) fn block_value(&mut self, stmts: &[&Statement], want: Option<&Ty>) -> PResult<PBlock> {
        self.push();
        let r = self.stmts_with_tail(stmts, want, true);
        self.pop();
        r
    }
}

/// Rebuild a parser `Type` from the textual form the closure-parameter parser keeps (`i32`, `Vec<int>`, `(a, b)`).
fn type_from_str(s: &str) -> Type {
    let s = s.trim().trim_start_matches('&').trim_start_matches("mut ").trim();
    if let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        let parts: Vec<Type> = split_top(inner).into_iter().map(type_from_str).collect();
        return Type::Tuple(parts);
    }
    if let (Some(lt), true) = (s.find('<'), s.ends_with('>')) {
        let args: Vec<Type> = split_top(&s[lt + 1..s.len() - 1]).into_iter().map(type_from_str).collect();
        return Type::Parameterized(s[..lt].trim().to_string(), args);
    }
    match s {
        "int" => Type::Int,
        "i32" => Type::Int32,
        "uint" => Type::Uint,
        "float" => Type::Float,
        "bool" => Type::Bool,
        "string" => Type::String,
        other => Type::Custom(other.to_string()),
    }
}

fn split_top(s: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = s[start..].trim();
    if !last.is_empty() {
        out.push(last);
    }
    out
}

/// Value of an integer literal, possibly negated (`7`, `-1`).
fn const_int(e: &PExpr) -> Option<i128> {
    match &*e.kind {
        PK::Int(v) => Some(*v),
        PK::Unary(UnKind::Neg, inner) => const_int(inner).map(|v| -v),
        _ => None,
    }
}

/// Two's-complement wrap of `v` to `bits`, read back signed or unsigned (Rust `as`).
fn wrap_int(v: i128, bits: u32, signed: bool) -> i128 {
    if bits >= 128 {
        return v;
    }
    let m = v & ((1i128 << bits) - 1);
    if signed && m >= 1i128 << (bits - 1) {
        m - (1i128 << bits)
    } else {
        m
    }
}

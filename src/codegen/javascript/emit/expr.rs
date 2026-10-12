//! Expression emission (expressions are pure trees after flattening).

use super::names::{ident, js_string, prop, variant_class};
use super::num;
use super::Js;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{IntKind, Ty};

fn is_simple(e: &PExpr) -> bool {
    match &*e.kind {
        PK::Local(_) => true,
        PK::Field(a, _) | PK::TupleIdx(a, _) => is_simple(a),
        PK::Index(a, i) => is_simple(a) && is_simple(i),
        PK::Int(_) | PK::Str(_) => true,
        PK::Copy(a) => is_simple(a),
        _ => false,
    }
}

impl<'m> Js<'m> {
    pub fn args(&mut self, es: &[PExpr]) -> PResult<String> {
        let v: PResult<Vec<String>> = es.iter().map(|e| self.expr(e)).collect();
        Ok(v?.join(", "))
    }

    /// Emit a place for assignment or compound update.
    pub fn place(&mut self, e: &PExpr) -> PResult<String> {
        match &*e.kind {
            PK::Index(obj, idx) if matches!(obj.ty, Ty::Vec(_)) => {
                let (o, i) = (self.place_or_expr(obj)?, self.expr(idx)?);
                Ok(format!("{o}[$ix({o}, {i})]"))
            }
            PK::Field(..) | PK::TupleIdx(..) | PK::Local(_) => self.expr(e),
            PK::Copy(inner) => self.place(inner),
            _ => self.expr(e),
        }
    }

    fn place_or_expr(&mut self, e: &PExpr) -> PResult<String> {
        if matches!(&*e.kind, PK::Index(..)) { self.place(e) } else { self.expr(e) }
    }

    pub fn expr(&mut self, e: &PExpr) -> PResult<String> {
        match &*e.kind {
            PK::Int(v) => Ok(match &e.ty {
                Ty::Int(k) => num::lit_int(*v, *k),
                Ty::F64 | Ty::F32 => num::float_lit(*v as f64, e.ty == Ty::F32),
                _ => num::lit_int(*v, IntKind::I64),
            }),
            PK::Float(f) => Ok(num::float_lit(*f, e.ty == Ty::F32)),
            PK::Bool(b) => Ok(b.to_string()),
            PK::Str(s) => Ok(js_string(s)),
            PK::Char(c) => Ok(js_string(&c.to_string())),
            PK::Unit => Ok("undefined".into()),
            PK::Local(n) if self.cells.contains(n) => Ok(format!("{}.v", ident(n))),
            PK::Local(n) => Ok(if n == "self" { "this".into() } else { ident(n) }),
            PK::FnRef(n) => Ok(ident(n)),
            PK::Unary(op, a) => self.unary(*op, a),
            PK::Binary(op, a, b) => self.binary(*op, a, b),
            PK::Cast(a, to) => {
                let v = self.expr(a)?;
                Ok(num::cast(&a.ty, to, &v))
            }
            PK::Field(a, f) => Ok(format!("{}.{}", self.expr_paren(a)?, prop(f))),
            PK::TupleIdx(a, i) => Ok(format!("{}[{i}]", self.expr_paren(a)?)),
            PK::Index(a, i) => self.index(a, i),
            PK::Call { name, args } => {
                let f = self.find_fn(None, name);
                Ok(format!("{}({})", ident(name), self.args_for(f, args)?))
            }
            PK::StdCall { module, name, args } => Ok(format!("$std_{module}_{name}({})", self.args(args)?)),
            PK::StaticCall { owner, name, args } => self.static_call(owner, name, args, &e.ty),
            PK::CallValue { f, args } => Ok(format!("{}({})", self.expr_paren(f)?, self.args(args)?)),
            PK::Method { recv, name, args, res } => self.method(recv, name, args, res, &e.ty),
            PK::Struct { name, fields } => self.struct_lit(name, fields),
            PK::Variant { enum_name, variant, fields } => {
                let a: PResult<Vec<String>> = fields.iter().map(|(_, e)| self.expr(e)).collect();
                Ok(format!("new {}({})", variant_class(enum_name, variant), a?.join(", ")))
            }
            PK::Some(a) => Ok(format!("{{ v: {} }}", self.expr(a)?)),
            PK::None => Ok("null".into()),
            PK::Ok(a) => Ok(format!("{{ ok: true, v: {} }}", self.expr(a)?)),
            PK::Err(a) => Ok(format!("{{ ok: false, v: {} }}", self.expr(a)?)),
            PK::VecLit(es) => Ok(format!("[{}]", self.args(es)?)),
            PK::VecRepeat(x, n) => {
                let (xv, nv) = (self.expr(x)?, self.expr(n)?);
                let c = self.cloner(&x.ty).unwrap_or_else(|| "null".into());
                Ok(format!("$rep({xv}, {nv}, {c})"))
            }
            PK::MapLit(ps) => {
                let parts: PResult<Vec<String>> = ps.iter().map(|(k, v)| Ok(format!("[{}, {}]", self.expr(k)?, self.expr(v)?))).collect();
                Ok(format!("new Map([{}])", parts?.join(", ")))
            }
            PK::Tuple(es) => Ok(format!("[{}]", self.args(es)?)),
            PK::Closure { params, body, captures, .. } => self.closure(params, body, captures),
            PK::Copy(a) => {
                let v = self.expr(a)?;
                Ok(self.clone_js(&a.ty, &v))
            }
            PK::BorrowMut(_) => self.unsupported("`&mut` outside a `let` initializer"),
            PK::Format(pieces) => self.format(pieces),
            PK::Print { newline, to_stderr, pieces } => {
                let s = self.format(pieces)?;
                let s = if *newline { format!("{s} + \"\\n\"") } else { s };
                Ok(format!("{}({s})", if *to_stderr { "$eprint" } else { "$print" }))
            }
            PK::Range { start, end, inclusive } => {
                let (a, b) = (self.expr(start)?, self.expr(end)?);
                Ok(format!("$range({a}, {b}, {inclusive})"))
            }
            PK::Panic(m) => Ok(format!("$panic({})", self.expr(m)?)),
            PK::If { .. } | PK::Match { .. } | PK::Block(_) | PK::Try { .. } => self.unsupported("unflattened control flow in expression"),
        }
    }

    fn expr_paren(&mut self, e: &PExpr) -> PResult<String> {
        let s = self.expr(e)?;
        Ok(if matches!(&*e.kind, PK::Local(_) | PK::Field(..) | PK::TupleIdx(..) | PK::Call { .. } | PK::Method { .. } | PK::Index(..)) { s } else { format!("({s})") })
    }

    fn unary(&mut self, op: UnKind, a: &PExpr) -> PResult<String> {
        let v = self.expr(a)?;
        Ok(match (op, &a.ty) {
            (UnKind::Neg, Ty::Int(k)) => format!("{}(-{v})", num::chk(*k)),
            (UnKind::Neg, _) => format!("(-{v})"),
            (UnKind::Not, Ty::Bool) => format!("(!{v})"),
            (UnKind::Not, Ty::Int(k)) => num::wrap_expr(*k, &format!("~{v}")),
            (UnKind::Not, _) => format!("(!{v})"),
        })
    }

    fn binary(&mut self, op: BinKind, a: &PExpr, b: &PExpr) -> PResult<String> {
        let (x, y) = (self.expr(a)?, self.expr(b)?);
        match op {
            BinKind::And => return Ok(format!("({x} && {y})")),
            BinKind::Or => return Ok(format!("({x} || {y})")),
            _ => {}
        }
        let aggregate = !self.is_prim(&a.ty);
        match op {
            BinKind::Eq | BinKind::Ne if aggregate => {
                let eq = format!("$eq({x}, {y})");
                Ok(if op == BinKind::Eq { eq } else { format!("(!{eq})") })
            }
            BinKind::Lt | BinKind::Le | BinKind::Gt | BinKind::Ge if aggregate => {
                let sym = match op {
                    BinKind::Lt => "<",
                    BinKind::Le => "<=",
                    BinKind::Gt => ">",
                    _ => ">=",
                };
                Ok(format!("($cmp({x}, {y}) {sym} 0)"))
            }
            BinKind::Eq => Ok(format!("({x} === {y})")),
            BinKind::Ne => Ok(format!("({x} !== {y})")),
            BinKind::Lt | BinKind::Le | BinKind::Gt | BinKind::Ge => {
                let sym = match op {
                    BinKind::Lt => "<",
                    BinKind::Le => "<=",
                    BinKind::Gt => ">",
                    _ => ">=",
                };
                Ok(format!("({x} {sym} {y})"))
            }
            _ => self.binary_js(op, &a.ty, &x, &y, b),
        }
    }

    fn index(&mut self, a: &PExpr, i: &PExpr) -> PResult<String> {
        let o = self.expr(a)?;
        if let PK::Range { start, end, inclusive } = &*i.kind {
            let (s, e) = (self.expr(start)?, self.expr(end)?);
            return Ok(match a.ty {
                Ty::Str => format!("$strSlice({o}, {s}, {e}, {inclusive})"),
                _ => format!("$vecSlice({o}, {s}, {e}, {inclusive})"),
            });
        }
        let idx = self.expr(i)?;
        match &a.ty {
            Ty::Map(..) => Ok(format!("$mget({o}, {idx})")),
            _ if is_simple(a) => Ok(format!("{o}[$ix({o}, {idx})]")),
            _ => Ok(format!("$at({o}, {idx})")),
        }
    }

    fn struct_lit(&mut self, name: &str, fields: &[(String, PExpr)]) -> PResult<String> {
        let Some(decl) = self.m.structs.iter().find(|s| s.name == name) else { return self.unsupported(format!("struct `{name}`")) };
        let mut vals = Vec::new();
        for (fname, _) in &decl.fields {
            let Some((_, e)) = fields.iter().find(|(n, _)| n == fname) else { return self.unsupported(format!("missing field `{fname}` in `{name}` literal")) };
            vals.push(self.expr(e)?);
        }
        Ok(format!("new {}({})", ident(name), vals.join(", ")))
    }

    fn closure(&mut self, params: &[(String, Ty)], body: &PBlock, captures: &[(String, Ty)]) -> PResult<String> {
        let arrow = self.closure_arrow(params, body)?;
        if captures.is_empty() {
            return Ok(arrow);
        }
        let names: Vec<String> = captures.iter().map(|(n, _)| ident(n)).collect();
        let vals: Vec<String> = captures.iter().map(|(n, t)| self.clone_js(t, &ident(n))).collect();
        Ok(format!("(({}) => {arrow})({})", names.join(", "), vals.join(", ")))
    }

    fn closure_arrow(&mut self, params: &[(String, Ty)], body: &PBlock) -> PResult<String> {
        let ps: Vec<String> = params.iter().map(|(n, t)| if self.ts { format!("{}: {}", ident(n), self.ts_type(t)) } else { ident(n) }).collect();
        let saved = std::mem::take(&mut self.out);
        let saved_ind = self.ind;
        self.ind = 1;
        self.stmts(&body.stmts)?;
        let inner = std::mem::replace(&mut self.out, saved);
        self.ind = saved_ind;
        Ok(format!("(({}) => {{\n{}{}}})", ps.join(", "), inner, "  ".repeat(self.ind)))
    }

    fn static_call(&mut self, owner: &str, name: &str, args: &[PExpr], ty: &Ty) -> PResult<String> {
        let user = self.m.structs.iter().any(|s| s.name == owner) || self.m.enums.iter().any(|e| e.name == owner) || self.m.traits.iter().any(|t| t.name == owner);
        let a = if user { self.args_for(self.find_fn(Some(owner), name), args)? } else { self.args(args)? };
        if user {
            return Ok(format!("{}.{}({a})", ident(owner), self.method_name(name)));
        }
        self.builtin_static(owner, name, &a, args, ty)
    }

    pub fn method_name(&self, name: &str) -> String {
        let n = ident(name);
        if self.fields.contains(name) { format!("{n}$m") } else { n }
    }
}

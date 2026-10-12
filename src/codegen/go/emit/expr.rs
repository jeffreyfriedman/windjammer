//! Expression emission for Go.

use super::names::{field, go_string, ident, variant_type};
use super::num;
use super::Go;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::Ty;

impl<'m> Go<'m> {
    pub fn args_for(&mut self, f: Option<&PFn>, args: &[PExpr]) -> PResult<String> {
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let mode = f.and_then(|f| f.params.get(i)).map(|p| p.mode).unwrap_or(ParamMode::Value);
            out.push(if mode == ParamMode::MutRef { self.addr(a)? } else { self.expr(a)? });
        }
        Ok(out.join(", "))
    }

    pub fn plain_args(&mut self, args: &[PExpr]) -> PResult<String> {
        let v: PResult<Vec<String>> = args.iter().map(|e| self.expr(e)).collect();
        Ok(v?.join(", "))
    }

    /// Address of a place (for `&mut` parameters and mutating built-ins).
    pub fn addr(&mut self, e: &PExpr) -> PResult<String> {
        if self.is_ref_type(&e.ty) {
            return self.expr(e);
        }
        match &*e.kind {
            PK::Local(n) if self.ptr_locals.contains_key(n) => Ok(ident(n)),
            PK::Local(n) => Ok(format!("&{}", ident(n))),
            PK::Field(..) | PK::TupleIdx(..) | PK::Index(..) => Ok(format!("&{}", self.expr(e)?)),
            PK::Copy(inner) => self.addr(inner),
            _ => Ok(format!("wjPtr({})", self.expr(e)?)),
        }
    }

    /// Receiver/base position: pointer-to-struct locals auto-dereference in Go.
    pub fn base(&mut self, e: &PExpr) -> PResult<String> {
        if let PK::Local(n) = &*e.kind {
            if let Some(Ty::Adt(..)) = self.ptr_locals.get(n) {
                return Ok(ident(n));
            }
        }
        self.expr(e)
    }

    /// Assignable form of a place.
    pub fn place(&mut self, e: &PExpr) -> PResult<String> {
        match &*e.kind {
            PK::Local(n) if self.ptr_locals.contains_key(n) => Ok(format!("*{}", ident(n))),
            PK::Copy(inner) => self.place(inner),
            PK::Index(a, i) if matches!(a.ty, Ty::Map(..)) => Ok(format!("{}[{}]", self.expr(a)?, self.expr(i)?)),
            _ => self.expr(e),
        }
    }

    pub fn expr(&mut self, e: &PExpr) -> PResult<String> {
        match &*e.kind {
            PK::Int(v) => Ok(match &e.ty {
                Ty::Int(k) => num::lit_int(*v, *k),
                Ty::F64 | Ty::F32 => num::float_lit(*v as f64, e.ty == Ty::F32),
                _ => num::lit_int(*v, crate::codegen::portable::ty::IntKind::I64),
            }),
            PK::Float(f) => Ok(num::float_lit(*f, e.ty == Ty::F32)),
            PK::Bool(b) => Ok(b.to_string()),
            PK::Str(s) => Ok(go_string(s)),
            PK::Char(c) => Ok(format!("Char({})", *c as u32)),
            PK::Unit => Ok("struct{}{}".into()),
            PK::Local(n) => {
                let id = ident(n);
                Ok(if self.ptr_locals.contains_key(n) { format!("(*{id})") } else { id })
            }
            PK::FnRef(n) => Ok(ident(n)),
            PK::Unary(op, a) => self.unary(*op, a),
            PK::Binary(op, a, b) => self.binary(*op, a, b),
            PK::Cast(a, to) => {
                let v = self.expr(a)?;
                Ok(num::cast(&a.ty, to, &v))
            }
            PK::Field(a, f) => Ok(format!("{}.{}", self.base(a)?, field(f))),
            PK::TupleIdx(a, i) => Ok(format!("{}.F{i}", self.base(a)?)),
            PK::Index(a, i) => self.index(a, i),
            PK::Call { name, args } => {
                let f = self.find_fn(None, name);
                Ok(format!("{}({})", ident(name), self.args_for(f, args)?))
            }
            PK::StdCall { module, name, args } => Ok(format!("wjstd_{module}_{name}({})", self.plain_args(args)?)),
            PK::StaticCall { owner, name, args } => self.static_call(owner, name, args, &e.ty),
            PK::CallValue { f, args } => {
                let fv = self.expr(f)?;
                Ok(format!("{fv}({})", self.plain_args(args)?))
            }
            PK::Method { recv, name, args, res } => self.method(recv, name, args, res, &e.ty),
            PK::Struct { name, fields } => self.struct_lit(name, fields, &e.ty),
            PK::Variant { enum_name, variant, fields } => {
                let targs = self.targs(&e.ty)?;
                let vals: PResult<Vec<String>> = fields.iter().map(|(n, v)| Ok(format!("{}: {}", field(n), self.expr(v)?))).collect();
                Ok(format!("{}{targs}{{{}}}", variant_type(enum_name, variant), vals?.join(", ")))
            }
            PK::Some(a) => Ok(format!("{}{{Ok: true, V: {}}}", self.ty(&e.ty)?, self.expr(a)?)),
            PK::None => Ok(format!("{}{{}}", self.ty(&e.ty)?)),
            PK::Ok(a) => Ok(format!("{}{{Ok: true, V: {}}}", self.ty(&e.ty)?, self.expr(a)?)),
            PK::Err(a) => Ok(format!("{}{{E: {}}}", self.ty(&e.ty)?, self.expr(a)?)),
            PK::VecLit(es) => Ok(format!("{}{{{}}}", self.ty(&e.ty)?, self.plain_args(es)?)),
            PK::VecRepeat(x, n) => {
                let (xv, nv) = (self.expr(x)?, self.expr(n)?);
                Ok(if self.needs_deep_clone(&x.ty) { format!("wjRepeatClone({xv}, {nv})") } else { format!("wjRepeat({xv}, {nv})") })
            }
            PK::MapLit(ps) => {
                let parts: PResult<Vec<String>> = ps.iter().map(|(k, v)| Ok(format!("{}: {}", self.expr(k)?, self.expr(v)?))).collect();
                Ok(format!("{}{{{}}}", self.ty(&e.ty)?, parts?.join(", ")))
            }
            PK::Tuple(es) => {
                let parts: PResult<Vec<String>> = es.iter().enumerate().map(|(i, x)| Ok(format!("F{i}: {}", self.expr(x)?))).collect();
                Ok(format!("{}{{{}}}", self.ty(&e.ty)?, parts?.join(", ")))
            }
            PK::Closure { params, ret, body, captures } => self.closure_snapshot(params, ret, body, captures, &e.ty),
            PK::Copy(a) => {
                let v = self.expr(a)?;
                self.clone_go(&a.ty, &v)
            }
            PK::Format(pieces) => self.format(pieces),
            PK::Print { newline, to_stderr, pieces } => {
                let s = self.format(pieces)?;
                let s = if *newline { format!("{s} + \"\\n\"") } else { s };
                Ok(format!("{}({s})", if *to_stderr { "wjEprint" } else { "wjPrint" }))
            }
            PK::Range { start, end, inclusive } => Ok(format!("wjRange({}, {}, {inclusive})", self.expr(start)?, self.expr(end)?)),
            PK::Panic(m) => Ok(format!("wjPanic({})", self.expr(m)?)),
            PK::If { .. } | PK::Match { .. } | PK::Block(_) | PK::Try { .. } => self.unsupported("unflattened control flow in expression"),
        }
    }

    /// `[T, U]` type-argument list of an ADT type (empty when non-generic).
    pub fn targs(&self, t: &Ty) -> PResult<String> {
        match t {
            Ty::Adt(_, args) if !args.is_empty() => {
                let v: PResult<Vec<String>> = args.iter().map(|a| self.ty(a)).collect();
                Ok(format!("[{}]", v?.join(", ")))
            }
            _ => Ok(String::new()),
        }
    }

    fn unary(&mut self, op: UnKind, a: &PExpr) -> PResult<String> {
        let v = self.expr(a)?;
        Ok(match (op, &a.ty) {
            (UnKind::Neg, Ty::Int(_)) => format!("wjNeg({v})"),
            (UnKind::Neg, _) => format!("wjFneg({v})"),
            (UnKind::Not, Ty::Bool) => format!("(!{v})"),
            (UnKind::Not, Ty::Int(_)) => format!("(^{v})"),
            (UnKind::Not, _) => format!("(!{v})"),
        })
    }

    fn binary(&mut self, op: BinKind, a: &PExpr, b: &PExpr) -> PResult<String> {
        let (x, y) = (self.expr(a)?, self.expr(b)?);
        let sym = |op: BinKind| match op {
            BinKind::Add => "+",
            BinKind::Sub => "-",
            BinKind::Mul => "*",
            BinKind::Div => "/",
            BinKind::Lt => "<",
            BinKind::Le => "<=",
            BinKind::Gt => ">",
            BinKind::Ge => ">=",
            BinKind::Eq => "==",
            BinKind::Ne => "!=",
            _ => "?",
        };
        match op {
            BinKind::And => return Ok(format!("({x} && {y})")),
            BinKind::Or => return Ok(format!("({x} || {y})")),
            _ => {}
        }
        let aggregate = !self.is_prim(&a.ty);
        match (&a.ty, op) {
            (_, BinKind::Eq | BinKind::Ne) if aggregate => {
                let eq = format!("wjEq({x}, {y})");
                Ok(if op == BinKind::Eq { eq } else { format!("(!{eq})") })
            }
            (_, BinKind::Lt | BinKind::Le | BinKind::Gt | BinKind::Ge) if aggregate => self.unsupported("ordering comparison of aggregate values"),
            (Ty::Int(_), _) => Ok(num::int_binary(op, &x, &y)),
            (Ty::F64 | Ty::F32, BinKind::Rem) => Ok(format!("wjFmod({x}, {y})")),
            // Two literal operands would be folded by Go's exact constant arithmetic, which has no `-0`
            // and rounds differently from IEEE; route them through a call so they run as float64 ops.
            (Ty::F64 | Ty::F32, BinKind::Add | BinKind::Sub | BinKind::Mul)
                if matches!(*a.kind, PK::Float(_)) && matches!(*b.kind, PK::Float(_)) =>
            {
                let f = match op {
                    BinKind::Add => "wjFadd",
                    BinKind::Sub => "wjFsub",
                    _ => "wjFmul",
                };
                Ok(format!("{f}({x}, {y})"))
            }
            // Go rejects constant float division by zero at compile time; IEEE semantics (inf/NaN) need a call.
            (Ty::F64 | Ty::F32, BinKind::Div) => Ok(format!("wjFdiv({x}, {y})")),
            (Ty::Str, BinKind::Add) => Ok(format!("({x} + {y})")),
            (_, BinKind::Add | BinKind::Sub | BinKind::Mul | BinKind::Div | BinKind::Lt | BinKind::Le | BinKind::Gt | BinKind::Ge | BinKind::Eq | BinKind::Ne) => Ok(format!("({x} {} {y})", sym(op))),
            _ => self.unsupported("operator on this type"),
        }
    }

    fn index(&mut self, a: &PExpr, i: &PExpr) -> PResult<String> {
        let o = self.expr(a)?;
        if let PK::Range { start, end, inclusive } = &*i.kind {
            let (s, e) = (self.expr(start)?, self.expr(end)?);
            return Ok(match a.ty {
                Ty::Str => format!("wjStrSlice({o}, {s}, {e}, {inclusive})"),
                _ => format!("wjVecSlice({o}, {s}, {e}, {inclusive})"),
            });
        }
        let idx = self.expr(i)?;
        Ok(match &a.ty {
            Ty::Map(..) => format!("wjMapIndex({o}, {idx})"),
            _ => format!("{o}[{idx}]"),
        })
    }

    fn struct_lit(&mut self, name: &str, fields: &[(String, PExpr)], ty: &Ty) -> PResult<String> {
        let targs = self.targs(ty)?;
        let vals: PResult<Vec<String>> = fields.iter().map(|(n, v)| Ok(format!("{}: {}", field(n), self.expr(v)?))).collect();
        Ok(format!("{}{targs}{{{}}}", ident(name), vals?.join(", ")))
    }

    fn closure_snapshot(&mut self, params: &[(String, Ty)], ret: &Ty, body: &PBlock, captures: &[(String, Ty)], fn_ty: &Ty) -> PResult<String> {
        let lit = self.closure(params, ret, body)?;
        if captures.is_empty() {
            return Ok(lit);
        }
        let decl: PResult<Vec<String>> = captures.iter().map(|(n, t)| Ok(format!("{} {}", ident(n), self.ty(t)?))).collect();
        let vals: PResult<Vec<String>> = captures
            .iter()
            .map(|(n, t)| {
                let v = if self.ptr_locals.contains_key(n) { format!("(*{})", ident(n)) } else { ident(n) };
                self.clone_go(t, &v)
            })
            .collect();
        Ok(format!("(func({}) {} {{ return {lit} }})({})", decl?.join(", "), self.ty(fn_ty)?, vals?.join(", ")))
    }

    fn closure(&mut self, params: &[(String, Ty)], ret: &Ty, body: &PBlock) -> PResult<String> {
        let ps: PResult<Vec<String>> = params.iter().map(|(n, t)| Ok(format!("{} {}", ident(n), self.ty(t)?))).collect();
        let rt = if *ret == Ty::Unit || ret.is_unknown() { String::new() } else { format!(" {}", self.ty(ret)?) };
        let saved = std::mem::take(&mut self.out);
        let (saved_ind, saved_ret) = (self.ind, std::mem::replace(&mut self.ret, ret.clone()));
        self.ind += 1;
        self.stmts(&body.stmts)?;
        if *ret != Ty::Unit && !self.terminating(&body.stmts) {
            self.line("panic(\"unreachable\")");
        }
        let inner = std::mem::replace(&mut self.out, saved);
        self.ind = saved_ind;
        self.ret = saved_ret;
        Ok(format!("func({}){rt} {{\n{inner}{}}}", ps?.join(", "), "\t".repeat(self.ind)))
    }

    fn static_call(&mut self, owner: &str, name: &str, args: &[PExpr], ty: &Ty) -> PResult<String> {
        let user = self.is_struct(owner) || self.is_enum(owner) || self.is_trait(owner);
        if user {
            let f = self.find_fn(Some(owner), name);
            let targs = self.targs(ty)?;
            let a = self.args_for(f, args)?;
            return Ok(format!("{}_{}{targs}({a})", ident(owner), ident(name)));
        }
        let a = self.plain_args(args)?;
        self.builtin_static(owner, name, &a, args, ty)
    }
}

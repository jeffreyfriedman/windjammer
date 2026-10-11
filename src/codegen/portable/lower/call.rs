//! Calls: free functions, closures, static paths, user and built-in methods, macros.

use super::{Lowerer, PResult};
use crate::codegen::portable::builtins;
use crate::codegen::portable::env::FnSig;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{from_ast, Ty};
use crate::parser::{Expression, Literal, MacroDelimiter, Type};

type Args<'a> = [(Option<String>, &'a Expression<'a>)];

/// Structural unification used to infer generic arguments from actual argument types.
pub(super) fn unify(pat: &Ty, actual: &Ty, params: &[String], bound: &mut Vec<Ty>) {
    match (pat, actual) {
        (Ty::Param(n), a) if !matches!(a, Ty::Unknown) => {
            if let Some(i) = params.iter().position(|p| p == n) {
                if i < bound.len() && bound[i].is_unknown() {
                    bound[i] = a.clone();
                }
            }
        }
        (Ty::Vec(p), Ty::Vec(a)) | (Ty::Option(p), Ty::Option(a)) | (Ty::Set(p), Ty::Set(a)) | (Ty::Iter(p), Ty::Iter(a)) | (Ty::Range(p), Ty::Range(a)) => unify(p, a, params, bound),
        (Ty::Map(pk, pv), Ty::Map(ak, av)) | (Ty::Result(pk, pv), Ty::Result(ak, av)) => {
            unify(pk, ak, params, bound);
            unify(pv, av, params, bound);
        }
        (Ty::Tuple(p), Ty::Tuple(a)) => p.iter().zip(a).for_each(|(p, a)| unify(p, a, params, bound)),
        (Ty::Adt(pn, p), Ty::Adt(an, a)) if pn == an => p.iter().zip(a).for_each(|(p, a)| unify(p, a, params, bound)),
        (Ty::Fn(pp, pr), Ty::Fn(ap, ar)) => {
            pp.iter().zip(ap).for_each(|(p, a)| unify(p, a, params, bound));
            unify(pr, ar, params, bound);
        }
        _ => {}
    }
}

impl<'e> Lowerer<'e> {
    pub(super) fn call(&mut self, function: &Expression, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        match function {
            Expression::Identifier { name, .. } => self.named_call(name, args, want),
            Expression::FieldAccess { object, field, .. } => {
                // The parser desugars `Type::ctor(..)` and (in `match`/`for` heads) `recv.method(..)` to
                // `Call(FieldAccess)`; a call of a field value would need parentheses in the source.
                if let Expression::Identifier { name: owner, .. } = object {
                    if self.lookup(owner).is_none() {
                        return self.path_call(owner, field, args, want);
                    }
                }
                self.method_call(object, field, None, args, want)
            }
            other => {
                let f = self.expr(other, None)?;
                self.call_value(f, args)
            }
        }
    }

    fn named_call(&mut self, name: &str, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        let plain: Vec<&Expression> = args.iter().map(|(_, e)| *e).collect();
        match name {
            "println" | "print" | "eprintln" | "eprint" => return self.print_macro(name, &plain),
            "panic" => return self.panic_macro(&plain),
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" => return self.assert_macro(name, &plain),
            "Some" | "Ok" | "Err" => return self.wrapper_ctor(name, &plain, want),
            _ => {}
        }
        if let Some(l) = self.lookup(name) {
            if matches!(l.ty, Ty::Fn(..)) || l.ty.is_unknown() {
                let f = self.local_expr(name)?;
                return self.call_value(f, args);
            }
        }
        if let Some((owner, member)) = name.rsplit_once("::") {
            return self.path_call(owner, member, args, want);
        }
        if let Some(sig) = self.env.fns.get(name).cloned() {
            let (args, ret) = self.lower_args_sig(&sig, args, want, false)?;
            return Ok(PExpr::new(ret, PK::Call { name: name.to_string(), args }));
        }
        if let Some(en) = self.env.enum_of_variant(name).map(|s| s.to_string()) {
            return self.variant_ctor(&en, name, args, want);
        }
        self.err(format!("call of unknown function `{name}`"))
    }

    fn call_value(&mut self, f: PExpr, args: &Args) -> PResult<PExpr> {
        let (ptys, ret) = match &f.ty {
            Ty::Fn(p, r) => (p.clone(), (**r).clone()),
            _ => (vec![], Ty::Unknown),
        };
        let mut out = Vec::new();
        for (i, (_, a)) in args.iter().enumerate() {
            let v = self.expr(a, ptys.get(i))?;
            out.push(self.own(v));
        }
        Ok(PExpr::new(ret, PK::CallValue { f, args: out }))
    }

    /// Lower arguments against a signature, inferring generics; returns (args, return type).
    fn lower_args_sig(&mut self, sig: &FnSig, args: &Args, want: Option<&Ty>, skip_self: bool) -> PResult<(Vec<PExpr>, Ty)> {
        let _ = skip_self;
        let mut bound = vec![Ty::Unknown; sig.type_params.len()];
        if let Some(w) = want {
            unify(&sig.ret, w, &sig.type_params, &mut bound);
        }
        let mut out = Vec::new();
        for (i, (_, a)) in args.iter().enumerate() {
            let Some((_, pty, mode)) = sig.params.get(i) else { return self.err("too many arguments") };
            let expect = pty.subst(&sig.type_params, &bound);
            let v = self.expr(a, Some(&expect).filter(|t| !t.is_unknown()))?;
            unify(pty, &v.ty, &sig.type_params, &mut bound);
            self.constrain(&expect, &v.ty);
            out.push(if *mode == ParamMode::Value { self.own(v) } else { v });
        }
        Ok((out, sig.ret.subst(&sig.type_params, &bound)))
    }

    fn path_call(&mut self, owner: &str, name: &str, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        let owner = if owner == "Self" { self.owner.clone().unwrap_or_default() } else { owner.to_string() };
        let module = owner.rsplit("::").next().unwrap_or(&owner).to_string();
        if self.env.variant(&owner, name).is_none() && self.env.method(&owner, name).is_none() {
            if let Some(sig) = crate::codegen::portable::stdlib::lookup(&module, name) {
                return self.std_call(&module, name, sig, args);
            }
        }
        if self.env.variant(&owner, name).is_some() {
            return self.variant_ctor(&owner, name, args, want);
        }
        if let Some(sig) = self.env.method(&owner, name).cloned() {
            if sig.recv.is_some() {
                return self.ufcs_call(&owner, name, args, want);
            }
            let (a, ret) = self.lower_args_sig(&sig, args, want, false)?;
            return Ok(PExpr::new(ret, PK::StaticCall { owner, name: name.to_string(), args: a }));
        }
        let mut out = Vec::new();
        for (i, (_, a)) in args.iter().enumerate() {
            let w = if owner == "String" || owner == "string" { Some(Ty::Str) } else { out.first().map(|p: &PExpr| p.ty.clone()).filter(|_| i > 0) };
            let v = self.expr(a, w.as_ref())?;
            out.push(self.own(v));
        }
        match builtins::static_ret(&owner, name, &out, want) {
            Some(ty) => Ok(PExpr::new(ty, PK::StaticCall { owner, name: name.to_string(), args: out })),
            None => self.err(format!("unknown associated function `{owner}::{name}`")),
        }
    }

    fn std_call(&mut self, module: &str, name: &str, sig: crate::codegen::portable::stdlib::StdSig, args: &Args) -> PResult<PExpr> {
        if args.len() != sig.params.len() {
            return self.err(format!("`{module}::{name}` takes {} argument(s)", sig.params.len()));
        }
        let mut out = Vec::new();
        for ((_, a), want) in args.iter().zip(&sig.params) {
            out.push(self.expr(a, Some(want))?);
        }
        Ok(PExpr::new(sig.ret, PK::StdCall { module: module.to_string(), name: name.to_string(), args: out }))
    }

    /// `Type::method(recv, args...)` form of a method call.
    fn ufcs_call(&mut self, owner: &str, name: &str, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        let Some(((_, recv), rest)) = args.split_first() else { return self.err("method call without receiver") };
        let _ = owner;
        self.method_call(recv, name, None, rest, want)
    }

    pub(super) fn variant_ctor(&mut self, en: &str, var: &str, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        let info = self.env.variant(en, var).cloned().unwrap();
        let einfo = self.env.enums.get(en).cloned().unwrap();
        let mut bound = self.enum_args_from_want(en, want);
        let mut fields = Vec::new();
        for (i, (_, a)) in args.iter().enumerate() {
            let Some((fname, fty)) = info.fields.get(i) else { return self.err(format!("too many fields for `{en}::{var}`")) };
            let expect = fty.subst(&einfo.type_params, &bound);
            let v = self.expr(a, Some(&expect).filter(|t| !t.is_unknown()))?;
            unify(fty, &v.ty, &einfo.type_params, &mut bound);
            self.constrain(&expect, &v.ty);
            fields.push((fname.clone(), self.own(v)));
        }
        Ok(PExpr::new(Ty::Adt(en.to_string(), bound), PK::Variant { enum_name: en.to_string(), variant: var.to_string(), fields }))
    }

    fn wrapper_ctor(&mut self, name: &str, args: &[&Expression], want: Option<&Ty>) -> PResult<PExpr> {
        let Some(arg) = args.first() else { return self.err(format!("`{name}` without argument")) };
        match name {
            "Some" => {
                let w = match want {
                    Some(Ty::Option(t)) => Some((**t).clone()),
                    _ => None,
                };
                let v = self.expr(arg, w.as_ref())?;
                let v = self.own(v);
                Ok(PExpr::new(Ty::Option(Box::new(v.ty.clone())), PK::Some(v)))
            }
            "Ok" => {
                let (w, e) = match want {
                    Some(Ty::Result(t, e)) => (Some((**t).clone()), (**e).clone()),
                    _ => (None, Ty::Unknown),
                };
                let v = self.expr(arg, w.as_ref())?;
                let v = self.own(v);
                Ok(PExpr::new(Ty::Result(Box::new(v.ty.clone()), Box::new(e)), PK::Ok(v)))
            }
            _ => {
                let (t, w) = match want {
                    Some(Ty::Result(t, e)) => ((**t).clone(), Some((**e).clone())),
                    _ => (Ty::Unknown, None),
                };
                let v = self.expr(arg, w.as_ref())?;
                let v = self.own(v);
                Ok(PExpr::new(Ty::Result(Box::new(t), Box::new(v.ty.clone())), PK::Err(v)))
            }
        }
    }

    pub(super) fn method_call(&mut self, object: &Expression, method: &str, turbofish: Option<&[Type]>, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        // Quirk of the `match`-head parser: `recv.name::<T>(..)` arrives as `MethodCall { method: "", object: FieldAccess }`.
        if method.is_empty() {
            if let Expression::FieldAccess { object: inner, field, .. } = object {
                return self.method_call(inner, field, turbofish, args, want);
            }
        }
        let recv = self.expr(object, None)?;
        let rty = self.resolve_for_dispatch(&recv.ty);
        let recv = PExpr { ty: rty, ..recv };
        let owner_ty = match &recv.ty {
            Ty::Adt(n, a) => Some((n.clone(), a.clone())),
            _ => None,
        };
        if let Some((owner, targs)) = owner_ty {
            if let Some(sig) = self.env.method(&owner, method).cloned() {
                return self.user_method(recv, &owner, &targs, method, &sig, args, want);
            }
            if method == "clone" {
                let ty = recv.ty.clone();
                return Ok(PExpr::new(ty, PK::Copy(recv)));
            }
            return self.err(format!("no method `{method}` on `{owner}`"));
        }
        if let Ty::Param(p) = &recv.ty {
            let traits = self.bounds.get(p).cloned().unwrap_or_default();
            for t in traits {
                if let Some(sig) = self.env.method(&t, method).cloned() {
                    return self.user_method(recv, &t, &[], method, &sig, args, want);
                }
            }
            if method == "clone" {
                let ty = recv.ty.clone();
                return Ok(PExpr::new(ty, PK::Copy(recv)));
            }
        }
        self.builtin_method(recv, method, turbofish, args, want)
    }

    #[allow(clippy::too_many_arguments)]
    fn user_method(&mut self, recv: PExpr, owner: &str, targs: &[Ty], method: &str, sig: &FnSig, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        let mut sig = sig.clone();
        let tp = self.env.structs.get(owner).map(|s| s.type_params.clone()).or_else(|| self.env.enums.get(owner).map(|e| e.type_params.clone())).unwrap_or_default();
        if !targs.is_empty() {
            sig.params = sig.params.iter().map(|(n, t, m)| (n.clone(), t.subst(&tp, targs), *m)).collect();
            sig.ret = sig.ret.subst(&tp, targs);
        }
        let recv = if sig.recv == Some(ParamMode::Value) { self.own(recv) } else { recv };
        let (a, ret) = self.lower_args_sig(&sig, args, want, true)?;
        Ok(PExpr::new(ret, PK::Method { recv, name: method.to_string(), args: a, res: Resolution::User { owner: owner.to_string() } }))
    }

    fn builtin_method(&mut self, recv: PExpr, method: &str, turbofish: Option<&[Type]>, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        if recv.ty.is_unknown() {
            return self.err(format!("cannot infer receiver type for `.{method}()` (annotate the variable or closure parameter)"));
        }
        if matches!(method, "clone" | "to_owned") && !matches!(recv.ty, Ty::Iter(_)) {
            let ty = recv.ty.clone();
            return Ok(PExpr::new(ty, PK::Copy(recv)));
        }
        let tf = turbofish.and_then(|t| t.first()).map(|t| from_ast(t, &self.generics));
        let want_for_ret: Option<Ty> = match (method, &tf) {
            ("parse", Some(t)) => Some(Ty::Result(Box::new(t.clone()), Box::new(Ty::Str))),
            ("collect", Some(t)) | ("sum", Some(t)) | ("product", Some(t)) => Some(t.clone()),
            _ => want.cloned(),
        };
        let mut lowered: Vec<PExpr> = Vec::new();
        for (i, (_, a)) in args.iter().enumerate() {
            let w = builtins::arg_want(&recv.ty, method, i, &lowered);
            let v = self.expr(a, w.as_ref())?;
            if let Some(w) = &w {
                self.constrain(w, &v.ty);
            }
            lowered.push(if builtins::stores_args(method) { self.own(v) } else { v });
        }
        let ty = builtins::ret_ty(&recv.ty, method, &lowered, want_for_ret.as_ref());
        let Some(mut ty) = ty else { return self.err(format!("built-in method `{}::{method}` is not supported", recv.ty.describe())) };
        if method == "collect" && ty == Ty::Vec(Box::new(Ty::Unknown)) {
            ty = Ty::Vec(Box::new(recv.ty.elem().unwrap_or(Ty::Unknown)));
        }
        if method == "collect" {
            if let (Some(src), Some(dst)) = (recv.ty.elem(), ty.elem()) {
                self.constrain(&dst, &src);
            }
        }
        let ty = refine_container(ty, &recv, method, &lowered);
        Ok(PExpr::new(ty, PK::Method { recv, name: method.to_string(), args: lowered, res: Resolution::Builtin }))
    }

    pub(super) fn print_macro(&mut self, name: &str, args: &[&Expression]) -> PResult<PExpr> {
        let newline = name.ends_with("ln");
        let to_stderr = name.starts_with('e');
        let pieces = self.pieces_from_args(args)?;
        Ok(PExpr::new(Ty::Unit, PK::Print { newline, to_stderr, pieces }))
    }

    fn pieces_from_args(&mut self, args: &[&Expression]) -> PResult<Vec<FmtPiece>> {
        match args.first() {
            None => Ok(vec![]),
            Some(Expression::Literal { value: Literal::String(s), .. }) => self.format_pieces(s, &args[1..]),
            Some(other) => {
                let v = self.expr(other, None)?;
                Ok(vec![FmtPiece::Arg(v, FmtSpec::default())])
            }
        }
    }

    fn panic_macro(&mut self, args: &[&Expression]) -> PResult<PExpr> {
        let pieces = self.pieces_from_args(args)?;
        let msg = PExpr::new(Ty::Str, PK::Format(pieces));
        Ok(PExpr::new(Ty::Unknown, PK::Panic(msg)))
    }

    fn assert_macro(&mut self, name: &str, args: &[&Expression]) -> PResult<PExpr> {
        let Some(first) = args.first() else { return self.err("assert without condition") };
        let cond = if name == "assert_eq" || name == "assert_ne" {
            let second = args.get(1).ok_or_else(|| self.unsupported("assert_eq needs two arguments".into()))?;
            let (a, b) = (self.expr(first, None)?, self.expr(second, Some(&Ty::Unknown))?);
            let kind = if name == "assert_eq" { BinKind::Eq } else { BinKind::Ne };
            PExpr::new(Ty::Bool, PK::Binary(kind, a, b))
        } else {
            self.expr(first, Some(&Ty::Bool))?
        };
        let msg_args = if name.starts_with("assert_") { &args[2.min(args.len())..] } else { &args[1.min(args.len())..] };
        let pieces = if msg_args.is_empty() { vec![FmtPiece::Lit(format!("assertion failed ({name})"))] } else { self.pieces_from_args(msg_args)? };
        let panic = PExpr::new(Ty::Unknown, PK::Panic(PExpr::new(Ty::Str, PK::Format(pieces))));
        let not = PExpr::new(Ty::Bool, PK::Unary(UnKind::Not, cond));
        let then = PBlock { stmts: vec![PStmt::Expr(panic)], tail: None };
        Ok(PExpr::new(Ty::Unit, PK::If { cond: not, then, els: PBlock::default() }))
    }

    pub(super) fn macro_call(&mut self, name: &str, args: &[&Expression], is_repeat: bool, want: Option<&Ty>) -> PResult<PExpr> {
        let _ = MacroDelimiter::Parens;
        match name {
            "println" | "print" | "eprintln" | "eprint" => self.print_macro(name, args),
            "panic" | "unreachable" | "todo" | "unimplemented" => self.panic_macro(args),
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" => self.assert_macro(name, args),
            "format" => {
                let pieces = self.pieces_from_args(args)?;
                Ok(PExpr::new(Ty::Str, PK::Format(pieces)))
            }
            "vec" if is_repeat => {
                let ew = match want {
                    Some(Ty::Vec(t)) => Some((**t).clone()),
                    _ => None,
                };
                let elem = self.expr(args[0], ew.as_ref())?;
                let n = self.expr(args[1], Some(&Ty::usize()))?;
                let n = self.ensure_usize(n);
                Ok(PExpr::new(Ty::Vec(Box::new(elem.ty.clone())), PK::VecRepeat(self.own(elem), n)))
            }
            "vec" => self.vec_literal(args, want),
            other => self.err(format!("macro `{other}!`")),
        }
    }
}

/// Fill `Unknown` holes of a builtin result type from the receiver/args (e.g. `Vec::new()` elem).
fn refine_container(ty: Ty, recv: &PExpr, method: &str, args: &[PExpr]) -> Ty {
    match (&ty, method) {
        (Ty::Option(inner), "get" | "first" | "last") if inner.is_unknown() => recv.ty.elem().map(|t| Ty::Option(Box::new(t))).unwrap_or(ty),
        _ => {
            let _ = args;
            ty
        }
    }
}

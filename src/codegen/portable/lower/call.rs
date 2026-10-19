//! Calls: free functions, closures, static paths, user and built-in methods, macros.

use super::{Lowerer, PResult};
use crate::codegen::portable::builtins;
use crate::codegen::portable::env::FnSig;
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{from_ast, Ty};
use crate::intrinsics::Intrinsic;
use crate::parser::{Expression, Type};

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
            // `(|v| body)(x)` (what `x |> |v| body` desugars to): type the closure
            // parameters from the argument types, otherwise they default to the
            // wrong integer width on typed backends.
            Expression::Closure { .. } => {
                let mut lowered = Vec::new();
                for (_, a) in args.iter() {
                    lowered.push(self.expr(a, None)?);
                }
                let want_fn = Ty::Fn(lowered.iter().map(|v| v.ty.clone()).collect(), Box::new(Ty::Unknown));
                let f = self.expr(function, Some(&want_fn))?;
                let ret = match &f.ty {
                    Ty::Fn(_, r) => (**r).clone(),
                    _ => Ty::Unknown,
                };
                let out = lowered.into_iter().map(|v| self.own(v)).collect();
                Ok(PExpr::new(ret, PK::CallValue { f, args: out }))
            }
            other => {
                let f = self.expr(other, None)?;
                self.call_value(f, args)
            }
        }
    }

    fn named_call(&mut self, name: &str, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        // `self::f()` is the module-relative path to the free function `f` (never a method call;
        // the parser keeps it distinct from `self.f()`). Portable modules are flat.
        if let Some(rest) = name.strip_prefix("self::") {
            return self.named_call(rest, args, want);
        }
        let plain: Vec<&Expression> = args.iter().map(|(_, e)| *e).collect();
        if let Some(intr) = Intrinsic::from_call_name(name).filter(|_| !self.shadows_intrinsic(name)) {
            return self.intrinsic(intr, &plain, false, want);
        }
        if matches!(name, "Some" | "Ok" | "Err") {
            return self.wrapper_ctor(name, &plain, want);
        }
        if let Some(l) = self.lookup(name) {
            if matches!(l.ty, Ty::Fn(..)) || l.ty.is_unknown() {
                let f = self.local_expr(name)?;
                return self.call_value(f, args);
            }
        }
        if let Some(last) = crate::codegen::portable::alias::strip_modules(name) {
            return self.named_call(&last, args, want);
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
        if self.env.structs.get(name).is_some_and(|s| s.tuple) {
            return self.tuple_struct_ctor(name, args, want);
        }
        self.err(format!("call of unknown function `{name}`"))
    }

    /// `P(a, b)` for `struct P(A, B)`: a struct literal with fields `0`, `1`, ...
    fn tuple_struct_ctor(&mut self, name: &str, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        let info = self.env.structs[name].clone();
        if args.len() != info.fields.len() {
            return self.err(format!("`{name}` takes {} field(s), got {}", info.fields.len(), args.len()));
        }
        let mut bound: Vec<Ty> = match want {
            Some(Ty::Adt(n, a)) if n == name => a.clone(),
            _ => vec![Ty::Unknown; info.type_params.len()],
        };
        let mut fields = Vec::new();
        for ((fname, fty), (_, a)) in info.fields.iter().zip(args.iter()) {
            let expect = fty.subst(&info.type_params, &bound);
            let v = self.expr(a, Some(&expect).filter(|t| !t.is_unknown()))?;
            unify(fty, &v.ty, &info.type_params, &mut bound);
            self.constrain(&expect, &v.ty);
            fields.push((fname.clone(), self.own(v)));
        }
        Ok(PExpr::new(Ty::Adt(name.to_string(), bound), PK::Struct { name: name.to_string(), fields }))
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
        if crate::codegen::portable::alias::is_module(&owner) {
            return self.named_call(name, args, want);
        }
        let owner = crate::codegen::portable::alias::strip_modules(&owner).unwrap_or(owner);
        let owner = crate::codegen::portable::alias::nominal(&owner).unwrap_or(owner);
        // `Box<T>` / `Rc<T>` / `Arc<T>` are erased to `T` (values are already heap objects or
        // copied by value semantics), so `Box::new(x)` is `x`.
        if matches!(owner.as_str(), "Box" | "Rc" | "Arc") && name == "new" && args.len() == 1 && !self.env.structs.contains_key(&owner) {
            let v = self.expr(args[0].1, want)?;
            return Ok(self.own(v));
        }
        let module = owner.rsplit("::").next().unwrap_or(&owner).to_string();
        if self.env.variant(&owner, name).is_none() && self.env.method(&owner, name).is_none() {
            if let Some(sig) = crate::codegen::portable::stdlib::lookup(&module, name) {
                return self.std_call(&module, name, sig, args);
            }
        }
        if self.env.variant(&owner, name).is_some() {
            return self.variant_ctor(&owner, name, args, want);
        }
        if self.env.method(&owner, name).is_none() && self.env.trait_impls.contains_key(&(owner.clone(), name.to_string())) {
            return self.generic_impl_call(&owner, name, args, want);
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

    /// `E::from(x)` with several `impl From<X> for E`: the impl is chosen by the argument type.
    fn generic_impl_call(&mut self, owner: &str, name: &str, args: &Args, want: Option<&Ty>) -> PResult<PExpr> {
        let single = self.env.trait_impls[&(owner.to_string(), name.to_string())].len() == 1;
        let first = match args.first() {
            Some((_, a)) if !single => Some(self.expr(a, None)?),
            _ => None,
        };
        let Some(mangled) = self.pick_impl(owner, name, first.as_ref().map(|e| e.ty.clone())) else {
            return self.err(format!("no `{owner}::{name}` impl for this argument type"));
        };
        let sig = self.env.method(owner, &mangled).cloned().expect("registered impl method");
        let (mut a, ret) = match first {
            None => self.lower_args_sig(&sig, args, want, false)?,
            Some(f) => {
                let (rest, ret) = self.lower_args_sig(&FnSig { params: sig.params[1..].to_vec(), ..sig.clone() }, &args[1..], want, false)?;
                let mut all = vec![if sig.params[0].2 == ParamMode::Value { self.own(f) } else { f }];
                all.extend(rest);
                (all, ret)
            }
        };
        Ok(PExpr::new(ret, PK::StaticCall { owner: owner.to_string(), name: mangled, args: a }))
    }

    /// The emitted name of the `owner::name` generic-trait impl whose first trait argument is
    /// `arg` (any impl when there is exactly one).
    pub(super) fn pick_impl(&mut self, owner: &str, name: &str, arg: Option<Ty>) -> Option<String> {
        let cands = self.env.trait_impls.get(&(owner.to_string(), name.to_string()))?.clone();
        if cands.len() == 1 {
            return Some(cands[0].1.clone());
        }
        let arg = self.resolve(&arg?);
        if let Some((_, n)) = cands.iter().find(|(targs, _)| targs.first() == Some(&arg)) {
            return Some(n.clone());
        }
        // An unsuffixed literal: the only integer (or float) impl, as rustc's inference picks it.
        let Ty::Var(v) = arg else { return None };
        let fits = |t: &Ty| if self.float_vars.contains(&v) { matches!(t, Ty::F32 | Ty::F64) } else if self.int_vars.contains(&v) { matches!(t, Ty::Int(_)) } else { false };
        let hits: Vec<&(Vec<Ty>, String)> = cands.iter().filter(|(targs, _)| targs.first().is_some_and(fits)).collect();
        let [(targs, n)] = hits.as_slice() else { return None };
        let (t, n) = (targs[0].clone(), n.clone());
        self.constrain(&t, &arg);
        Some(n)
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
        // `x.into()` where the expected type has `impl From<X>`: that impl's `from`.
        if method == "into" && args.is_empty() {
            if let Some(Ty::Adt(target, _)) = want.map(|w| self.resolve(w)) {
                if let Some(m) = self.pick_impl(&target, "from", Some(recv.ty.clone())) {
                    let ty = want.cloned().unwrap_or(Ty::Unknown);
                    let recv = self.own(recv);
                    return Ok(PExpr::new(ty, PK::StaticCall { owner: target, name: m, args: vec![recv] }));
                }
            }
        }
        if method == crate::parser::optional_chain::OPT_CHAIN {
            return self.optional_chain(recv, args);
        }
        let owner_ty = match &recv.ty {
            Ty::Adt(n, a) => Some((n.clone(), a.clone())),
            _ => None,
        };
        let builtin_ordering = crate::codegen::portable::ordering::is_ordering(&recv.ty) && !self.env.enums.contains_key(crate::codegen::portable::ordering::NAME);
        if let Some((owner, targs)) = owner_ty.filter(|_| !builtin_ordering) {
            // `Scratch::new_list()` is `Vec::new()` (Go cannot infer a type argument that only the return type mentions).
            if method == "new_list" && args.is_empty() && super::arena::is_prelude_scratch(&self.env, &owner) {
                return self.path_call("Vec", "new", args, want);
            }
            if method == "get_mut" && args.len() == 1 && super::arena::is_prelude_arena(&self.env, &owner) {
                return self.arena_slot(recv, &targs, args);
            }
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
            // `t.into()` for `T: Into<string>`: the std types that convert into a string are
            // the string types themselves (and `char`, a one-character string on JS), so it is
            // a checked conversion of the generic value to `string`.
            let into_string = self.bounds.get(p).is_some_and(|b| b.iter().any(|t| t.replace(' ', "") == "Into<string>" || t.replace(' ', "") == "Into<String>"));
            if method == "into" && args.is_empty() && into_string {
                return Ok(PExpr::new(Ty::Str, PK::Cast(self.own(recv), Ty::Str)));
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
        // `v[a..b]` / `s[a..]` (the parser's `.slice(a, b)`): a range index (byte offsets on strings).
        if method == "slice" && args.len() == 2 && matches!(recv.ty, Ty::Vec(_) | Ty::Str) {
            let (a, b) = (self.expr(args[0].1, Some(&Ty::usize()))?, self.expr(args[1].1, Some(&Ty::usize()))?);
            let (a, b) = (self.ensure_usize(a), self.ensure_usize(b));
            let range = PExpr::new(Ty::Range(Box::new(Ty::usize())), PK::Range { start: a, end: b, inclusive: false });
            let ty = recv.ty.clone();
            return Ok(PExpr::new(ty, PK::Index(recv, range)));
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
        // SPEC-NUM-014: a length is usable as `int` (`i < v.len()` with `i: int`, `-> int { v.len() }`).
        let ty = if matches!(method, "len" | "count" | "capacity") && ty == Ty::usize() { self.len_var() } else { ty };
        Ok(PExpr::new(ty, PK::Method { recv, name: method.to_string(), args: lowered, res: Resolution::Builtin }))
    }

    /// Deprecated `name!(..)` macro nodes the parser left in the AST (`format!`, `vec![..]`, and
    /// intrinsic macros with non-literal arguments).
    pub(super) fn macro_call(&mut self, name: &str, args: &[&Expression], is_repeat: bool, want: Option<&Ty>) -> PResult<PExpr> {
        match Intrinsic::from_macro_name(name) {
            Some(intr) => self.intrinsic(intr, args, is_repeat, want),
            None => self.err(format!("macro `{name}!`")),
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

//! Built-in std method / constructor emission for Go (helpers live in `wj_core.go`).

use super::names::ident;
use super::num;
use super::Go;
use crate::codegen::portable::template::{self, Operands, TypeRef};
use crate::stdlib_surface::lowering::{CoreType, Fact, Guard, Hook, Modifier, Rule, Target, Widen};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::env::core_type;
use crate::codegen::portable::ty::{IntKind, Ty};

impl<'m> Go<'m> {
    pub fn method(&mut self, recv: &PExpr, name: &str, args: &[PExpr], res: &Resolution, ty: &Ty) -> PResult<String> {
        if let Resolution::User { owner } = res {
            return self.user_method(recv, owner, name, args);
        }
        let m = self.m;
        let found = m.lowering.lookup(core_type(&recv.ty), name);
        // A length typed `int` (SPEC-NUM-014) converts the `uint` length: any method declared to give a `usize`.
        if let (Some((_, row)), Ty::Int(k)) = (found, ty) {
            if row.returns_usize && *k != IntKind::Usize {
                let n = self.method(recv, name, args, res, &Ty::usize())?;
                return Ok(format!("{}({n})", super::types::int_name(*k)));
            }
        }
        let a: Vec<String> = args.iter().map(|e| self.expr(e)).collect::<PResult<_>>()?;
        if let Some(go) = self.enumerable_method(recv, name, &a)? {
            return Ok(go);
        }
        let r = self.expr(recv)?;
        if let Some(s) = self.ordering_method(recv, name, &r, &a) {
            return Ok(s);
        }
        match (found, core_type(&recv.ty)) {
            (Some((owner, _)), _) => self.lowered(owner, recv, name, &a, args, ty),
            (None, Some(ct)) => self.unsupported(format!("{} method `{name}`", ct.noun())),
            (None, None) => self.unsupported(format!("method `{name}` on `{}`", recv.ty.describe())),
        }
    }

    fn user_method(&mut self, recv: &PExpr, owner: &str, name: &str, args: &[PExpr]) -> PResult<String> {
        let f = self.find_fn(Some(owner), name);
        let a = self.args_for(f, args)?;
        if self.is_enum(owner) || f.is_some_and(|f| !f.type_params.is_empty()) {
            let mode = f.and_then(|f| f.recv).unwrap_or(ParamMode::Ref);
            let r = if mode == ParamMode::MutRef { self.addr(recv)? } else { self.expr(recv)? };
            let sep = if a.is_empty() { "" } else { ", " };
            return Ok(format!("{}_{}({r}{sep}{a})", ident(owner), ident(name)));
        }
        let r = self.base(recv)?;
        Ok(format!("{r}.{}({a})", ident(name)))
    }

    fn rv(&mut self, recv: &PExpr) -> PResult<String> {
        self.expr(recv)
    }

    /// A call of a std method of a migrated type: print the receiver's row for Go (`std/rust_core.wji`). `args` are the call's
    /// arguments (a `char` where the row wants a string is converted), `a` the same, printed.
    fn lowered(&mut self, ct: CoreType, recv: &PExpr, name: &str, a: &[String], args: &[PExpr], ty: &Ty) -> PResult<String> {
        let r = self.rv(recv)?;
        let m = self.m;
        let Some(row) = m.lowering.row(ct, name) else { return self.unsupported(format!("{} method `{name}`", ct.noun())) };
        let elem = recv.ty.elem().unwrap_or(Ty::Unknown);
        let prim = self.is_prim(&elem);
        let guard = |g: Guard| match g {
            Guard::ElemPrim => prim,
            Guard::ElemPrimNotBool => prim && elem != Ty::Bool,
            g => template::type_guard(g, &recv.ty, ty),
        };
        // `widen`: the trait computes in `float64`; an `f32` receiver is converted in and the result converted back.
        let widen = m.lowering.widen(ct, Target::Go).filter(|_| recv.ty == Ty::F32);
        match row.select(Target::Go, guard) {
            Some(Rule::Emit { template, .. }) => {
                // the receiver's address, for a row that changes it (`{&self}`)
                let place = if template.contains("{&self}") { Some(self.addr(recv)?) } else { None };
                let mut go_ty = |t: &Ty| self.ty(t);
                let out = template::render_emit(template, &row.params, &mut GoOperands { receiver: &r, place, args: a, arg_exprs: args, widen, recv: &recv.ty, result: ty, go_ty: &mut go_ty })?;
                Ok(if widen.is_some() && row.returns_receiver { format!("float32({out})") } else { out })
            }
            Some(Rule::Assign(_)) => self.unsupported(format!("{} method `{name}`: `assign` is a JavaScript place update", ct.noun())),
            Some(Rule::Hook(Hook::Display)) => self.display(recv, &FmtSpec::default()),
            Some(Rule::Hook(Hook::Clone)) => self.unsupported(format!("{} method `{name}`: `clone` is a JavaScript hook", ct.noun())),
            Some(Rule::Hook(Hook::Cast)) => self.unsupported(format!("{} method `{name}`: `cast` is the hook of an associated function", ct.noun())),
            Some(Rule::Unsupported(why)) => self.unsupported(format!("{} method `{name}`: {why}", ct.noun())),
            None => self.unsupported(format!("{} method `{name}` has no Go rule", ct.noun())),
        }
    }

    /// A call of an associated function or constant of a std type (`Vec::new`, `i64::MAX`): print its row for Go. `ty` is the
    /// call's type, which is also the type the function is on (`{R.max}`, `{R.go}`).
    pub fn builtin_static(&mut self, owner: &str, name: &str, a: &[String], args: &[PExpr], ty: &Ty) -> PResult<String> {
        if let Some(c) = self.ordering_const(owner, name) {
            return Ok(c);
        }
        let m = self.m;
        let Some((_, row)) = m.lowering.static_row(owner, name) else { return self.unsupported(format!("`{owner}::{name}`")) };
        match row.select(Target::Go, |g| template::type_guard(g, ty, ty)) {
            Some(Rule::Emit { template, .. }) => {
                let mut go_ty = |t: &Ty| self.ty(t);
                template::render_emit(template, &row.params, &mut GoOperands { receiver: "", place: None, args: a, arg_exprs: args, widen: None, recv: ty, result: ty, go_ty: &mut go_ty })
            }
            Some(Rule::Hook(Hook::Cast)) => match (args.first(), a.first()) {
                (Some(from), Some(text)) => Ok(num::cast(&from.ty, ty, text)),
                _ => self.unsupported(format!("`{owner}::{name}` converts its argument")),
            },
            Some(Rule::Unsupported(why)) => self.unsupported(format!("`{owner}::{name}`: {why}")),
            _ => self.unsupported(format!("`{owner}::{name}` has no Go rule")),
        }
    }
}

/// The operands of a Go template: the receiver and arguments as printed, converted to the compute type for `:f64`
/// when the trait declares `widen` and the receiver is narrower (`float64(x)` for an `f32`).
struct GoOperands<'a> {
    receiver: &'a str,
    /// The receiver's address, when the template asks for it.
    place: Option<String>,
    args: &'a [String],
    /// The arguments' expressions, for their types (a `char` where a string is wanted).
    arg_exprs: &'a [PExpr],
    /// `Some` when the receiver is narrower than the trait's compute type.
    widen: Option<Widen>,
    recv: &'a Ty,
    result: &'a Ty,
    /// The Go spelling of a type (`{R.go}` of a call whose result is not an integer: `map[string]int64`).
    go_ty: &'a mut dyn FnMut(&Ty) -> PResult<String>,
}

impl GoOperands<'_> {
    fn convert(&self, text: &str, ty: Option<&Ty>, modifier: Option<Modifier>) -> PResult<String> {
        match (modifier, self.widen) {
            (None, _) => Ok(text.to_string()),
            (Some(Modifier::F64), Some(Widen::F64)) => Ok(format!("float64({text})")),
            (Some(Modifier::F64), None) => Ok(text.to_string()),
            // a `char` is a rune: the string functions take its text
            (Some(Modifier::Str), _) => Ok(if ty == Some(&Ty::Char) { format!("string(rune({text}))") } else { text.to_string() }),
            (Some(Modifier::Num), _) => unsupported("this target has no `num` operand modifier for the template"),
        }
    }
}

impl Operands for GoOperands<'_> {
    fn receiver(&mut self, place: bool, modifier: Option<Modifier>) -> PResult<String> {
        if place {
            return match &self.place {
                Some(p) if modifier.is_none() => Ok(p.clone()),
                _ => unsupported("a receiver place in a Go template of this type"),
            };
        }
        self.convert(self.receiver, Some(self.recv), modifier)
    }

    fn arg(&mut self, index: usize, modifier: Option<Modifier>) -> PResult<String> {
        match self.args.get(index) {
            Some(a) => self.convert(a, self.arg_exprs.get(index).map(|e| &e.ty), modifier),
            None => unsupported(format!("the call has no argument {index}")),
        }
    }

    fn fact(&mut self, of: TypeRef, fact: Fact) -> PResult<String> {
        if fact == Fact::Go && of == TypeRef::Result && !matches!(self.result, Ty::Int(_)) {
            return (self.go_ty)(self.result);
        }
        match template::int_kind_of(of, self.recv, self.result).and_then(|k| int_fact(k, fact)) {
            Some(text) => Ok(text),
            None => unsupported(format!("the type fact `{}` is not available for this type in Go", fact.name())),
        }
    }
}

/// A fact about an integer type: its Go type name and range literals. `None`: Go has no such fact (`{T.chk}`, the wrap
/// text: the runtime helpers check and wrap).
fn int_fact(k: IntKind, fact: Fact) -> Option<String> {
    let (lo, hi) = num::range_of(k);
    match fact {
        Fact::Wide => Some((k.bits() == 64).to_string()),
        Fact::Min => Some(num::lit_int(lo, k)),
        Fact::Max => Some(num::lit_int(hi, k)),
        Fact::Zero => Some("0".to_string()),
        Fact::One => Some("1".to_string()),
        Fact::Go => Some(super::types::int_name(k).to_string()),
        Fact::Chk | Fact::WrapOpen | Fact::WrapClose => None,
        _ => template::common_int_fact(k, fact),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_facts_are_the_go_type_and_its_literals() {
        let f = |k, fact| int_fact(k, fact).unwrap();
        assert_eq!(f(IntKind::I8, Fact::Go), "int8");
        assert_eq!(f(IntKind::Usize, Fact::Go), "uint");
        assert_eq!(f(IntKind::U8, Fact::Max), "uint8(255)");
        assert_eq!(f(IntKind::I8, Fact::Min), "int8(-128)");
        assert_eq!(f(IntKind::U32, Fact::Zero), "0");
        assert_eq!(f(IntKind::U32, Fact::One), "1");
        assert_eq!(f(IntKind::I64, Fact::Wide), "true");
        assert_eq!(f(IntKind::I16, Fact::Bits), "16");
        assert_eq!(f(IntKind::I16, Fact::Signed), "true");
        assert_eq!(int_fact(IntKind::I32, Fact::Chk), None);
        assert_eq!(int_fact(IntKind::I32, Fact::WrapOpen), None);
    }
}

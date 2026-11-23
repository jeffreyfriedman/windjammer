//! Built-in std method / constructor emission for JavaScript.

use super::num;
use super::Js;
use crate::codegen::portable::template::{self, Operands, TypeRef};
use crate::stdlib_surface::lowering::{CoreType, Fact, Guard, Hook, Modifier, Rule, Target};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::env::{core_type, iterated};
use crate::codegen::portable::ty::{IntKind, Ty};

fn range_of(k: IntKind) -> (i128, i128) {
    let b = k.bits();
    if k.signed() { (-(1i128 << (b - 1)), (1i128 << (b - 1)) - 1) } else { (0, (1i128 << b) - 1) }
}

impl<'m> Js<'m> {
    pub fn method(&mut self, recv: &PExpr, name: &str, args: &[PExpr], res: &Resolution, ty: &Ty) -> PResult<String> {
        if let Resolution::User { owner } = res {
            let f = self.find_fn(Some(owner), name);
            let a = self.args_for(f, args)?;
            if let Some(f) = f.filter(|f| self.is_enum_mut_method(owner, f)) {
                let cell = self.cell_of(recv)?;
                let sep = if a.is_empty() { "" } else { ", " };
                return Ok(format!("{}.{}({cell}{sep}{a})", super::names::ident(owner), self.method_name(&f.name)));
            }
            let r = self.expr_recv(recv)?;
            return Ok(format!("{r}.{}({a})", self.method_name(name)));
        }
        let m = self.m;
        let found = m.lowering.lookup(core_type(&recv.ty), name);
        // A length typed as a narrow `int` (SPEC-NUM-014) is a Number, not a BigInt: any method declared to give a `usize`.
        if let (Some((_, row)), Ty::Int(k)) = (found, ty) {
            if row.returns_usize && !k.is_wide() {
                let n = self.method(recv, name, args, res, &Ty::usize())?;
                return Ok(format!("Number({n})"));
            }
        }
        let r = self.expr(recv)?;
        let a: Vec<String> = args.iter().map(|e| self.expr(e)).collect::<PResult<_>>()?;
        if let Some(js) = self.enumerable_method(recv, name, &r, &a, args)? {
            return Ok(js);
        }
        if let Some(s) = self.ordering_method(recv, name, &r, &a) {
            return Ok(s);
        }
        match (found, core_type(&recv.ty)) {
            (Some((owner, _)), _) => self.lowered(owner, recv, name, &r, &a, ty),
            (None, Some(ct)) => self.unsupported(format!("{} method `{name}`", ct.noun())),
            (None, None) => self.unsupported(format!("method `{name}` on `{}`", recv.ty.describe())),
        }
    }

    fn expr_recv(&mut self, e: &PExpr) -> PResult<String> {
        let s = self.expr(e)?;
        Ok(if matches!(&*e.kind, PK::Local(_) | PK::Field(..) | PK::TupleIdx(..) | PK::Call { .. } | PK::Method { .. } | PK::Index(..) | PK::Copy(_)) { s } else { format!("({s})") })
    }

    pub(super) fn sum_op(&self, t: &Ty, mul: bool) -> PResult<(String, String)> {
        let name = if mul { "product" } else { "sum" };
        match t {
            Ty::Int(k) => {
                let (z, o) = (num::zero(*k), num::one(*k));
                Ok((format!("(a, x) => {}(a {} x)", num::chk(*k), if mul { "*" } else { "+" }), if mul { o } else { z }))
            }
            Ty::F64 | Ty::F32 => Ok((format!("(a, x) => a {} x", if mul { "*" } else { "+" }), if mul { "1".into() } else { "0".into() })),
            other => self.unsupported(format!("{name}() over `{}`", other.describe())),
        }
    }

    /// A call of a std method of a migrated type: print the receiver's row for JavaScript (`std/rust_core.wji`); `ty` is
    /// the call's result type (`{R.chk}`, the `Ok` type of a `parse`).
    fn lowered(&mut self, ct: CoreType, recv: &PExpr, name: &str, r: &str, a: &[String], ty: &Ty) -> PResult<String> {
        let m = self.m;
        let Some(row) = m.lowering.row(ct, name) else { return self.unsupported(format!("{} method `{name}`", ct.noun())) };
        self.requires(ct.trait_name(), &recv.ty, ty)?;
        let elem = recv.ty.elem().unwrap_or(Ty::Unknown);
        let prim = self.is_prim(&elem);
        let guard = |g: Guard| match g {
            Guard::ElemPrim => prim,
            Guard::ElemPrimNotBool => prim && elem != Ty::Bool,
            g => template::type_guard(g, &recv.ty, ty),
        };
        match row.select(Target::Js, guard) {
            Some(Rule::Emit { template, .. }) => {
                let mut dbg = |t: &Ty| self.dbg_fn(t);
                template::render_emit(template, &row.params, &mut JsOperands { receiver: r, args: a, recv: &recv.ty, result: ty, place: None, dbg: &mut dbg })
            }
            Some(Rule::Assign(template)) => {
                let place = self.place(recv)?;
                let mut dbg = |t: &Ty| self.dbg_fn(t);
                let value = template::render_emit(template, &row.params, &mut JsOperands { receiver: r, args: a, recv: &recv.ty, result: ty, place: Some(&place), dbg: &mut dbg })?;
                Ok(format!("({place} = {value}, undefined)"))
            }
            Some(Rule::Hook(Hook::Display)) => self.display(recv, &FmtSpec::default()),
            // a copy of the elements by the element type's own cloner
            Some(Rule::Hook(Hook::Clone)) => Ok(match self.cloner(&iterated(&recv.ty).unwrap_or(Ty::Unknown)) {
                None => format!("{r}.slice()"),
                Some(c) => format!("{r}.map({c})"),
            }),
            Some(Rule::Hook(Hook::Cast)) => self.unsupported(format!("{} method `{name}`: `cast` is the hook of an associated function", ct.noun())),
            Some(Rule::Unsupported(why)) => self.unsupported(format!("{} method `{name}`: {why}", ct.noun())),
            None => self.unsupported(format!("{} method `{name}` has no JavaScript rule", ct.noun())),
        }
    }

    /// The condition the trait puts on every call of it (`@js(requires = ..)`): a map call needs a key a JavaScript `Map` compares by value.
    fn requires(&self, trait_name: &str, recv: &Ty, ty: &Ty) -> PResult<()> {
        let Some(req) = self.m.lowering.requirement(trait_name, Target::Js) else { return Ok(()) };
        if template::type_guard(req.guard, recv, ty) {
            return Ok(());
        }
        let key = [recv, ty].into_iter().find_map(|t| match t {
            Ty::Map(k, _) => Some(k.describe()),
            _ => None,
        });
        self.unsupported(req.reason.replace("{K}", &key.unwrap_or_default()))
    }

    /// A call of an associated function or constant of a std type (`Vec::new`, `i64::MAX`): print its row for JavaScript. `ty`
    /// is the call's type, which is also the type the function is on (`{R.max}`).
    pub fn builtin_static(&mut self, owner: &str, name: &str, a: &[String], args: &[PExpr], ty: &Ty) -> PResult<String> {
        if let Some(c) = self.ordering_const(owner, name) {
            return Ok(c);
        }
        let m = self.m;
        let Some((trait_name, row)) = m.lowering.static_row(owner, name) else { return self.unsupported(format!("`{owner}::{name}`")) };
        self.requires(trait_name, ty, ty)?;
        match row.select(Target::Js, |g| template::type_guard(g, ty, ty)) {
            Some(Rule::Emit { template, .. }) => {
                let mut dbg = |t: &Ty| self.dbg_fn(t);
                template::render_emit(template, &row.params, &mut JsOperands { receiver: "", args: a, recv: ty, result: ty, place: None, dbg: &mut dbg })
            }
            Some(Rule::Hook(Hook::Cast)) => match (args.first(), a.first()) {
                (Some(from), Some(text)) => Ok(num::cast(&from.ty, ty, text)),
                _ => self.unsupported(format!("`{owner}::{name}` converts its argument")),
            },
            Some(Rule::Unsupported(why)) => self.unsupported(format!("`{owner}::{name}`: {why}")),
            _ => self.unsupported(format!("`{owner}::{name}` has no JavaScript rule")),
        }
    }
}

/// The operands of a JavaScript template: the receiver and arguments as printed, and the facts of an integer type.
struct JsOperands<'a> {
    receiver: &'a str,
    args: &'a [String],
    recv: &'a Ty,
    result: &'a Ty,
    /// The receiver as a place, for an `assign` rule.
    place: Option<&'a str>,
    /// The debug formatter of a type (`{E.dbg}`): the emitter's own, asked only when a template names it.
    dbg: &'a mut dyn FnMut(&Ty) -> PResult<String>,
}

/// `Num`: an integer operand printed as a JavaScript Number (a BigInt is not a length or an index). No other modifier is
/// a JavaScript operation (`str`: a char is already a string).
fn modified(text: &str, modifier: Option<Modifier>) -> PResult<String> {
    match modifier {
        None => Ok(text.to_string()),
        Some(Modifier::Num) => Ok(format!("Number({text})")),
        Some(_) => unsupported("this target has no such operand modifier for the template"),
    }
}

impl Operands for JsOperands<'_> {
    fn receiver(&mut self, place: bool, modifier: Option<Modifier>) -> PResult<String> {
        if place {
            return match self.place {
                Some(p) if modifier.is_none() => Ok(p.to_string()),
                _ => unsupported("this target has no receiver place or modifier for the template"),
            };
        }
        modified(self.receiver, modifier)
    }

    fn arg(&mut self, index: usize, modifier: Option<Modifier>) -> PResult<String> {
        match self.args.get(index) {
            Some(a) => modified(a, modifier),
            None => unsupported(format!("the call has no argument {index}")),
        }
    }

    fn fact(&mut self, of: TypeRef, fact: Fact) -> PResult<String> {
        if fact == Fact::Dbg {
            return match (of, self.recv) {
                (TypeRef::Err, Ty::Result(_, e)) => (self.dbg)(e),
                _ => unsupported("the debug text is a fact about the error type of a `Result` receiver (`{E.dbg}`)"),
            };
        }
        match template::int_kind_of(of, self.recv, self.result).and_then(|k| int_fact(k, fact)) {
            Some(text) => Ok(text),
            None => unsupported(format!("the type fact `{}` is not available for this type in JavaScript", fact.name())),
        }
    }
}

/// A fact about an integer type, from the numeric helpers (`num`): the range-check wrapper, the literals of the type,
/// the text that wraps an expression into its range. `None`: JavaScript has no such fact (`{T.go}`).
fn int_fact(k: IntKind, fact: Fact) -> Option<String> {
    let (lo, hi) = range_of(k);
    match fact {
        Fact::Wide => Some(num::is_wide(k).to_string()),
        Fact::Min => Some(num::lit_int(lo, k)),
        Fact::Max => Some(num::lit_int(hi, k)),
        Fact::Lo => Some(format!("{lo}n")),
        Fact::Hi => Some(format!("{hi}n")),
        Fact::Zero => Some(num::zero(k)),
        Fact::One => Some(num::one(k)),
        Fact::Chk => Some(num::chk(k).to_string()),
        Fact::WrapOpen | Fact::WrapClose => {
            // `wrap_expr` brackets the expression: cut its output at a marker
            let wrapped = num::wrap_expr(k, "\u{1}");
            let (open, close) = wrapped.split_once('\u{1}')?;
            Some(if fact == Fact::WrapOpen { open } else { close }.to_string())
        }
        Fact::Go => None,
        _ => template::common_int_fact(k, fact),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_facts_are_the_numeric_helpers_text() {
        let f = |k, fact| int_fact(k, fact).unwrap();
        assert_eq!(f(IntKind::I8, Fact::Min), "(-128)");
        assert_eq!(f(IntKind::U8, Fact::Max), "255");
        assert_eq!(f(IntKind::I64, Fact::Min), "(-9223372036854775808n)");
        assert_eq!(f(IntKind::U64, Fact::Zero), "0n");
        assert_eq!(f(IntKind::U16, Fact::One), "1");
        assert_eq!(f(IntKind::I32, Fact::Chk), "$i32");
        assert_eq!(f(IntKind::Usize, Fact::Chk), "$u64");
        assert_eq!(f(IntKind::U64, Fact::Wide), "true");
        assert_eq!(f(IntKind::U32, Fact::Wide), "false");
        assert_eq!(f(IntKind::I16, Fact::Bits), "16");
        assert_eq!(f(IntKind::I32, Fact::Lo), "-2147483648n");
        assert_eq!(f(IntKind::U8, Fact::Hi), "255n");
        assert_eq!(f(IntKind::U32, Fact::Signed), "false");
        assert_eq!(int_fact(IntKind::I32, Fact::Go), None);
    }

    #[test]
    fn a_wrap_is_what_wrap_expr_prints_around_the_expression() {
        for k in [IntKind::I8, IntKind::U8, IntKind::I16, IntKind::U16, IntKind::I32, IntKind::U32, IntKind::I64, IntKind::U64, IntKind::Isize, IntKind::Usize] {
            let wrapped = format!("{}x + y{}", int_fact(k, Fact::WrapOpen).unwrap(), int_fact(k, Fact::WrapClose).unwrap());
            assert_eq!(wrapped, num::wrap_expr(k, "x + y"), "{k:?}");
        }
    }
}

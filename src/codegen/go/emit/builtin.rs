//! Built-in std method / constructor emission for Go (helpers live in `wj_core.go`).

use super::names::{go_string, ident};
use super::num;
use super::Go;
use crate::codegen::portable::template::{self, Operands, TypeRef};
use crate::stdlib_surface::lowering::{CoreType, Fact, Guard, Hook, Modifier, Rule, Target, Widen};
use crate::codegen::portable::ir::*;
use crate::codegen::portable::ty::{IntKind, Ty};

impl<'m> Go<'m> {
    pub fn method(&mut self, recv: &PExpr, name: &str, args: &[PExpr], res: &Resolution, ty: &Ty) -> PResult<String> {
        if let Resolution::User { owner } = res {
            return self.user_method(recv, owner, name, args);
        }
        if name == "to_string" {
            return self.display(recv, &FmtSpec::default());
        }
        // A length typed `int` (SPEC-NUM-014) converts the `uint` length.
        if matches!(name, "len" | "count" | "capacity") {
            if let Ty::Int(k) = ty {
                if *k != IntKind::Usize {
                    let n = self.method(recv, name, args, res, &Ty::usize())?;
                    return Ok(format!("{}({n})", super::types::int_name(*k)));
                }
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
        match &recv.ty {
            Ty::Vec(_) | Ty::Iter(_) | Ty::Range(_) => self.vec_method(recv, name, &a, args, ty),
            Ty::Str => self.lowered(CoreType::String, recv, name, &a, args, ty),
            Ty::Char => self.lowered(CoreType::Char, recv, name, &a, args, ty),
            Ty::Int(_) => self.lowered(CoreType::Integer, recv, name, &a, args, ty),
            Ty::F64 | Ty::F32 => self.lowered(CoreType::Float, recv, name, &a, args, ty),
            Ty::Option(_) => self.opt_method(recv, name, &a),
            Ty::Result(..) => self.res_method(recv, name, &a),
            Ty::Map(..) => self.map_method(recv, name, &a),
            Ty::Set(_) => self.set_method(recv, name, &a),
            other => self.unsupported(format!("method `{name}` on `{}`", other.describe())),
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

    fn vec_method(&mut self, recv: &PExpr, name: &str, a: &[String], args: &[PExpr], ty: &Ty) -> PResult<String> {
        let r = self.rv(recv)?;
        let elem = recv.ty.elem().unwrap_or(Ty::Unknown);
        let mutating = matches!(name, "push" | "push_back" | "pop" | "pop_back" | "insert" | "remove" | "swap_remove" | "clear" | "dedup" | "extend" | "append" | "extend_from_slice" | "truncate" | "retain" | "drain" | "split_off" | "pop_front" | "push_front");
        let p = if mutating { self.addr(recv)? } else { String::new() };
        Ok(match name {
            "len" | "capacity" | "count" => format!("uint(len({r}))"),
            "is_empty" => format!("(len({r}) == 0)"),
            "push" | "push_back" => format!("wjPush({p}, {})", a[0]),
            "push_front" => format!("wjPushFront({p}, {})", a[0]),
            "pop" | "pop_back" => format!("wjPop({p})"),
            "pop_front" => format!("wjPopFront({p})"),
            "get" => format!("wjGet({r}, {})", a[0]),
            "first" => format!("wjFirst({r})"),
            "last" => format!("wjLast({r})"),
            "contains" => if self.is_prim(&elem) { format!("wjContains({r}, {})", a[0]) } else { format!("wjContainsEq({r}, {})", a[0]) },
            "insert" => format!("wjInsert({p}, {}, {})", a[0], a[1]),
            "remove" => format!("wjRemove({p}, {})", a[0]),
            "swap_remove" => format!("wjSwapRemove({p}, {})", a[0]),
            "clear" => format!("wjClear({p})"),
            "sort" | "sort_unstable" if self.is_prim(&elem) && elem != Ty::Bool => format!("wjSort({r})"),
            "sort" | "sort_unstable" => format!("wjSortAny({r})"),
            "reverse" => format!("wjReverseInPlace({r})"),
            "dedup" => format!("wjDedup({p})"),
            "extend" | "append" | "extend_from_slice" => format!("wjExtend({p}, {})", a[0]),
            "truncate" => format!("wjTruncate({p}, {})", a[0]),
            "swap" => format!("wjSwap({r}, {}, {})", a[0], a[1]),
            "iter" | "iter_mut" | "into_iter" | "peekable" => r,
            "drain" => format!("wjDrain({p})"),
            "windows" => format!("wjWindows({r}, {})", a[0]),
            "chunks" => format!("wjChunks({r}, {})", a[0]),
            "to_vec" | "cloned" | "copied" => format!("wjClone({r})"),
            "join" => format!("wjJoin({r}, {})", a[0]),
            "binary_search" => format!("wjBsearch({r}, {})", a[0]),
            "retain" => format!("wjRetain({p}, {})", a[0]),
            "split_off" => format!("wjSplitOff({p}, {})", a[0]),
            "split_at" => format!("wjSplitAt({r}, {})", a[0]),
            "fill" => format!("wjFill({r}, {})", a[0]),
            "map" => format!("wjMap({r}, {})", a[0]),
            "filter" => format!("wjFilter({r}, {})", a[0]),
            "for_each" => format!("wjForEach({r}, {})", a[0]),
            "filter_map" => format!("wjFilterMap({r}, {})", a[0]),
            "flat_map" => format!("wjFlatMap({r}, {})", a[0]),
            "enumerate" => format!("wjEnumerate({r})"),
            "zip" => format!("wjZip({r}, {})", a[0]),
            "rev" => format!("wjRev({r})"),
            "take" => format!("wjTake({r}, {})", a[0]),
            "skip" => format!("wjSkip({r}, {})", a[0]),
            "step_by" => format!("wjStepBy({r}, {})", a[0]),
            "take_while" => format!("wjTakeWhile({r}, {})", a[0]),
            "skip_while" => format!("wjSkipWhile({r}, {})", a[0]),
            "chain" => format!("wjChain({r}, {})", a[0]),
            "collect" => self.collect(&r, ty)?,
            "sum" => format!("wjSum({r})"),
            "product" => format!("wjProduct({r})"),
            "fold" => format!("wjFold({r}, {}, {})", a[0], a[1]),
            "any" => format!("wjAny({r}, {})", a[0]),
            "all" => format!("wjAll({r}, {})", a[0]),
            "min" => format!("wjMinOf({r})"),
            "max" => format!("wjMaxOf({r})"),
            "find" => format!("wjFind({r}, {})", a[0]),
            "position" => format!("wjPosition({r}, {})", a[0]),
            "next" => format!("wjFirst({r})"),
            "nth" => format!("wjGet({r}, {})", a[0]),
            _ => {
                let _ = args;
                return self.unsupported(format!("Vec/iterator method `{name}`"));
            }
        })
    }

    fn collect(&self, r: &str, ty: &Ty) -> PResult<String> {
        Ok(match ty {
            Ty::Set(_) => format!("wjToSet({r})"),
            Ty::Map(..) => format!("wjToMap({r})"),
            Ty::Str => format!("wjCollectStr({r})"),
            Ty::Result(..) => format!("wjCollectResult({r})"),
            Ty::Option(_) => format!("wjCollectOption({r})"),
            _ => format!("wjClone({r})"),
        })
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
                let out = template::render_emit(template, &row.params, &mut GoOperands { receiver: &r, place, args: a, arg_exprs: args, widen, recv: &recv.ty, result: ty })?;
                Ok(if widen.is_some() && row.returns_receiver { format!("float32({out})") } else { out })
            }
            Some(Rule::Assign(_)) => self.unsupported(format!("{} method `{name}`: `assign` is a JavaScript place update", ct.noun())),
            Some(Rule::Hook(Hook::Display)) => self.display(recv, &FmtSpec::default()),
            Some(Rule::Unsupported(why)) => self.unsupported(format!("{} method `{name}`: {why}", ct.noun())),
            None => self.unsupported(format!("{} method `{name}` has no Go rule", ct.noun())),
        }
    }

    fn opt_method(&mut self, recv: &PExpr, name: &str, a: &[String]) -> PResult<String> {
        let r = self.rv(recv)?;
        Ok(match name {
            "is_some" => format!("{r}.Ok"),
            "is_none" => format!("(!{r}.Ok)"),
            "unwrap" => format!("wjUnwrapOpt({r})"),
            "expect" => format!("wjExpectOpt({r}, {})", a[0]),
            "unwrap_or" => format!("wjUnwrapOr({r}, {})", a[0]),
            "unwrap_or_else" => format!("wjOptOrElse({r}, {})", a[0]),
            "map" => format!("wjOptMap({r}, {})", a[0]),
            "and_then" => format!("wjOptAndThen({r}, {})", a[0]),
            "filter" => format!("wjOptFilter({r}, {})", a[0]),
            "ok_or" => format!("wjOkOr({r}, {})", a[0]),
            "map_or" => format!("wjOptMapOr({r}, {}, {})", a[0], a[1]),
            "is_some_and" => format!("wjOptIsSomeAnd({r}, {})", a[0]),
            _ => return self.unsupported(format!("Option method `{name}`")),
        })
    }

    fn res_method(&mut self, recv: &PExpr, name: &str, a: &[String]) -> PResult<String> {
        let r = self.rv(recv)?;
        Ok(match name {
            "is_ok" => format!("{r}.Ok"),
            "is_err" => format!("(!{r}.Ok)"),
            "unwrap" => format!("wjUnwrapRes({r})"),
            "expect" => format!("wjExpectRes({r}, {})", a[0]),
            "unwrap_err" => format!("wjUnwrapErr({r})"),
            "unwrap_or" => format!("wjResUnwrapOr({r}, {})", a[0]),
            "unwrap_or_else" => format!("wjResOrElse({r}, {})", a[0]),
            "map" => format!("wjResMap({r}, {})", a[0]),
            "map_err" => format!("wjResMapErr({r}, {})", a[0]),
            "and_then" => format!("wjResAndThen({r}, {})", a[0]),
            "ok" => format!("wjResOk({r})"),
            "err" => format!("wjResErr({r})"),
            _ => return self.unsupported(format!("Result method `{name}`")),
        })
    }

    fn map_method(&mut self, recv: &PExpr, name: &str, a: &[String]) -> PResult<String> {
        let r = self.rv(recv)?;
        Ok(match name {
            "len" => format!("uint(len({r}))"),
            "is_empty" => format!("(len({r}) == 0)"),
            "insert" => format!("wjMapInsert({r}, {}, {})", a[0], a[1]),
            "get" | "get_mut" => format!("wjMapGet({r}, {})", a[0]),
            "contains_key" => format!("wjHas({r}, {})", a[0]),
            "remove" => format!("wjMapRemove({r}, {})", a[0]),
            "clear" => format!("clear({r})"),
            "keys" | "into_keys" => format!("wjKeys({r})"),
            "values" | "values_mut" | "into_values" => format!("wjValues({r})"),
            "iter" | "iter_mut" | "into_iter" => format!("wjEntries({r})"),
            _ => return self.unsupported(format!("HashMap method `{name}`")),
        })
    }

    fn set_method(&mut self, recv: &PExpr, name: &str, a: &[String]) -> PResult<String> {
        let r = self.rv(recv)?;
        Ok(match name {
            "len" => format!("uint(len({r}))"),
            "is_empty" => format!("(len({r}) == 0)"),
            "insert" => format!("wjSetInsert({r}, {})", a[0]),
            "contains" => format!("wjHas({r}, {})", a[0]),
            "remove" => format!("wjSetRemove({r}, {})", a[0]),
            "clear" => format!("clear({r})"),
            "iter" | "into_iter" => format!("wjKeys({r})"),
            _ => return self.unsupported(format!("HashSet method `{name}`")),
        })
    }

    pub fn builtin_static(&mut self, owner: &str, name: &str, a: &str, args: &[PExpr], ty: &Ty) -> PResult<String> {
        if let Some(c) = self.ordering_const(owner, name) {
            return Ok(c);
        }
        if let Some(k) = IntKind::from_name(owner) {
            let (lo, hi) = num::range_of(k);
            return match name {
                "MAX" => Ok(num::lit_int(hi, k)),
                "MIN" => Ok(num::lit_int(lo, k)),
                "from" => Ok(num::cast(&args[0].ty, &Ty::Int(k), a)),
                "from_str_radix" => Ok(format!("wjParseIntRadix[{}]({a})", super::types::int_name(k))),
                _ => self.unsupported(format!("`{owner}::{name}`")),
            };
        }
        match (owner, name) {
            ("Vec", "new" | "with_capacity") | ("HashMap" | "BTreeMap" | "HashSet" | "BTreeSet", "new" | "with_capacity") => Ok(format!("{}{{}}", self.ty(ty)?)),
            ("Vec", "from") => Ok(format!("wjClone({a})")),
            ("String" | "string", "new" | "with_capacity") => Ok("\"\"".into()),
            ("String" | "string", "from") => Ok(a.to_string()),
            ("f32" | "f64", m) if crate::codegen::portable::builtins::float_const(owner, m).is_some() => {
                Ok(num::float_lit(crate::codegen::portable::builtins::float_const(owner, m).unwrap_or(0.0), owner == "f32"))
            }
            ("f64", "from") => Ok(num::cast(&args[0].ty, &Ty::F64, a)),
            ("char", "from_u32") => Ok(format!("wjCharFromU32({a})")),
            _ => {
                let _ = go_string("");
                self.unsupported(format!("`{owner}::{name}`"))
            }
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

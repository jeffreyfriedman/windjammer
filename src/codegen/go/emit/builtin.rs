//! Built-in std method / constructor emission for Go (helpers live in `wj_core.go`).

use super::names::{go_string, ident};
use super::num;
use super::Go;
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
            Ty::Str => self.str_method(recv, name, &a, args, ty),
            Ty::Char => self.char_method(recv, name, &a),
            Ty::Int(k) => self.int_method(recv, *k, name, &a),
            Ty::F64 | Ty::F32 => self.float_method(recv, name, &a),
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

    fn str_method(&mut self, recv: &PExpr, name: &str, a: &[String], args: &[PExpr], ty: &Ty) -> PResult<String> {
        let r = self.rv(recv)?;
        let arg_str = |i: usize| -> String {
            if matches!(args.get(i).map(|e| &e.ty), Some(Ty::Char)) { format!("string(rune({}))", a[i]) } else { a[i].clone() }
        };
        Ok(match name {
            "len" => format!("uint(len({r}))"),
            "is_empty" => format!("(len({r}) == 0)"),
            "to_uppercase" | "to_ascii_uppercase" => format!("wjUpper({r})"),
            "to_lowercase" | "to_ascii_lowercase" => format!("wjLower({r})"),
            "trim" => format!("wjTrim({r})"),
            "trim_start" => format!("wjTrimStart({r})"),
            "trim_end" => format!("wjTrimEnd({r})"),
            "as_str" | "to_owned" | "into_string" | "clone" => r,
            "contains" => format!("wjStrContains({r}, {})", arg_str(0)),
            "starts_with" => format!("wjStrStarts({r}, {})", arg_str(0)),
            "ends_with" => format!("wjStrEnds({r}, {})", arg_str(0)),
            "replace" => format!("wjStrReplace({r}, {}, {})", arg_str(0), arg_str(1)),
            "split" => format!("wjStrSplit({r}, {})", arg_str(0)),
            "chars" => format!("wjChars({r})"),
            "bytes" | "as_bytes" => format!("wjBytes({r})"),
            "lines" => format!("wjLines({r})"),
            "split_whitespace" => format!("wjSplitWs({r})"),
            "char_indices" => format!("wjCharIndices({r})"),
            "find" => format!("wjStrFind({r}, {})", arg_str(0)),
            "repeat" => format!("wjStrRepeat({r}, {})", a[0]),
            "substring" => format!("wjStrSlice({r}, {}, {}, false)", a[0], a[1]),
            "push" => format!("wjPushStr({}, string(rune({})))", self.addr(recv)?, a[0]),
            "push_str" => format!("wjPushStr({}, {})", self.addr(recv)?, a[0]),
            "clear" => format!("wjClearStr({})", self.addr(recv)?),
            "parse" => match ty {
                Ty::Result(ok, _) => match &**ok {
                    Ty::Int(k) => format!("wjParseInt[{}]({r})", super::types::int_name(*k)),
                    Ty::F64 => format!("wjParseFloat({r})"),
                    Ty::F32 => format!("wjParseFloat32({r})"),
                    Ty::Bool => format!("wjParseBool({r})"),
                    other => return self.unsupported(format!("parse::<{}>", other.describe())),
                },
                _ => return self.unsupported("parse without a target type"),
            },
            _ => return self.unsupported(format!("string method `{name}`")),
        })
    }

    fn char_method(&mut self, recv: &PExpr, name: &str, a: &[String]) -> PResult<String> {
        let r = self.rv(recv)?;
        Ok(match name {
            "is_alphabetic" => format!("wjIsAlphabetic({r})"),
            "is_numeric" => format!("wjIsNumeric({r})"),
            "is_alphanumeric" => format!("(wjIsAlphabetic({r}) || wjIsNumeric({r}))"),
            "is_whitespace" => format!("wjIsWhitespace({r})"),
            "is_uppercase" => format!("wjIsUpper({r})"),
            "is_lowercase" => format!("wjIsLower({r})"),
            "is_ascii_digit" => format!("({r} >= '0' && {r} <= '9')"),
            "is_ascii_alphabetic" => format!("(({r} >= 'a' && {r} <= 'z') || ({r} >= 'A' && {r} <= 'Z'))"),
            "is_ascii_alphanumeric" => format!("(({r} >= 'a' && {r} <= 'z') || ({r} >= 'A' && {r} <= 'Z') || ({r} >= '0' && {r} <= '9'))"),
            "is_ascii_uppercase" => format!("({r} >= 'A' && {r} <= 'Z')"),
            "is_ascii_lowercase" => format!("({r} >= 'a' && {r} <= 'z')"),
            "is_ascii" => format!("({r} < 128)"),
            "to_ascii_uppercase" => format!("wjAsciiUpper({r})"),
            "to_ascii_lowercase" => format!("wjAsciiLower({r})"),
            "to_uppercase" => format!("wjUpper(string(rune({r})))"),
            "to_lowercase" => format!("wjLower(string(rune({r})))"),
            "to_digit" => format!("wjToDigit({r}, {})", a[0]),
            "len_utf8" => format!("uint(len(string(rune({r}))))"),
            _ => return self.unsupported(format!("char method `{name}`")),
        })
    }

    fn int_method(&mut self, recv: &PExpr, k: IntKind, name: &str, a: &[String]) -> PResult<String> {
        let r = self.rv(recv)?;
        let (lo, hi) = num::range_of(k);
        let (lo_s, hi_s) = (num::lit_int(lo, k), num::lit_int(hi, k));
        Ok(match name {
            "abs" => format!("wjAbs({r})"),
            "pow" => format!("wjPow({r}, {})", a[0]),
            "min" => format!("wjMin2({r}, {})", a[0]),
            "max" => format!("wjMax2({r}, {})", a[0]),
            "clamp" => format!("wjMin2(wjMax2({r}, {}), {})", a[0], a[1]),
            "signum" => format!("wjSignum({r})"),
            "wrapping_add" => format!("wjWAdd({r}, {})", a[0]),
            "wrapping_sub" => format!("wjWSub({r}, {})", a[0]),
            "wrapping_mul" => format!("wjWMul({r}, {})", a[0]),
            "wrapping_neg" => format!("wjWNeg({r})"),
            "wrapping_shl" => format!("({r} << (uint({}) % {}))", a[0], k.bits()),
            "wrapping_shr" => format!("({r} >> (uint({}) % {}))", a[0], k.bits()),
            "saturating_add" => format!("wjSatAdd({r}, {})", a[0]),
            "saturating_sub" => format!("wjSatSub({r}, {})", a[0]),
            "saturating_mul" => format!("wjSatMul({r}, {})", a[0]),
            "checked_add" => format!("wjCheckedAdd({r}, {})", a[0]),
            "checked_sub" => format!("wjCheckedSub({r}, {})", a[0]),
            "checked_mul" => format!("wjCheckedMul({r}, {})", a[0]),
            "checked_div" => format!("wjCheckedDiv({r}, {})", a[0]),
            "checked_rem" => format!("wjCheckedRem({r}, {})", a[0]),
            "rem_euclid" => format!("wjRemEuclid({r}, {})", a[0]),
            "div_euclid" => format!("wjDivEuclid({r}, {})", a[0]),
            "abs_diff" => format!("wjAbsDiff({r}, {})", a[0]),
            "count_ones" => format!("wjPopcnt({r})"),
            "leading_zeros" => format!("wjClz({r})"),
            "trailing_zeros" => format!("wjCtz({r})"),
            "rotate_left" => format!("wjRotl({r}, {})", a[0]),
            "rotate_right" => format!("wjRotr({r}, {})", a[0]),
            "is_positive" => format!("({r} > 0)"),
            "is_negative" => format!("({r} < 0)"),
            "is_power_of_two" => format!("(wjPopcnt({r}) == 1)"),
            _ => {
                let _ = (lo_s, hi_s);
                return self.unsupported(format!("integer method `{name}`"));
            }
        })
    }

    fn float_method(&mut self, recv: &PExpr, name: &str, a: &[String]) -> PResult<String> {
        let r = self.rv(recv)?;
        let is32 = recv.ty == Ty::F32;
        let w = |s: String| if is32 { format!("float32({s})") } else { s };
        let f = |x: &str| if is32 { format!("float64({x})") } else { x.to_string() };
        let rr = f(&r);
        Ok(match name {
            "sqrt" | "abs" | "floor" | "ceil" | "trunc" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "cbrt" => w(format!("wjMath_{name}({rr})")),
            "round" => w(format!("wjRound({rr})")),
            "fract" => w(format!("({rr} - wjMath_trunc({rr}))")),
            "powi" => w(format!("wjMath_pow({rr}, float64({}))", a[0])),
            "powf" => w(format!("wjMath_pow({rr}, {})", f(&a[0]))),
            "ln" => w(format!("wjMath_log({rr})")),
            "log10" => w(format!("wjMath_log10({rr})")),
            "log2" => w(format!("wjMath_log2({rr})")),
            "min" => w(format!("wjFmin({rr}, {})", f(&a[0]))),
            "max" => w(format!("wjFmax({rr}, {})", f(&a[0]))),
            "clamp" => w(format!("wjFmin(wjFmax({rr}, {}), {})", f(&a[0]), f(&a[1]))),
            "signum" => w(format!("wjFsignum({rr})")),
            "atan2" => w(format!("wjMath_atan2({rr}, {})", f(&a[0]))),
            "hypot" => w(format!("wjMath_hypot({rr}, {})", f(&a[0]))),
            "is_nan" => format!("wjIsNaN({rr})"),
            "is_infinite" => format!("wjIsInf({rr})"),
            "is_finite" => format!("wjIsFinite({rr})"),
            "is_sign_negative" => format!("wjSignNeg({rr})"),
            "is_sign_positive" => format!("(!wjSignNeg({rr}))"),
            "to_degrees" => w(format!("({rr} * 180 / wjPi)")),
            "to_radians" => w(format!("({rr} * wjPi / 180)")),
            "mul_add" => w(format!("({rr} * {} + {})", f(&a[0]), f(&a[1]))),
            _ => return self.unsupported(format!("float method `{name}`")),
        })
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
            ("f64", "from") => Ok(num::cast(&args[0].ty, &Ty::F64, a)),
            ("char", "from_u32") => Ok(format!("wjCharFromU32({a})")),
            _ => {
                let _ = go_string("");
                self.unsupported(format!("`{owner}::{name}`"))
            }
        }
    }
}

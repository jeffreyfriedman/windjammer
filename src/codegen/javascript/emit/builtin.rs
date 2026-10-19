//! Built-in std method / constructor emission for JavaScript.

use super::num;
use super::Js;
use crate::codegen::portable::ir::*;
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
        // A length typed as a narrow `int` (SPEC-NUM-014) is a Number, not a BigInt.
        if matches!(name, "len" | "count" | "capacity") {
            if let Ty::Int(k) = ty {
                if !k.is_wide() {
                    let n = self.method(recv, name, args, res, &Ty::usize())?;
                    return Ok(format!("Number({n})"));
                }
            }
        }
        let r = self.expr(recv)?;
        let a: Vec<String> = args.iter().map(|e| self.expr(e)).collect::<PResult<_>>()?;
        if name == "to_string" {
            return self.display(recv, &FmtSpec::default());
        }
        if let Some(s) = self.ordering_method(recv, name, &r, &a) {
            return Ok(s);
        }
        match &recv.ty {
            Ty::Vec(t) => self.vec_method(recv, t, name, &r, &a, args, ty),
            Ty::Iter(t) | Ty::Range(t) => self.iter_method(t, name, &r, &a, args, ty),
            Ty::Str => self.str_method(recv, name, &r, &a, ty),
            Ty::Char => self.char_method(name, &r, &a),
            Ty::Int(k) => self.int_method(*k, name, &r, &a),
            Ty::F64 | Ty::F32 => self.float_method(name, &r, &a),
            Ty::Option(_) => self.option_method(name, &r, &a),
            Ty::Result(_, e) => self.result_method(name, &r, &a, e),
            Ty::Map(k, v) => self.map_method(k, v, name, &r, &a),
            Ty::Set(_) => self.set_method(name, &r, &a),
            other => self.unsupported(format!("method `{name}` on `{}`", other.describe())),
        }
    }

    fn expr_recv(&mut self, e: &PExpr) -> PResult<String> {
        let s = self.expr(e)?;
        Ok(if matches!(&*e.kind, PK::Local(_) | PK::Field(..) | PK::TupleIdx(..) | PK::Call { .. } | PK::Method { .. } | PK::Index(..) | PK::Copy(_)) { s } else { format!("({s})") })
    }

    #[allow(clippy::too_many_arguments)]
    fn vec_method(&mut self, recv: &PExpr, t: &Ty, name: &str, r: &str, a: &[String], args: &[PExpr], ty: &Ty) -> PResult<String> {
        Ok(match name {
            "len" => format!("BigInt({r}.length)"),
            "capacity" => format!("BigInt({r}.length)"),
            "is_empty" => format!("({r}.length === 0)"),
            "push" | "push_back" => format!("{r}.push({})", a[0]),
            "push_front" => format!("{r}.unshift({})", a[0]),
            "pop" | "pop_back" => format!("$pop({r})"),
            "pop_front" => format!("$popFront({r})"),
            "get" => format!("$vget({r}, {})", a[0]),
            "first" => format!("$vget({r}, 0n)"),
            "last" => format!("$vlast({r})"),
            "contains" => if self.is_prim(t) { format!("{r}.includes({})", a[0]) } else { format!("{r}.some((x) => $eq(x, {}))", a[0]) },
            "insert" => format!("$vinsert({r}, {}, {})", a[0], a[1]),
            "remove" => format!("$vremove({r}, {})", a[0]),
            "swap_remove" => format!("$vswapRemove({r}, {})", a[0]),
            "clear" => format!("({r}.length = 0)"),
            "sort" | "sort_unstable" => format!("$sort({r})"),
            "reverse" => format!("{r}.reverse()"),
            "dedup" => format!("$dedup({r})"),
            "extend" | "append" | "extend_from_slice" => format!("$extend({r}, {})", a[0]),
            "truncate" => format!("$truncate({r}, {})", a[0]),
            "swap" => format!("$swap({r}, {}, {})", a[0], a[1]),
            "iter" | "iter_mut" | "into_iter" => r.to_string(),
            "drain" => format!("$drain({r})"),
            "windows" => format!("$windows({r}, {})", a[0]),
            "chunks" => format!("$chunks({r}, {})", a[0]),
            "to_vec" => self.clone_js(&recv.ty, r),
            "join" => format!("{r}.join({})", a[0]),
            "binary_search" => format!("$bsearch({r}, {})", a[0]),
            "retain" => format!("$retain({r}, {})", a[0]),
            "split_off" => format!("$splitOff({r}, {})", a[0]),
            "split_at" => format!("$splitAt({r}, {})", a[0]),
            "fill" => format!("{r}.fill({})", a[0]),
            _ => return self.iter_method(t, name, r, a, args, ty),
        })
    }

    fn sum_op(&self, t: &Ty, name: &str) -> PResult<(String, String)> {
        let mul = name == "product";
        match t {
            Ty::Int(k) => {
                let (z, o) = (num::zero(*k), num::one(*k));
                Ok((format!("(a, x) => {}(a {} x)", num::chk(*k), if mul { "*" } else { "+" }), if mul { o } else { z }))
            }
            Ty::F64 | Ty::F32 => Ok((format!("(a, x) => a {} x", if mul { "*" } else { "+" }), if mul { "1".into() } else { "0".into() })),
            other => self.unsupported(format!("{name}() over `{}`", other.describe())),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn iter_method(&mut self, t: &Ty, name: &str, r: &str, a: &[String], args: &[PExpr], ty: &Ty) -> PResult<String> {
        Ok(match name {
            "iter" | "iter_mut" | "into_iter" | "peekable" => r.to_string(),
            "map" => format!("{r}.map((x) => ({})(x))", a[0]),
            "filter" => format!("{r}.filter((x) => ({})(x))", a[0]),
            "for_each" => format!("{r}.forEach((x) => ({})(x))", a[0]),
            "inspect" => format!("{r}.map((x) => (({})(x), x))", a[0]),
            "filter_map" => format!("$filterMap({r}, {})", a[0]),
            "flat_map" => format!("{r}.flatMap((x) => ({})(x))", a[0]),
            "enumerate" => format!("{r}.map((x, i) => [BigInt(i), x])"),
            "zip" => format!("$zip({r}, {})", a[0]),
            "rev" => format!("{r}.slice().reverse()"),
            "take" => format!("{r}.slice(0, Number({}))", a[0]),
            "skip" => format!("{r}.slice(Number({}))", a[0]),
            "step_by" => format!("{r}.filter((_, i) => i % Number({}) === 0)", a[0]),
            "take_while" => format!("$takeWhile({r}, {})", a[0]),
            "skip_while" => format!("$skipWhile({r}, {})", a[0]),
            "chain" => format!("{r}.concat({})", a[0]),
            "cloned" | "copied" => match self.cloner(t) {
                None => format!("{r}.slice()"),
                Some(c) => format!("{r}.map({c})"),
            },
            "collect" => self.collect(r, ty),
            "sum" | "product" => {
                let (f, z) = self.sum_op(ty, name)?;
                format!("{r}.reduce({f}, {z})")
            }
            "count" => format!("BigInt({r}.length)"),
            "fold" => format!("{r}.reduce((acc, x) => ({})(acc, x), {})", a[1], a[0]),
            "any" => format!("{r}.some((x) => ({})(x))", a[0]),
            "all" => format!("{r}.every((x) => ({})(x))", a[0]),
            "min" => format!("$minmax({r}, -1)"),
            "max" => format!("$minmax({r}, 1)"),
            "find" => format!("$find({r}, {})", a[0]),
            "position" => format!("$position({r}, {})", a[0]),
            "last" => format!("$vlast({r})"),
            "next" => format!("$vget({r}, 0n)"),
            "nth" => format!("$vget({r}, {})", a[0]),
            _ => return self.unsupported(format!("iterator method `{name}` ({} args)", args.len())),
        })
    }

    fn collect(&self, r: &str, ty: &Ty) -> String {
        match ty {
            Ty::Set(_) => format!("new Set({r})"),
            Ty::Map(..) => format!("new Map({r})"),
            Ty::Str => format!("{r}.join(\"\")"),
            Ty::Result(..) => format!("$collectResult({r})"),
            Ty::Option(_) => format!("$collectOption({r})"),
            _ => format!("{r}.slice()"),
        }
    }

    fn assign_place(&mut self, recv: &PExpr, f: impl FnOnce(&str) -> String) -> PResult<String> {
        let p = self.place(recv)?;
        let rhs = f(&p);
        Ok(format!("({p} = {rhs}, undefined)"))
    }

    fn str_method(&mut self, recv: &PExpr, name: &str, r: &str, a: &[String], ty: &Ty) -> PResult<String> {
        Ok(match name {
            "len" => format!("BigInt($strlen({r}))"),
            "is_empty" => format!("({r}.length === 0)"),
            "to_uppercase" | "to_ascii_uppercase" => format!("{r}.toUpperCase()"),
            "to_lowercase" | "to_ascii_lowercase" => format!("{r}.toLowerCase()"),
            "trim" => format!("{r}.trim()"),
            "trim_start" => format!("{r}.trimStart()"),
            "trim_end" => format!("{r}.trimEnd()"),
            "as_str" | "to_owned" | "into_string" | "clone" => r.to_string(),
            "contains" => format!("{r}.includes({})", a[0]),
            "starts_with" => format!("{r}.startsWith({})", a[0]),
            "ends_with" => format!("{r}.endsWith({})", a[0]),
            "replace" => format!("{r}.split({}).join({})", a[0], a[1]),
            "split" => format!("$split({r}, {})", a[0]),
            "chars" => format!("Array.from({r})"),
            "bytes" | "as_bytes" => format!("$bytes({r})"),
            "lines" => format!("$lines({r})"),
            "split_whitespace" => format!("{r}.split(/\\s+/).filter((s) => s.length > 0)"),
            "char_indices" => format!("$charIndices({r})"),
            "find" => format!("$strFind({r}, {})", a[0]),
            "repeat" => format!("{r}.repeat(Number({}))", a[0]),
            "substring" => format!("$strSlice({r}, {}, {}, false)", a[0], a[1]),
            "push" | "push_str" => self.assign_place(recv, |p| format!("{p} + {}", a[0]))?,
            "clear" => self.assign_place(recv, |_| "\"\"".to_string())?,
            "parse" => match ty {
                Ty::Result(ok, _) => match &**ok {
                    Ty::Int(k) => {
                        let (lo, hi) = range_of(*k);
                        format!("$parseInt({r}, {lo}n, {hi}n, {})", num::is_wide(*k))
                    }
                    Ty::F64 | Ty::F32 => format!("$parseFloat({r})"),
                    Ty::Bool => format!("$parseBool({r})"),
                    other => return self.unsupported(format!("parse::<{}>", other.describe())),
                },
                _ => return self.unsupported("parse without a target type"),
            },
            _ => return self.unsupported(format!("string method `{name}`")),
        })
    }

    fn char_method(&mut self, name: &str, r: &str, a: &[String]) -> PResult<String> {
        Ok(match name {
            "is_alphabetic" => format!("/^\\p{{L}}$/u.test({r})"),
            "is_numeric" => format!("/^\\p{{N}}$/u.test({r})"),
            "is_alphanumeric" => format!("/^[\\p{{L}}\\p{{N}}]$/u.test({r})"),
            "is_whitespace" => format!("/^\\s$/u.test({r})"),
            "is_uppercase" => format!("/^\\p{{Lu}}$/u.test({r})"),
            "is_lowercase" => format!("/^\\p{{Ll}}$/u.test({r})"),
            "is_ascii_digit" => format!("/^[0-9]$/.test({r})"),
            "is_ascii_alphabetic" => format!("/^[A-Za-z]$/.test({r})"),
            "is_ascii_alphanumeric" => format!("/^[A-Za-z0-9]$/.test({r})"),
            "is_ascii_uppercase" => format!("/^[A-Z]$/.test({r})"),
            "is_ascii_lowercase" => format!("/^[a-z]$/.test({r})"),
            "is_ascii_punctuation" => format!("/^[!-\\/:-@\\[-`{{-~]$/.test({r})"),
            "is_ascii" => format!("({r}.codePointAt(0) < 128)"),
            "to_ascii_uppercase" => format!("$asciiUpper({r})"),
            "to_ascii_lowercase" => format!("$asciiLower({r})"),
            "to_uppercase" => format!("{r}.toUpperCase()"),
            "to_lowercase" => format!("{r}.toLowerCase()"),
            "to_digit" => format!("$toDigit({r}, {})", a[0]),
            "is_digit" => format!("($toDigit({r}, {}) !== null)", a[0]),
            "len_utf8" => format!("BigInt($strlen({r}))"),
            _ => return self.unsupported(format!("char method `{name}`")),
        })
    }

    fn int_method(&mut self, k: IntKind, name: &str, r: &str, a: &[String]) -> PResult<String> {
        let (lo, hi) = range_of(k);
        let (lo_s, hi_s) = (num::lit_int(lo, k), num::lit_int(hi, k));
        let wide = num::is_wide(k);
        let conv = |x: &str| if wide { format!("BigInt({x})") } else { x.to_string() };
        Ok(match name {
            "abs" => format!("({r} < {} ? {}(-{r}) : {r})", num::zero(k), num::chk(k)),
            "pow" => format!("{}({r} ** {})", num::chk(k), conv(&a[0])),
            "min" => format!("$min2({r}, {})", a[0]),
            "max" => format!("$max2({r}, {})", a[0]),
            "clamp" => format!("$min2($max2({r}, {}), {})", a[0], a[1]),
            "signum" => format!("({r} > {z} ? {o} : {r} < {z} ? -{o} : {z})", z = num::zero(k), o = num::one(k)),
            "wrapping_add" => num::wrap_expr(k, &format!("{r} + {}", a[0])),
            "wrapping_sub" => num::wrap_expr(k, &format!("{r} - {}", a[0])),
            "wrapping_mul" => match (k, wide) {
                (IntKind::I32, _) => format!("Math.imul({r}, {})", a[0]),
                (IntKind::U32, _) => format!("(Math.imul({r}, {}) >>> 0)", a[0]),
                _ => num::wrap_expr(k, &format!("{r} * {}", a[0])),
            },
            "wrapping_neg" => num::wrap_expr(k, &format!("-{r}")),
            "wrapping_shl" => num::wrap_expr(k, &format!("{r} << {}", conv(&format!("(Number({}) % {})", a[0], k.bits())))),
            "wrapping_shr" => format!("({r} >> {})", conv(&format!("(Number({}) % {})", a[0], k.bits()))),
            "saturating_add" => format!("$sat({lo_s}, {hi_s}, {r} + {})", a[0]),
            "saturating_sub" => format!("$sat({lo_s}, {hi_s}, {r} - {})", a[0]),
            "saturating_mul" => format!("$sat({lo_s}, {hi_s}, {r} * {})", a[0]),
            "checked_add" => format!("$chkd({lo_s}, {hi_s}, {r} + {})", a[0]),
            "checked_sub" => format!("$chkd({lo_s}, {hi_s}, {r} - {})", a[0]),
            "checked_mul" => format!("$chkd({lo_s}, {hi_s}, {r} * {})", a[0]),
            "checked_div" => format!("({} === {z} ? null : $chkd({lo_s}, {hi_s}, {}))", a[0], if wide { format!("{r} / {}", a[0]) } else { format!("Math.trunc({r} / {})", a[0]) }, z = num::zero(k)),
            "checked_rem" => format!("({} === {z} ? null : {{ v: {r} % {} }})", a[0], a[0], z = num::zero(k)),
            "rem_euclid" => format!("$remEuclid({r}, {})", a[0]),
            "div_euclid" => format!("$divEuclid({r}, {})", a[0]),
            "abs_diff" => format!("({r} > {} ? {r} - {} : {} - {r})", a[0], a[0], a[0]),
            "count_ones" => format!("$popcnt({r}, {})", k.bits()),
            "leading_zeros" => format!("$clz({r}, {})", k.bits()),
            "trailing_zeros" => format!("$ctz({r}, {})", k.bits()),
            "rotate_left" => format!("$rotl({r}, {}, {}, {})", a[0], k.bits(), k.signed()),
            "rotate_right" => format!("$rotr({r}, {}, {}, {})", a[0], k.bits(), k.signed()),
            "is_positive" => format!("({r} > {})", num::zero(k)),
            "is_negative" => format!("({r} < {})", num::zero(k)),
            "is_power_of_two" => format!("($popcnt({r}, {}) === 1)", k.bits()),
            _ => return self.unsupported(format!("integer method `{name}`")),
        })
    }

    fn float_method(&mut self, name: &str, r: &str, a: &[String]) -> PResult<String> {
        Ok(match name {
            "sqrt" => format!("Math.sqrt({r})"),
            "abs" => format!("Math.abs({r})"),
            "floor" => format!("Math.floor({r})"),
            "ceil" => format!("Math.ceil({r})"),
            "round" => format!("$round({r})"),
            "trunc" => format!("Math.trunc({r})"),
            "fract" => format!("({r} - Math.trunc({r}))"),
            "powi" => format!("Math.pow({r}, Number({}))", a[0]),
            "powf" => format!("Math.pow({r}, {})", a[0]),
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "exp" | "cbrt" => format!("Math.{name}({r})"),
            "ln" => format!("Math.log({r})"),
            "log10" => format!("Math.log10({r})"),
            "log2" => format!("Math.log2({r})"),
            "min" => format!("$fmin({r}, {})", a[0]),
            "max" => format!("$fmax({r}, {})", a[0]),
            "clamp" => format!("$fmin($fmax({r}, {}), {})", a[0], a[1]),
            "signum" => format!("$fsignum({r})"),
            "atan2" => format!("Math.atan2({r}, {})", a[0]),
            "hypot" => format!("Math.hypot({r}, {})", a[0]),
            "is_nan" => format!("Number.isNaN({r})"),
            "is_infinite" => format!("(Math.abs({r}) === Infinity)"),
            "is_finite" => format!("Number.isFinite({r})"),
            "is_sign_negative" => format!("$signNeg({r})"),
            "is_sign_positive" => format!("(!$signNeg({r}))"),
            "to_degrees" => format!("({r} * 180 / Math.PI)"),
            "to_radians" => format!("({r} * Math.PI / 180)"),
            "mul_add" => format!("({r} * {} + {})", a[0], a[1]),
            _ => return self.unsupported(format!("float method `{name}`")),
        })
    }

    fn option_method(&mut self, name: &str, r: &str, a: &[String]) -> PResult<String> {
        Ok(match name {
            "is_some" => format!("({r} !== null)"),
            "is_none" => format!("({r} === null)"),
            "unwrap" => format!("$unwrapOpt({r})"),
            "expect" => format!("$expectOpt({r}, {})", a[0]),
            "unwrap_or" => format!("$unwrapOr({r}, {})", a[0]),
            "unwrap_or_else" => format!("$optOrElse({r}, {})", a[0]),
            "map" => format!("$optMap({r}, {})", a[0]),
            "and_then" => format!("$optAndThen({r}, {})", a[0]),
            "filter" => format!("$optFilter({r}, {})", a[0]),
            "ok_or" => format!("$okOr({r}, {})", a[0]),
            "map_or" => format!("$optMapOr({r}, {}, {})", a[0], a[1]),
            "is_some_and" => format!("({r} !== null && ({})({r}.v))", a[0]),
            _ => return self.unsupported(format!("Option method `{name}`")),
        })
    }

    fn result_method(&mut self, name: &str, r: &str, a: &[String], e: &Ty) -> PResult<String> {
        let dbg = self.dbg_fn(e)?;
        Ok(match name {
            "is_ok" => format!("{r}.ok"),
            "is_err" => format!("(!{r}.ok)"),
            "unwrap" => format!("$unwrapRes({r}, {dbg})"),
            "expect" => format!("$expectRes({r}, {}, {dbg})", a[0]),
            "unwrap_err" => format!("$unwrapErr({r})"),
            "unwrap_or" => format!("({r}.ok ? {r}.v : {})", a[0]),
            "unwrap_or_else" => format!("$resOrElse({r}, {})", a[0]),
            "map" => format!("$resMap({r}, {})", a[0]),
            "map_err" => format!("$resMapErr({r}, {})", a[0]),
            "and_then" => format!("$resAndThen({r}, {})", a[0]),
            "ok" => format!("$resOk({r})"),
            "err" => format!("$resErr({r})"),
            _ => return self.unsupported(format!("Result method `{name}`")),
        })
    }

    fn key_ok(&self, k: &Ty) -> PResult<()> {
        if matches!(k, Ty::Int(_) | Ty::Str | Ty::Char | Ty::Bool) {
            Ok(())
        } else {
            self.unsupported(format!("HashMap/HashSet key type `{}` (only integer, string, char and bool keys)", k.describe()))
        }
    }

    fn map_method(&mut self, k: &Ty, v: &Ty, name: &str, r: &str, a: &[String]) -> PResult<String> {
        self.key_ok(k)?;
        let _ = v;
        Ok(match name {
            "len" => format!("BigInt({r}.size)"),
            "is_empty" => format!("({r}.size === 0)"),
            "insert" => format!("$mapInsert({r}, {}, {})", a[0], a[1]),
            "get" | "get_mut" => format!("$mapGetOpt({r}, {})", a[0]),
            "contains_key" => format!("{r}.has({})", a[0]),
            "remove" => format!("$mapRemove({r}, {})", a[0]),
            "clear" => format!("{r}.clear()"),
            "keys" | "into_keys" => format!("$sortedKeys({r})"),
            "values" | "values_mut" | "into_values" => format!("$sortedValues({r})"),
            "iter" | "iter_mut" | "into_iter" => format!("$sortedEntries({r})"),
            _ => return self.unsupported(format!("HashMap method `{name}`")),
        })
    }

    fn set_method(&mut self, name: &str, r: &str, a: &[String]) -> PResult<String> {
        Ok(match name {
            "len" => format!("BigInt({r}.size)"),
            "is_empty" => format!("({r}.size === 0)"),
            "insert" => format!("$setInsert({r}, {})", a[0]),
            "contains" => format!("{r}.has({})", a[0]),
            "remove" => format!("{r}.delete({})", a[0]),
            "clear" => format!("{r}.clear()"),
            "iter" | "into_iter" => format!("$sortedKeys({r})"),
            _ => return self.unsupported(format!("HashSet method `{name}`")),
        })
    }

    pub fn builtin_static(&mut self, owner: &str, name: &str, a: &str, args: &[PExpr], ty: &Ty) -> PResult<String> {
        if let Some(c) = self.ordering_const(owner, name) {
            return Ok(c);
        }
        if let Some(k) = IntKind::from_name(owner) {
            let (lo, hi) = range_of(k);
            return match name {
                "MAX" => Ok(num::lit_int(hi, k)),
                "MIN" => Ok(num::lit_int(lo, k)),
                "from" => Ok(num::cast(&args[0].ty, &Ty::Int(k), a)),
                "from_str_radix" => Ok(format!("$parseIntRadix({a}, {lo}n, {hi}n, {})", num::is_wide(k))),
                _ => self.unsupported(format!("`{owner}::{name}`")),
            };
        }
        match (owner, name) {
            ("Vec", "new") | ("Vec", "with_capacity") => Ok("[]".into()),
            ("Vec", "from") => Ok(format!("{a}.slice()")),
            ("HashMap" | "BTreeMap", "new" | "with_capacity") => {
                if let Ty::Map(k, _) = ty {
                    self.key_ok(k)?;
                }
                Ok("new Map()".into())
            }
            ("HashSet" | "BTreeSet", "new" | "with_capacity") => Ok("new Set()".into()),
            ("String" | "string", "new" | "with_capacity") => Ok("\"\"".into()),
            ("String" | "string", "from") => Ok(a.to_string()),
            ("f64", "from") => Ok(num::cast(&args[0].ty, &Ty::F64, a)),
            ("char", "from_u32") => Ok(format!("$charFromU32({a})")),
            _ => self.unsupported(format!("`{owner}::{name}`")),
        }
    }
}

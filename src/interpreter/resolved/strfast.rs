//! Strings without the call path: a statement `s.push_str(x)` appends to the local's string in place,
//! as the builtin does (`builtins/text.rs`: the argument a string or a char, references stripped, the
//! string made unique first), without the argument vector and the receiver's take/write-back. Another
//! receiver at run time, a box in use or a lock guard takes the generic call. `s.push_str("${a},${b}")`
//! formats into one buffer and appends that, with no intermediate string value.

use super::compile::{run_all, Cx, S};
use super::expr::{evals, expr};
use super::mapfast::simple;
use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::{semantic, R};
use crate::interpreter::int::IntTy;
use crate::interpreter::place::{deref_value, strip_refs};
use crate::interpreter::value::Value;
use crate::parser::{Expression, Statement};
use std::rc::Rc;

/// The string a local holds directly or through its box (no projection, no lock guard).
fn local_str(v: &mut Value, f: impl FnOnce(&mut Rc<String>)) -> bool {
    match v {
        Value::Str(s) => {
            f(s);
            true
        }
        Value::Ref(p) if p.path.is_empty() && p.guard.is_none() => match p.slot.try_borrow_mut() {
            Ok(mut g) => match &mut *g {
                Value::Str(s) => {
                    f(s);
                    true
                }
                _ => false,
            },
            Err(_) => false,
        },
        _ => false,
    }
}

/// A statement `s.push_str(x)` (its value unused) on a local holding a string.
pub(super) fn push_str_stmt(cx: &mut Cx, s: S) -> Option<Code> {
    let Statement::Expression { expr: call, .. } = s else { return None };
    let Expression::MethodCall { object: Expression::Identifier { name, .. }, method, type_args: None, arguments, location } = call else { return None };
    let loc = location.as_ref();
    let [(None, x)] = arguments.as_slice() else { return None };
    if method != "push_str" || !simple(x) {
        return None;
    }
    let slot = cx.local(name).filter(|s| cx.slot_ty(*s).is_none())?;
    let generic = expr(cx, call)?.code();
    if let Some((pieces, args, floc)) = plain_format(cx, x) {
        return Some(Box::new(move |it, b| {
            if it.fast_off("pushfmt") || !local_str(&mut it.rstack[b + slot], |_| ()) {
                return generic(it, b);
            }
            it.mark(floc);
            // formatted aside first: a failing argument or `Display` leaves the string as it was. The buffer is
            // the interpreter's scratch, taken (a nested statement in an argument finds it empty and makes its
            // own) and given back cleared on every exit; a huge one is not kept.
            let mut out = std::mem::take(&mut it.fmt_scratch);
            out.clear();
            let done = append_plain(it, b, &pieces, &args, &mut out);
            if done.is_ok() {
                it.mark(loc);
                it.fast.pushfmt += 1;
                local_str(&mut it.rstack[b + slot], |s| Rc::make_mut(s).push_str(&out));
            }
            out.clear();
            if out.capacity() <= SCRATCH_KEEP {
                it.fmt_scratch = out;
            }
            done?;
            Ok(Value::Unit)
        }));
    }
    let xc = expr(cx, x)?.code();
    Some(Box::new(move |it, b| {
        if it.fast_off("pushstr") || !local_str(&mut it.rstack[b + slot], |_| ()) {
            return generic(it, b);
        }
        let x = xc(it, b)?;
        it.mark(loc); // as the generic call marks the call once its arguments are evaluated
        // the builtin's `arg_str`: a string or a char, references stripped
        let t: Rc<String> = match deref_value(strip_refs(x)?)? {
            Value::Str(t) => t,
            Value::Char(c) => Rc::new(c.to_string()),
            other => return Err(semantic(format!("`push_str` expects a string, found {}", other.type_name()))),
        };
        it.fast.pushstr += 1;
        local_str(&mut it.rstack[b + slot], |s| Rc::make_mut(s).push_str(&t));
        Ok(Value::Unit)
    }))
}

/// The largest scratch buffer kept between statements (bytes of capacity).
const SCRATCH_KEEP: usize = 1 << 16;

type Loc = Option<&'static crate::source_map::Location>;

/// `format!` (an interpolated string) of a template made only of bare `{}`s: its pieces, the compiled
/// arguments after the template, and its location.
fn plain_format(cx: &mut Cx, e: &'static Expression<'static>) -> Option<(Vec<String>, Vec<Code>, Loc)> {
    let Expression::MacroInvocation { name, args, is_repeat: false, location, .. } = e else { return None };
    let Some(Expression::Literal { value: crate::parser::Literal::String(t), .. }) = args.first() else { return None };
    if name != "format" {
        return None;
    }
    let pieces = crate::interpreter::format::plain_template(t, args.len() - 1)?;
    let args = evals(cx, args[1..].iter().copied())?;
    Some((pieces, args, location.as_ref()))
}

/// The pieces of a plain template joined with the displays of `args`, appended to `out`: every argument
/// first, then the displays (a user `Display` runs after them all), as `format_args` does.
pub(super) fn append_plain(it: &mut Interpreter, b: usize, pieces: &[String], args: &[Code], out: &mut String) -> R<()> {
    let (one, many);
    let vals: &[Value] = match args {
        [a] => {
            one = [a(it, b)?];
            &one
        }
        _ => {
            many = run_all(it, b, args)?;
            &many
        }
    };
    out.push_str(&pieces[0]);
    for (v, piece) in vals.iter().zip(&pieces[1..]) {
        match v {
            // the plain scalars and strings are read in place; any other value is dereferenced as before
            Value::Int(n, t) if *t != IntTy::U128 => push_int(out, *n),
            Value::Str(s) => out.push_str(s),
            Value::Char(c) => out.push(*c),
            v => display_into(it, deref_value(v.clone())?, out)?,
        }
        out.push_str(piece);
    }
    Ok(())
}

/// `display_string`, appended in place for the values that need no allocation of their own.
fn display_into(it: &mut Interpreter, v: Value, out: &mut String) -> R<()> {
    match &v {
        Value::Int(n, t) if *t != IntTy::U128 => push_int(out, *n),
        Value::Str(s) => out.push_str(s),
        Value::Char(c) => out.push(*c),
        _ => out.push_str(&it.display_string(&v)?),
    }
    Ok(())
}

/// `n` in decimal appended to `out`: the bytes `{n}` produces for an `i128`, without `core::fmt`. A
/// sign, then the magnitude's digits two at a time (`u64` arithmetic when it fits, `u128` above).
pub(super) fn push_int(out: &mut String, n: i128) {
    const PAIRS: &[u8; 200] = b"0001020304050607080910111213141516171819202122232425262728293031323334353637383940414243444546474849\
5051525354555657585960616263646566676869707172737475767778798081828384858687888990919293949596979899";
    let mut buf = [0u8; 40];
    let mut at = buf.len();
    let mut m = n.unsigned_abs();
    while m > u64::MAX as u128 {
        let (q, r) = (m / 100, (m % 100) as usize);
        at -= 2;
        buf[at..at + 2].copy_from_slice(&PAIRS[r * 2..r * 2 + 2]);
        m = q;
    }
    let mut m = m as u64;
    while m >= 100 {
        let r = (m % 100) as usize;
        m /= 100;
        at -= 2;
        buf[at..at + 2].copy_from_slice(&PAIRS[r * 2..r * 2 + 2]);
    }
    if m >= 10 {
        at -= 2;
        buf[at..at + 2].copy_from_slice(&PAIRS[m as usize * 2..m as usize * 2 + 2]);
    } else {
        at -= 1;
        buf[at] = b'0' + m as u8;
    }
    if n < 0 {
        at -= 1;
        buf[at] = b'-';
    }
    if let Ok(digits) = std::str::from_utf8(&buf[at..]) {
        out.push_str(digits);
    }
}

#[cfg(test)]
mod tests {
    use super::push_int;
    use std::fmt::Write;

    fn same(n: i128) {
        let (mut a, mut b) = (String::from("x"), String::from("x"));
        push_int(&mut a, n);
        write!(b, "{n}").unwrap();
        assert_eq!(a, b, "{n}");
    }

    #[test]
    fn boundaries_match_core_fmt() {
        let mut vals = vec![0i128, 1, -1, 9, 10, 11, 99, 100, 101, -9, -10, -99, -100, i128::MIN, i128::MAX, i128::MIN + 1, i128::MAX - 1];
        for bits in [8u32, 16, 32, 64, 128] {
            let (lo, hi) = (-(1i128 << (bits - 1).min(126)) , (1i128 << (bits - 1).min(126)));
            for d in -2..=2 {
                vals.extend([lo + d, hi + d, (hi - 1) + d]);
            }
        }
        for t in [i8::MIN as i128, i8::MAX as i128, i16::MIN as i128, i16::MAX as i128, i32::MIN as i128, i32::MAX as i128, i64::MIN as i128, i64::MAX as i128, u8::MAX as i128, u16::MAX as i128, u32::MAX as i128, u64::MAX as i128, u64::MAX as i128 + 1, u64::MAX as i128 - 1] {
            vals.extend([t - 1, t, t + 1, -t]);
        }
        let mut p = 1i128;
        while let Some(next) = p.checked_mul(10) {
            vals.extend([p - 1, p, p + 1, -p, -(p - 1), -(p + 1)]);
            p = next;
        }
        for v in vals {
            same(v);
        }
    }

    #[test]
    fn random_sample_matches_core_fmt() {
        // xorshift with a fixed seed; each draw is shifted to every magnitude from 1 to 128 bits
        let mut x: u128 = 0x9E37_79B9_7F4A_7C15_F39C_C060_5CED_C834;
        for i in 0..200_000u32 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let v = (x >> (i % 128)) as i128;
            same(v);
            same(-v);
        }
    }
}

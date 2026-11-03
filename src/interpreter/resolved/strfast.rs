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
            // formatted aside first: a failing argument or `Display` leaves the string as it was
            let mut out = String::new();
            append_plain(it, b, &pieces, &args, &mut out)?;
            it.mark(loc);
            it.fast.pushfmt += 1;
            local_str(&mut it.rstack[b + slot], |s| Rc::make_mut(s).push_str(&out));
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
        display_into(it, deref_value(v.clone())?, out)?;
        out.push_str(piece);
    }
    Ok(())
}

/// `display_string`, appended in place for the values that need no allocation of their own.
fn display_into(it: &mut Interpreter, v: Value, out: &mut String) -> R<()> {
    use std::fmt::Write;
    match &v {
        Value::Int(n, t) if *t != IntTy::U128 => {
            let _ = write!(out, "{n}");
        }
        Value::Str(s) => out.push_str(s),
        Value::Char(c) => out.push(*c),
        _ => out.push_str(&it.display_string(&v)?),
    }
    Ok(())
}

//! Strings without the call path: a statement `s.push_str(x)` appends to the local's string in place,
//! as the builtin does (`builtins/text.rs`: the argument a string or a char, references stripped, the
//! string made unique first), without the argument vector and the receiver's take/write-back. Another
//! receiver at run time, a box in use or a lock guard takes the generic call.

use super::compile::{Cx, S};
use super::expr::expr;
use super::mapfast::simple;
use super::Code;
use crate::interpreter::error::semantic;
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

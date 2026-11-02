//! Maps without the call path: `match m.get(&k) { .. }` looks the key up in place (`MapGet`, used by
//! `matching.rs`) and a statement `m.insert(k, x)` writes into the local's map directly. Both are what
//! the builtins do (`builtins/map.rs`: keys and values with references stripped, `Key` equality, the
//! map made unique before writing) without the argument vector, the reference `&k` makes, the `Option`
//! built for the result and the receiver's take/write-back. Anything else (another receiver at run
//! time, types with `Drop`) goes through the generic call. User methods never take a map's builtin
//! method's place (`find_user_method` looks only at structs and enums), so none is checked for.

use super::compile::{Cx, E, S};
use super::expr::expr;
use super::Code;
use crate::interpreter::engine::Interpreter;
use crate::interpreter::error::R;
use crate::interpreter::place::strip_refs;
use crate::interpreter::value::{Key, Value};
use crate::parser::{Expression, Statement, UnaryOp};
use std::rc::Rc;

/// `m.get(k)` on a map local, looked up in place.
pub(super) struct MapGet {
    map: Code,
    key: Code,
}

impl MapGet {
    /// `Ok(None)`: the receiver is not a map (the generic call decides).
    pub(super) fn run(&self, it: &mut Interpreter, b: usize) -> R<Option<Option<Value>>> {
        let m = (self.map)(it, b)?;
        let Value::Map(km) = &m else { return Ok(None) };
        let k = strip_refs((self.key)(it, b)?)?;
        Ok(Some(km.get(&Key(k)).cloned()))
    }
}

/// `m.get(k)` / `m.get(&k)` with `m` and `k` locals or literals (nothing to evaluate twice).
pub(super) fn map_get(cx: &mut Cx, value: E) -> Option<MapGet> {
    let Expression::MethodCall { object: o @ Expression::Identifier { .. }, method, type_args: None, arguments, .. } = value else { return None };
    let [(None, arg)] = arguments.as_slice() else { return None };
    let key = match arg {
        Expression::Unary { op: UnaryOp::Ref, operand, .. } => *operand,
        a => *a,
    };
    if method != "get" || !matches!(key, Expression::Identifier { .. } | Expression::Literal { .. }) {
        return None;
    }
    Some(MapGet { map: expr(cx, o)?.code(), key: expr(cx, key)?.code() })
}

/// An expression that cannot rebind a local or call anything: locals, literals, references to them,
/// and operators over those (so the receiver's slot is the same before and after it runs).
fn simple(e: E) -> bool {
    match e {
        Expression::Identifier { .. } | Expression::Literal { .. } => true,
        Expression::Unary { operand, .. } => simple(operand),
        Expression::Binary { left, right, .. } => simple(left) && simple(right),
        Expression::FieldAccess { object, .. } => simple(object),
        _ => false,
    }
}

/// The map a local holds directly or through its box (no projection, no lock guard).
fn local_map(v: &mut Value, f: impl FnOnce(&mut Rc<crate::interpreter::kmap::KMap>)) -> bool {
    match v {
        Value::Map(km) => {
            f(km);
            true
        }
        Value::Ref(p) if p.path.is_empty() && p.guard.is_none() => match p.slot.try_borrow_mut() {
            Ok(mut g) => match &mut *g {
                Value::Map(km) => {
                    f(km);
                    true
                }
                _ => false,
            },
            Err(_) => false,
        },
        _ => false,
    }
}

/// A statement `m.insert(k, x)` (its value unused) on a local holding a map.
pub(super) fn insert_stmt(cx: &mut Cx, s: S) -> Option<Code> {
    let Statement::Expression { expr: call, .. } = s else { return None };
    let Expression::MethodCall { object: Expression::Identifier { name, .. }, method, type_args: None, arguments, .. } = call else { return None };
    let [(None, k), (None, x)] = arguments.as_slice() else { return None };
    if method != "insert" || !cx.it.drop_types.is_empty() || !simple(k) || !simple(x) {
        return None;
    }
    let slot = cx.local(name).filter(|s| cx.slot_ty(*s).is_none())?;
    let generic = expr(cx, call)?.code();
    let (kc, xc) = (expr(cx, k)?.code(), expr(cx, x)?.code());
    Some(Box::new(move |it, b| {
        // another receiver (a `Vec`'s `insert(i, x)`...), or a box in use: the generic call
        if !local_map(&mut it.rstack[b + slot], |_| ()) {
            return generic(it, b);
        }
        let k = strip_refs(kc(it, b)?)?;
        let x = strip_refs(xc(it, b)?)?;
        // test-only mutant (debug builds): a map another binding shares is not written (no copy)
        let skip_shared = cfg!(debug_assertions) && it.jit_mutant.as_deref() == Some("mapins");
        local_map(&mut it.rstack[b + slot], |km| {
            if !(skip_shared && Rc::strong_count(km) > 1) {
                // the old value (if any) is dropped here, as the builtin's `Option` would have been
                Rc::make_mut(km).insert(Key(k), x);
            }
        });
        Ok(Value::Unit)
    }))
}

//! `Option`, `Result`, `Ordering`, universal methods (`clone`, `to_string`, `cmp`, ...) and `Formatter`.

use super::*;
use std::cmp::Ordering;

fn payload(v: &Value) -> Option<Value> {
    match v {
        Value::Enum(e) => match &e.payload {
            Payload::Tuple(p) => p.first().cloned(),
            _ => None,
        },
        _ => None,
    }
}

fn is_some(v: &Value) -> bool {
    matches!(v, Value::Enum(e) if &*e.ty == "Option" && e.idx == 1)
}

fn is_ok(v: &Value) -> bool {
    matches!(v, Value::Enum(e) if &*e.ty == "Result" && e.idx == 0)
}

fn call(it: &mut Interpreter, f: &Value, args: Vec<Value>) -> R {
    it.call_value(f, args)
}

fn truthy(v: &Value) -> bool {
    matches!(v, Value::Bool(true))
}

pub fn register(r: &mut Registry) {
    // ---------------- Option ----------------
    r.pure(Kind::Option, "is_some", |_, v, _| Ok(Value::Bool(is_some(v))));
    r.pure(Kind::Option, "is_none", |_, v, _| Ok(Value::Bool(!is_some(v))));
    r.pure(Kind::Option, "is_some_and", |it, v, a| {
        if !is_some(v) {
            return Ok(Value::Bool(false));
        }
        let f = arg(&a, 0, "is_some_and")?;
        Ok(Value::Bool(truthy(&call(it, &f, vec![payload(v).unwrap()])?)))
    });
    r.pure(Kind::Option, "unwrap", |_, v, _| {
        payload(v).filter(|_| is_some(v)).ok_or_else(|| panic_ctl("called `Option::unwrap()` on a `None` value"))
    });
    r.pure(Kind::Option, "expect", |_, v, a| {
        payload(v).filter(|_| is_some(v)).ok_or_else(|| panic_ctl(arg_str(&a, 0, "expect").map(|s| (*s).clone()).unwrap_or_default()))
    });
    r.pure(Kind::Option, "unwrap_or", |_, v, a| if is_some(v) { Ok(payload(v).unwrap()) } else { arg(&a, 0, "unwrap_or") });
    r.pure(Kind::Option, "unwrap_or_else", |it, v, a| {
        if is_some(v) {
            Ok(payload(v).unwrap())
        } else {
            let f = arg(&a, 0, "unwrap_or_else")?;
            call(it, &f, vec![])
        }
    });
    r.pure(Kind::Option, "unwrap_or_default", |it, v, _| {
        if is_some(v) {
            Ok(payload(v).unwrap())
        } else {
            match it.type_hint.take() {
                Some(t) => it.default_for(&t),
                None => Ok(Value::int(0)),
            }
        }
    });
    r.pure(Kind::Option, "map", |it, v, a| {
        if !is_some(v) {
            return Ok(Value::none());
        }
        let f = arg(&a, 0, "map")?;
        Ok(Value::some(call(it, &f, vec![payload(v).unwrap()])?))
    });
    r.pure(Kind::Option, "map_or", |it, v, a| {
        if !is_some(v) {
            return arg(&a, 0, "map_or");
        }
        let f = arg(&a, 1, "map_or")?;
        call(it, &f, vec![payload(v).unwrap()])
    });
    r.pure(Kind::Option, "map_or_else", |it, v, a| {
        if !is_some(v) {
            let d = arg(&a, 0, "map_or_else")?;
            return call(it, &d, vec![]);
        }
        let f = arg(&a, 1, "map_or_else")?;
        call(it, &f, vec![payload(v).unwrap()])
    });
    r.pure(Kind::Option, "and_then", |it, v, a| {
        if !is_some(v) {
            return Ok(Value::none());
        }
        let f = arg(&a, 0, "and_then")?;
        call(it, &f, vec![payload(v).unwrap()])
    });
    r.pure(Kind::Option, "and", |_, v, a| if is_some(v) { arg(&a, 0, "and") } else { Ok(Value::none()) });
    r.pure(Kind::Option, "or", |_, v, a| if is_some(v) { Ok(v.clone()) } else { arg(&a, 0, "or") });
    r.pure(Kind::Option, "or_else", |it, v, a| {
        if is_some(v) {
            Ok(v.clone())
        } else {
            let f = arg(&a, 0, "or_else")?;
            call(it, &f, vec![])
        }
    });
    r.pure(Kind::Option, "xor", |_, v, a| {
        let o = arg(&a, 0, "xor")?;
        Ok(match (is_some(v), is_some(&o)) {
            (true, false) => v.clone(),
            (false, true) => o,
            _ => Value::none(),
        })
    });
    r.pure(Kind::Option, "filter", |it, v, a| {
        if !is_some(v) {
            return Ok(Value::none());
        }
        let f = arg(&a, 0, "filter")?;
        Ok(if truthy(&call(it, &f, vec![payload(v).unwrap()])?) { v.clone() } else { Value::none() })
    });
    r.pure(Kind::Option, "ok_or", |_, v, a| if is_some(v) { Ok(Value::ok(payload(v).unwrap())) } else { Ok(Value::err(arg(&a, 0, "ok_or")?)) });
    r.pure(Kind::Option, "ok_or_else", |it, v, a| {
        if is_some(v) {
            Ok(Value::ok(payload(v).unwrap()))
        } else {
            let f = arg(&a, 0, "ok_or_else")?;
            Ok(Value::err(call(it, &f, vec![])?))
        }
    });
    r.pure(Kind::Option, "zip", |_, v, a| {
        let o = arg(&a, 0, "zip")?;
        Ok(if is_some(v) && is_some(&o) { Value::some(Value::tuple(vec![payload(v).unwrap(), payload(&o).unwrap()])) } else { Value::none() })
    });
    r.pure(Kind::Option, "flatten", |_, v, _| if is_some(v) { Ok(payload(v).unwrap()) } else { Ok(Value::none()) });
    for name in ["as_ref", "as_deref", "cloned", "copied", "as_slice"] {
        r.pure(Kind::Option, name, |_, v, _| Ok(v.clone()));
    }
    r.pure(Kind::Option, "iter", |_, v, _| Ok(new_iter(if is_some(v) { vec![payload(v).unwrap()] } else { vec![] })));
    r.pure(Kind::Option, "into_iter", |_, v, _| Ok(new_iter(if is_some(v) { vec![payload(v).unwrap()] } else { vec![] })));
    r.pure(Kind::Option, "unzip", |_, v, _| {
        if let Some(Value::Tuple(t)) = payload(v).filter(|_| is_some(v)) {
            return Ok(Value::tuple(vec![Value::some(t[0].clone()), Value::some(t[1].clone())]));
        }
        Ok(Value::tuple(vec![Value::none(), Value::none()]))
    });
    r.mutating(Kind::Option, "take", |_, v, _| Ok(std::mem::replace(v, Value::none())));
    r.mutating(Kind::Option, "replace", |_, v, a| Ok(std::mem::replace(v, Value::some(arg(&a, 0, "replace")?))));
    r.mutating(Kind::Option, "insert", |_, v, a| {
        *v = Value::some(arg(&a, 0, "insert")?);
        Ok(Value::Unit)
    });
    r.place(Kind::Option, "as_mut", |_, p, _| {
        let cur = p.read()?;
        Ok(if is_some(&cur) { Value::some(Value::Ref(p.proj(Proj::Index(0)))) } else { Value::none() })
    });
    r.place(Kind::Option, "get_or_insert_with", |it, p, a| {
        let cur = p.read()?;
        if !is_some(&cur) {
            let f = arg(&a, 0, "get_or_insert_with")?;
            let x = call(it, &f, vec![])?;
            p.write(Value::some(x))?;
        }
        Ok(Value::Ref(p.proj(Proj::Index(0))))
    });

    // ---------------- Result ----------------
    r.pure(Kind::Result, "is_ok", |_, v, _| Ok(Value::Bool(is_ok(v))));
    r.pure(Kind::Result, "is_err", |_, v, _| Ok(Value::Bool(!is_ok(v))));
    r.pure(Kind::Result, "unwrap", |it, v, _| {
        if is_ok(v) {
            return Ok(payload(v).unwrap());
        }
        let e = it.debug_string(&payload(v).unwrap_or(Value::Unit))?;
        Err(panic_ctl(format!("called `Result::unwrap()` on an `Err` value: {}", e)))
    });
    r.pure(Kind::Result, "expect", |it, v, a| {
        if is_ok(v) {
            return Ok(payload(v).unwrap());
        }
        let e = it.debug_string(&payload(v).unwrap_or(Value::Unit))?;
        Err(panic_ctl(format!("{}: {}", arg_str(&a, 0, "expect")?, e)))
    });
    r.pure(Kind::Result, "unwrap_err", |it, v, _| {
        if !is_ok(v) {
            return Ok(payload(v).unwrap());
        }
        let e = it.debug_string(&payload(v).unwrap_or(Value::Unit))?;
        Err(panic_ctl(format!("called `Result::unwrap_err()` on an `Ok` value: {}", e)))
    });
    r.pure(Kind::Result, "expect_err", |it, v, a| {
        if !is_ok(v) {
            return Ok(payload(v).unwrap());
        }
        let e = it.debug_string(&payload(v).unwrap_or(Value::Unit))?;
        Err(panic_ctl(format!("{}: {}", arg_str(&a, 0, "expect_err")?, e)))
    });
    r.pure(Kind::Result, "unwrap_or", |_, v, a| if is_ok(v) { Ok(payload(v).unwrap()) } else { arg(&a, 0, "unwrap_or") });
    r.pure(Kind::Result, "unwrap_or_else", |it, v, a| {
        if is_ok(v) {
            Ok(payload(v).unwrap())
        } else {
            let f = arg(&a, 0, "unwrap_or_else")?;
            call(it, &f, vec![payload(v).unwrap()])
        }
    });
    r.pure(Kind::Result, "unwrap_or_default", |it, v, _| {
        if is_ok(v) {
            Ok(payload(v).unwrap())
        } else {
            match it.type_hint.take() {
                Some(t) => it.default_for(&t),
                None => Ok(Value::int(0)),
            }
        }
    });
    r.pure(Kind::Result, "map", |it, v, a| {
        if !is_ok(v) {
            return Ok(v.clone());
        }
        let f = arg(&a, 0, "map")?;
        Ok(Value::ok(call(it, &f, vec![payload(v).unwrap()])?))
    });
    r.pure(Kind::Result, "map_err", |it, v, a| {
        if is_ok(v) {
            return Ok(v.clone());
        }
        let f = arg(&a, 0, "map_err")?;
        Ok(Value::err(call(it, &f, vec![payload(v).unwrap()])?))
    });
    r.pure(Kind::Result, "map_or", |it, v, a| {
        if !is_ok(v) {
            return arg(&a, 0, "map_or");
        }
        let f = arg(&a, 1, "map_or")?;
        call(it, &f, vec![payload(v).unwrap()])
    });
    r.pure(Kind::Result, "and_then", |it, v, a| {
        if !is_ok(v) {
            return Ok(v.clone());
        }
        let f = arg(&a, 0, "and_then")?;
        call(it, &f, vec![payload(v).unwrap()])
    });
    r.pure(Kind::Result, "or_else", |it, v, a| {
        if is_ok(v) {
            return Ok(v.clone());
        }
        let f = arg(&a, 0, "or_else")?;
        call(it, &f, vec![payload(v).unwrap()])
    });
    r.pure(Kind::Result, "ok", |_, v, _| Ok(if is_ok(v) { Value::some(payload(v).unwrap()) } else { Value::none() }));
    r.pure(Kind::Result, "err", |_, v, _| Ok(if !is_ok(v) { Value::some(payload(v).unwrap()) } else { Value::none() }));
    r.pure(Kind::Result, "is_ok_and", |it, v, a| {
        if !is_ok(v) {
            return Ok(Value::Bool(false));
        }
        let f = arg(&a, 0, "is_ok_and")?;
        Ok(Value::Bool(truthy(&call(it, &f, vec![payload(v).unwrap()])?)))
    });
    for name in ["as_ref", "as_deref", "cloned", "copied"] {
        r.pure(Kind::Result, name, |_, v, _| Ok(v.clone()));
    }
    r.pure(Kind::Result, "iter", |_, v, _| Ok(new_iter(if is_ok(v) { vec![payload(v).unwrap()] } else { vec![] })));

    // ---------------- Ordering ----------------
    r.pure(Kind::Ordering, "reverse", |_, v, _| Ok(ordering(ordering_of(v).unwrap().reverse())));
    r.pure(Kind::Ordering, "then", |_, v, a| {
        let o = ordering_of(v).unwrap();
        Ok(if o == Ordering::Equal { arg(&a, 0, "then")? } else { v.clone() })
    });
    r.pure(Kind::Ordering, "then_with", |it, v, a| {
        let o = ordering_of(v).unwrap();
        if o == Ordering::Equal {
            let f = arg(&a, 0, "then_with")?;
            call(it, &f, vec![])
        } else {
            Ok(v.clone())
        }
    });
    r.pure(Kind::Ordering, "is_lt", |_, v, _| Ok(Value::Bool(ordering_of(v) == Some(Ordering::Less))));
    r.pure(Kind::Ordering, "is_le", |_, v, _| Ok(Value::Bool(ordering_of(v) != Some(Ordering::Greater))));
    r.pure(Kind::Ordering, "is_gt", |_, v, _| Ok(Value::Bool(ordering_of(v) == Some(Ordering::Greater))));
    r.pure(Kind::Ordering, "is_ge", |_, v, _| Ok(Value::Bool(ordering_of(v) != Some(Ordering::Less))));
    r.pure(Kind::Ordering, "is_eq", |_, v, _| Ok(Value::Bool(ordering_of(v) == Some(Ordering::Equal))));
    r.pure(Kind::Ordering, "is_ne", |_, v, _| Ok(Value::Bool(ordering_of(v) != Some(Ordering::Equal))));

    // ---------------- Formatter ----------------
    r.pure(Kind::Formatter, "write_str", |_, v, a| {
        if let Value::Formatter(f) = v {
            f.borrow_mut().push_str(&arg_str(&a, 0, "write_str")?);
        }
        Ok(Value::ok(Value::Unit))
    });
    r.pure(Kind::Formatter, "write_char", |_, v, a| {
        if let (Value::Formatter(f), Value::Char(c)) = (v, arg(&a, 0, "write_char")?) {
            f.borrow_mut().push(c);
        }
        Ok(Value::ok(Value::Unit))
    });

    // ---------------- Any ----------------
    r.pure(Kind::Any, "clone", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Any, "to_owned", |_, v, _| Ok(v.clone()));
    r.pure(Kind::Any, "into", |it, v, _| match it.type_hint.take() {
        Some(t) => it.coerce(v.clone(), &t),
        None => Ok(v.clone()),
    });
    r.pure(Kind::Any, "try_into", |it, v, _| match (it.type_hint.take(), v) {
        (Some(t), Value::Int(n, _)) => match IntTy::from_type(&t) {
            Some(ty) if ty.fits(*n) => Ok(Value::ok(Value::Int(*n, ty))),
            Some(_) => Ok(Value::err(Value::str("out of range integral type conversion attempted"))),
            None => Ok(Value::ok(v.clone())),
        },
        _ => Ok(Value::ok(v.clone())),
    });
    for name in ["borrow", "borrow_mut", "as_ref", "as_mut", "deref", "to_vec", "unwrap_ref", "iter_ref", "lock_unwrap"] {
        r.pure(Kind::Any, name, |_, v, _| Ok(v.clone()));
    }
    r.pure(Kind::Any, "to_string", |it, v, _| Ok(Value::str(it.display_string(v)?)));
    r.pure(Kind::Any, "eq", |it, v, a| it.binary(crate::parser::BinaryOp::Eq, v.clone(), arg(&a, 0, "eq")?));
    r.pure(Kind::Any, "ne", |it, v, a| it.binary(crate::parser::BinaryOp::Ne, v.clone(), arg(&a, 0, "ne")?));
    r.pure(Kind::Any, "lt", |it, v, a| it.binary(crate::parser::BinaryOp::Lt, v.clone(), arg(&a, 0, "lt")?));
    r.pure(Kind::Any, "le", |it, v, a| it.binary(crate::parser::BinaryOp::Le, v.clone(), arg(&a, 0, "le")?));
    r.pure(Kind::Any, "gt", |it, v, a| it.binary(crate::parser::BinaryOp::Gt, v.clone(), arg(&a, 0, "gt")?));
    r.pure(Kind::Any, "ge", |it, v, a| it.binary(crate::parser::BinaryOp::Ge, v.clone(), arg(&a, 0, "ge")?));
    r.pure(Kind::Any, "cmp", |_, v, a| Ok(ordering(v.structural_cmp(&arg(&a, 0, "cmp")?))));
    r.pure(Kind::Any, "partial_cmp", |_, v, a| {
        let o = arg(&a, 0, "partial_cmp")?;
        if let (Value::Float(x, _), Value::Float(y, _)) = (v, &o) {
            return Ok(match x.partial_cmp(y) {
                Some(c) => Value::some(ordering(c)),
                None => Value::none(),
            });
        }
        Ok(Value::some(ordering(v.structural_cmp(&o))))
    });
    r.pure(Kind::Any, "min", |_, v, a| {
        let o = arg(&a, 0, "min")?;
        Ok(if o.structural_cmp(v) == Ordering::Less { o } else { v.clone() })
    });
    r.pure(Kind::Any, "max", |_, v, a| {
        let o = arg(&a, 0, "max")?;
        Ok(if o.structural_cmp(v) == Ordering::Less { v.clone() } else { o })
    });
    r.pure(Kind::Any, "hash", |_, _, _| Ok(Value::Unit));
}

/// ADR-0018 `a?.b`: `a.__opt_chain(|_oc| _oc.b)`, an `and_then` that lifts a non-optional result into `Some`.
pub(super) fn register_chain(r: &mut Registry) {
    r.pure(Kind::Option, "__opt_chain", |it, v, a| {
        if !is_some(v) {
            return Ok(Value::none());
        }
        let f = arg(&a, 0, "?.")?;
        let out = call(it, &f, vec![payload(v).unwrap()])?;
        Ok(if out.as_option().is_some() { out } else { Value::some(out) })
    });
}

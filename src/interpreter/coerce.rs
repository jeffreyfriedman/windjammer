//! Binding-site coercions (`let x: u8 = 5`, parameter and return types) and `as` casts.

use super::engine::Interpreter;
use super::error::{semantic, unsupported, R};
use super::int::IntTy;
use super::value::*;
use crate::parser::Type;
use std::rc::Rc;

impl Interpreter {
    fn resolve_alias<'t>(&'t self, t: &'t Type) -> &'t Type {
        if self.type_aliases.is_empty() {
            return t;
        }
        let mut cur = t;
        for _ in 0..8 {
            if let Type::Custom(n) = cur {
                if let Some(a) = self.type_aliases.get(n) {
                    cur = a;
                    continue;
                }
            }
            break;
        }
        cur
    }

    /// Give literal-typed numbers the width of the type they are bound to.
    pub(crate) fn coerce(&self, v: Value, ty: &Type) -> R {
        let ty = self.resolve_alias(ty);
        match (&v, ty) {
            (Value::Int(i, it), _) => {
                if let Some(target) = IntTy::from_type(ty) {
                    if *it == IntTy::Lit && !target.fits(*i) {
                        if *i < 0 && !target.signed() && target.bits() == 64 {
                            // literals >= 2^63 are lexed as wrapped i64
                            return Ok(Value::Int(target.wrap(*i), target));
                        }
                        return Err(semantic(format!(
                            "literal out of range for `{}`: {}",
                            target.name(),
                            i
                        )));
                    }
                    return Ok(Value::Int(*i, target));
                }
                if let Some(ft) = type_is_float(ty) {
                    if *it == IntTy::Lit {
                        return Ok(Value::Float(*i as f64, ft));
                    }
                }
                Ok(v)
            }
            (Value::Float(f, _), _) => {
                if let Some(ft) = type_is_float(ty) {
                    let f = if ft == FloatTy::F32 { *f as f32 as f64 } else { *f };
                    return Ok(Value::Float(f, ft));
                }
                Ok(v)
            }
            (Value::Enum(e), Type::Option(inner)) if &*e.ty == "Option" => {
                if let Payload::Tuple(p) = &e.payload {
                    if let Some(x) = p.first() {
                        if needs_coerce(x) {
                            return Ok(Value::some(self.coerce(x.clone(), inner)?));
                        }
                    }
                }
                Ok(v)
            }
            (Value::Enum(e), Type::Result(ok, er)) if &*e.ty == "Result" => {
                if let Payload::Tuple(p) = &e.payload {
                    if let Some(x) = p.first() {
                        if needs_coerce(x) {
                            return Ok(if e.idx == 0 {
                                Value::ok(self.coerce(x.clone(), ok)?)
                            } else {
                                Value::err(self.coerce(x.clone(), er)?)
                            });
                        }
                    }
                }
                Ok(v)
            }
            (Value::Vec(items), Type::Vec(inner)) | (Value::Vec(items), Type::Array(inner, _)) => {
                if items.first().map(needs_coerce).unwrap_or(false) {
                    let mut out = Vec::with_capacity(items.len());
                    for x in items.iter() {
                        out.push(self.coerce(x.clone(), inner)?);
                    }
                    return Ok(Value::Vec(Rc::new(out)));
                }
                Ok(v)
            }
            (Value::Tuple(items), Type::Tuple(tys)) if items.len() == tys.len() => {
                if items.iter().any(needs_coerce) {
                    let mut out = Vec::with_capacity(items.len());
                    for (x, t) in items.iter().zip(tys.iter()) {
                        out.push(self.coerce(x.clone(), t)?);
                    }
                    return Ok(Value::tuple(out));
                }
                Ok(v)
            }
            (Value::Vec(items), Type::Parameterized(n, args)) if n == "Vec" && args.len() == 1 => {
                if items.first().map(needs_coerce).unwrap_or(false) {
                    let mut out = Vec::with_capacity(items.len());
                    for x in items.iter() {
                        out.push(self.coerce(x.clone(), &args[0])?);
                    }
                    return Ok(Value::Vec(Rc::new(out)));
                }
                Ok(v)
            }
            (Value::Vec(items), Type::String) => super::builtins::collect_into(items.to_vec(), Some(ty)),
            (Value::Vec(items), Type::Result(..)) | (Value::Vec(items), Type::Option(..)) => {
                super::builtins::collect_into(items.to_vec(), Some(ty))
            }
            (Value::Vec(items), Type::Parameterized(n, _)) | (Value::Vec(items), Type::Custom(n))
                if matches!(
                    n.rsplit("::").next().unwrap_or(n),
                    "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet"
                ) =>
            {
                super::builtins::collect_into(items.to_vec(), Some(ty))
            }
            (_, Type::Reference(inner)) | (_, Type::MutableReference(inner)) => {
                if matches!(v, Value::Ref(_)) {
                    Ok(v)
                } else {
                    self.coerce(v, inner)
                }
            }
            _ => Ok(v),
        }
    }

    /// `expr as ty`
    pub(crate) fn cast(&self, v: Value, ty: &Type) -> R {
        let v = super::place::deref_value(v)?;
        let ty = self.resolve_alias(ty);
        if let Some(target) = IntTy::from_type(ty) {
            let n: i128 = match &v {
                Value::Int(i, _) => *i,
                Value::Float(f, _) => {
                    if f.is_nan() {
                        0
                    } else if target == IntTy::U128 {
                        // saturating float -> u128 over the full range (bit pattern storage)
                        f.trunc().max(0.0).min(u128::MAX as f64) as u128 as i128
                    } else {
                        let t = f.trunc();
                        let (lo, hi) = (target.min() as f64, target.max() as f64);
                        if t <= lo {
                            target.min()
                        } else if t >= hi {
                            target.max()
                        } else {
                            t as i128
                        }
                    }
                }
                Value::Bool(b) => *b as i128,
                Value::Char(c) => *c as i128,
                Value::Enum(e) => e.idx as i128,
                other => return Err(unsupported(format!("cast of {} to {}", other.type_name(), target.name()))),
            };
            return Ok(Value::Int(target.wrap(n), target));
        }
        if let Some(ft) = type_is_float(ty) {
            let f = match &v {
                Value::Int(i, IntTy::U128) => *i as u128 as f64,
                Value::Int(i, _) => *i as f64,
                Value::Float(f, _) => *f,
                other => return Err(unsupported(format!("cast of {} to float", other.type_name()))),
            };
            let f = if ft == FloatTy::F32 { f as f32 as f64 } else { f };
            return Ok(Value::Float(f, ft));
        }
        match (ty, &v) {
            (Type::Custom(n), Value::Int(i, _)) if n == "char" => {
                Ok(Value::Char(char::from_u32(*i as u32).ok_or_else(|| semantic("invalid char cast"))?))
            }
            (Type::Custom(n), Value::Char(_)) if n == "char" => Ok(v),
            (Type::Bool, Value::Bool(_)) => Ok(v),
            _ => Ok(v), // casts between reference/pointer-ish and identical types are no-ops
        }
    }
}

fn needs_coerce(v: &Value) -> bool {
    match v {
        Value::Int(_, IntTy::Lit) => true,
        Value::Float(_, FloatTy::Lit) => true,
        Value::Tuple(t) => t.iter().any(needs_coerce),
        Value::Vec(t) => t.first().map(needs_coerce).unwrap_or(false),
        Value::Enum(e) => match &e.payload {
            Payload::Tuple(p) => p.iter().any(needs_coerce),
            _ => false,
        },
        _ => false,
    }
}

//! Operators with Rust semantics: checked integer arithmetic (panic on overflow),
//! IEEE floats, string concatenation, comparison, and user operator overloading.

use super::engine::Interpreter;
use super::error::{panic_ctl, unsupported, R};
use super::int::IntTy;
use super::value::*;
use crate::parser::{BinaryOp, CompoundOp};
use std::cmp::Ordering;
use std::rc::Rc;

fn op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "add",
        BinaryOp::Sub => "subtract",
        BinaryOp::Mul => "multiply",
        BinaryOp::Div => "divide",
        BinaryOp::Mod => "calculate the remainder",
        BinaryOp::Shl => "shift left",
        BinaryOp::Shr => "shift right",
        _ => "operate",
    }
}

pub fn compound_to_binary(op: CompoundOp) -> BinaryOp {
    match op {
        CompoundOp::Add => BinaryOp::Add,
        CompoundOp::Sub => BinaryOp::Sub,
        CompoundOp::Mul => BinaryOp::Mul,
        CompoundOp::Div => BinaryOp::Div,
        CompoundOp::Mod => BinaryOp::Mod,
        CompoundOp::BitAnd => BinaryOp::BitAnd,
        CompoundOp::BitOr => BinaryOp::BitOr,
        CompoundOp::BitXor => BinaryOp::BitXor,
        CompoundOp::Shl => BinaryOp::Shl,
        CompoundOp::Shr => BinaryOp::Shr,
    }
}

pub fn int_binary(op: BinaryOp, a: i128, at: IntTy, b: i128, bt: IntTy) -> R {
    // shifts keep the left type; others unify literal widths
    let ty = if matches!(op, BinaryOp::Shl | BinaryOp::Shr) { at } else { IntTy::unify(at, bt) };
    let overflow = || panic_ctl(format!("attempt to {} with overflow", op_name(op)));
    let check = |r: Option<i128>| -> R {
        let r = r.ok_or_else(overflow)?;
        if ty.fits(r) {
            Ok(Value::Int(r, ty))
        } else {
            Err(overflow())
        }
    };
    match op {
        BinaryOp::Add => check(a.checked_add(b)),
        BinaryOp::Sub => check(a.checked_sub(b)),
        BinaryOp::Mul => check(a.checked_mul(b)),
        BinaryOp::Div => {
            if b == 0 {
                return Err(panic_ctl("attempt to divide by zero"));
            }
            check(a.checked_div(b))
        }
        BinaryOp::Mod => {
            if b == 0 {
                return Err(panic_ctl("attempt to calculate the remainder with a divisor of zero"));
            }
            check(a.checked_rem(b))
        }
        BinaryOp::BitAnd => Ok(Value::Int(a & b, ty)),
        BinaryOp::BitOr => Ok(Value::Int(a | b, ty)),
        BinaryOp::BitXor => Ok(Value::Int(a ^ b, ty)),
        BinaryOp::Shl | BinaryOp::Shr => {
            if b < 0 || b >= ty.bits() as i128 {
                return Err(overflow());
            }
            let r = if op == BinaryOp::Shl { a << b } else { a >> b };
            Ok(Value::Int(if op == BinaryOp::Shl { ty.wrap(r) } else { r }, ty))
        }
        BinaryOp::Eq => Ok(Value::Bool(a == b)),
        BinaryOp::Ne => Ok(Value::Bool(a != b)),
        BinaryOp::Lt => Ok(Value::Bool(a < b)),
        BinaryOp::Le => Ok(Value::Bool(a <= b)),
        BinaryOp::Gt => Ok(Value::Bool(a > b)),
        BinaryOp::Ge => Ok(Value::Bool(a >= b)),
        BinaryOp::And | BinaryOp::Or => Err(unsupported("logical operator on integers")),
    }
}

pub fn float_binary(op: BinaryOp, a: f64, at: FloatTy, b: f64, bt: FloatTy) -> R {
    let ty = if at == FloatTy::Lit { bt } else { at };
    let fin = |r: f64| -> Value {
        Value::Float(if ty == FloatTy::F32 { r as f32 as f64 } else { r }, ty)
    };
    Ok(match op {
        BinaryOp::Add => fin(a + b),
        BinaryOp::Sub => fin(a - b),
        BinaryOp::Mul => fin(a * b),
        BinaryOp::Div => fin(a / b),
        BinaryOp::Mod => fin(a % b),
        BinaryOp::Eq => Value::Bool(a == b),
        BinaryOp::Ne => Value::Bool(a != b),
        BinaryOp::Lt => Value::Bool(a < b),
        BinaryOp::Le => Value::Bool(a <= b),
        BinaryOp::Gt => Value::Bool(a > b),
        BinaryOp::Ge => Value::Bool(a >= b),
        _ => return Err(unsupported("bitwise operator on floats")),
    })
}

fn cmp_result(op: BinaryOp, o: Ordering) -> Option<bool> {
    Some(match op {
        BinaryOp::Eq => o == Ordering::Equal,
        BinaryOp::Ne => o != Ordering::Equal,
        BinaryOp::Lt => o == Ordering::Less,
        BinaryOp::Le => o != Ordering::Greater,
        BinaryOp::Gt => o == Ordering::Greater,
        BinaryOp::Ge => o != Ordering::Less,
        _ => return None,
    })
}

fn overload_name(op: BinaryOp) -> Option<&'static str> {
    Some(match op {
        BinaryOp::Add => "add",
        BinaryOp::Sub => "sub",
        BinaryOp::Mul => "mul",
        BinaryOp::Div => "div",
        BinaryOp::Mod => "rem",
        BinaryOp::BitAnd => "bitand",
        BinaryOp::BitOr => "bitor",
        BinaryOp::BitXor => "bitxor",
        BinaryOp::Shl => "shl",
        BinaryOp::Shr => "shr",
        _ => return None,
    })
}

impl Interpreter {
    pub(crate) fn binary(&mut self, op: BinaryOp, l: Value, r: Value) -> R {
        let l = super::place::deref_value(l)?;
        let r = super::place::deref_value(r)?;
        match (&l, &r) {
            (Value::Int(a, at), Value::Int(b, bt)) => int_binary(op, *a, *at, *b, *bt),
            (Value::Float(a, at), Value::Float(b, bt)) => float_binary(op, *a, *at, *b, *bt),
            (Value::Float(a, at), Value::Int(b, IntTy::Lit)) => float_binary(op, *a, *at, *b as f64, *at),
            (Value::Int(a, IntTy::Lit), Value::Float(b, bt)) => float_binary(op, *a as f64, *bt, *b, *bt),
            (Value::Bool(a), Value::Bool(b)) => Ok(match op {
                BinaryOp::BitAnd | BinaryOp::And => Value::Bool(*a & *b),
                BinaryOp::BitOr | BinaryOp::Or => Value::Bool(*a | *b),
                BinaryOp::BitXor => Value::Bool(*a ^ *b),
                BinaryOp::Eq => Value::Bool(a == b),
                BinaryOp::Ne => Value::Bool(a != b),
                BinaryOp::Lt => Value::Bool(a < b),
                BinaryOp::Le => Value::Bool(a <= b),
                BinaryOp::Gt => Value::Bool(a > b),
                BinaryOp::Ge => Value::Bool(a >= b),
                _ => return Err(unsupported("arithmetic on bool")),
            }),
            (Value::Str(a), Value::Str(b)) if op == BinaryOp::Add => {
                let mut s = String::with_capacity(a.len() + b.len());
                s.push_str(a);
                s.push_str(b);
                Ok(Value::Str(Rc::new(s)))
            }
            (Value::Str(a), Value::Char(c)) if op == BinaryOp::Add => {
                let mut s = (**a).clone();
                s.push(*c);
                Ok(Value::Str(Rc::new(s)))
            }
            (Value::Struct(_), _) | (Value::Enum(_), _) if overload_name(op).is_some() => {
                let tn = l.type_name();
                let name = overload_name(op).unwrap();
                if let Some(f) = self.methods.get(&(tn.clone(), name.to_string())).cloned() {
                    return self.call_func(&f, vec![r], Some(l));
                }
                self.compare_or_fail(op, &l, &r)
            }
            _ => self.compare_or_fail(op, &l, &r),
        }
    }

    fn compare_or_fail(&mut self, op: BinaryOp, l: &Value, r: &Value) -> R {
        if matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge) {
            // user PartialEq / PartialOrd / Ord impls take precedence over structure
            if matches!(l, Value::Struct(_) | Value::Enum(_)) {
                let tn = l.type_name();
                if matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
                    if let Some(f) = self.methods.get(&(tn.clone(), "eq".to_string())).cloned() {
                        let v = self.call_func(&f, vec![r.clone()], Some(l.clone()))?;
                        let b = matches!(v, Value::Bool(true));
                        return Ok(Value::Bool(if op == BinaryOp::Eq { b } else { !b }));
                    }
                } else {
                    for m in ["partial_cmp", "cmp"] {
                        if let Some(f) = self.methods.get(&(tn.clone(), m.to_string())).cloned() {
                            let v = self.call_func(&f, vec![r.clone()], Some(l.clone()))?;
                            let ord = match &v {
                                Value::Enum(e) if &*e.ty == "Option" => match &e.payload {
                                    Payload::Tuple(p) => p.first().cloned(),
                                    _ => None,
                                },
                                other => Some(other.clone()),
                            };
                            if let Some(Value::Enum(e)) = ord {
                                let o = match &*e.variant {
                                    "Less" => Ordering::Less,
                                    "Greater" => Ordering::Greater,
                                    _ => Ordering::Equal,
                                };
                                return Ok(Value::Bool(cmp_result(op, o).unwrap()));
                            }
                        }
                    }
                }
            }
            if let (Value::Float(a, _), Value::Float(b, _)) = (l, r) {
                return float_binary(op, *a, FloatTy::F64, *b, FloatTy::F64);
            }
            if matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
                let eq = l.structural_eq(r);
                return Ok(Value::Bool(if op == BinaryOp::Eq { eq } else { !eq }));
            }
            return Ok(Value::Bool(cmp_result(op, l.structural_cmp(r)).unwrap()));
        }
        Err(unsupported(format!(
            "operator {:?} on {} and {}",
            op,
            l.type_name(),
            r.type_name()
        )))
    }

    pub(crate) fn negate(&mut self, v: Value) -> R {
        match super::place::deref_value(v)? {
            Value::Int(i, t) => {
                if t.fits(-i) {
                    Ok(Value::Int(-i, t))
                } else {
                    Err(panic_ctl("attempt to negate with overflow"))
                }
            }
            Value::Float(f, t) => Ok(Value::Float(-f, t)),
            v @ Value::Struct(_) | v @ Value::Enum(_) => {
                let tn = v.type_name();
                if let Some(f) = self.methods.get(&(tn, "neg".to_string())).cloned() {
                    return self.call_func(&f, vec![], Some(v));
                }
                Err(unsupported("unary minus on this type"))
            }
            other => Err(unsupported(format!("unary minus on {}", other.type_name()))),
        }
    }

    pub(crate) fn logical_not(&mut self, v: Value) -> R {
        match super::place::deref_value(v)? {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            Value::Int(i, t) => {
                let r = if t.signed() { !i } else { t.max() ^ i };
                Ok(Value::Int(r, t))
            }
            other => Err(unsupported(format!("`!` on {}", other.type_name()))),
        }
    }
}

//! Pure value operations for the tree-walking interpreter (no environment).

use super::value::Value;
use crate::parser::{BinaryOp, CompoundOp, Literal};

/// Apply a compound operation (+=, -=, etc.) without borrowing the interpreter
pub(crate) fn apply_compound_op_static(left: &Value, right: &Value, op: &CompoundOp) -> Value {
    let bin_op = match op {
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
    };
    if let (Some(a), Some(b)) = (left.as_int(), right.as_int()) {
        return match bin_op {
            BinaryOp::Add => Value::Int(a + b),
            BinaryOp::Sub => Value::Int(a - b),
            BinaryOp::Mul => Value::Int(a * b),
            BinaryOp::Div => Value::Int(if b != 0 { a / b } else { 0 }),
            BinaryOp::Mod => Value::Int(if b != 0 { a % b } else { 0 }),
            BinaryOp::BitAnd => Value::Int(a & b),
            BinaryOp::BitOr => Value::Int(a | b),
            BinaryOp::BitXor => Value::Int(a ^ b),
            BinaryOp::Shl => Value::Int(a << b),
            BinaryOp::Shr => Value::Int(a >> b),
            _ => Value::Nil,
        };
    }
    if let (Some(a), Some(b)) = (left.as_float(), right.as_float()) {
        return match bin_op {
            BinaryOp::Add => Value::Float(a + b),
            BinaryOp::Sub => Value::Float(a - b),
            BinaryOp::Mul => Value::Float(a * b),
            BinaryOp::Div => Value::Float(a / b),
            BinaryOp::Mod => Value::Float(a % b),
            _ => Value::Nil,
        };
    }
    Value::Nil
}

pub(crate) fn binary_op(left: &Value, right: &Value, op: &BinaryOp) -> Value {
    if let BinaryOp::Add = op {
        if let (Value::String(a), Value::String(b)) = (left, right) {
            return Value::String(format!("{}{}", a, b));
        }
    }

    if let (Some(a), Some(b)) = (left.as_int(), right.as_int()) {
        let either_float = matches!(left, Value::Float(_)) || matches!(right, Value::Float(_));
        if !either_float {
            return match op {
                BinaryOp::Add => Value::Int(a + b),
                BinaryOp::Sub => Value::Int(a - b),
                BinaryOp::Mul => Value::Int(a * b),
                BinaryOp::Div => {
                    if b == 0 {
                        Value::Int(0)
                    } else {
                        Value::Int(a / b)
                    }
                }
                BinaryOp::Mod => {
                    if b == 0 {
                        Value::Int(0)
                    } else {
                        Value::Int(a % b)
                    }
                }
                BinaryOp::Eq => Value::Bool(a == b),
                BinaryOp::Ne => Value::Bool(a != b),
                BinaryOp::Lt => Value::Bool(a < b),
                BinaryOp::Le => Value::Bool(a <= b),
                BinaryOp::Gt => Value::Bool(a > b),
                BinaryOp::Ge => Value::Bool(a >= b),
                BinaryOp::BitAnd => Value::Int(a & b),
                BinaryOp::BitOr => Value::Int(a | b),
                BinaryOp::BitXor => Value::Int(a ^ b),
                BinaryOp::Shl => Value::Int(a << b),
                BinaryOp::Shr => Value::Int(a >> b),
                _ => Value::Nil,
            };
        }
    }

    if let (Some(a), Some(b)) = (left.as_float(), right.as_float()) {
        return match op {
            BinaryOp::Add => Value::Float(a + b),
            BinaryOp::Sub => Value::Float(a - b),
            BinaryOp::Mul => Value::Float(a * b),
            BinaryOp::Div => Value::Float(a / b),
            BinaryOp::Mod => Value::Float(a % b),
            BinaryOp::Eq => Value::Bool(a == b),
            BinaryOp::Ne => Value::Bool(a != b),
            BinaryOp::Lt => Value::Bool(a < b),
            BinaryOp::Le => Value::Bool(a <= b),
            BinaryOp::Gt => Value::Bool(a > b),
            BinaryOp::Ge => Value::Bool(a >= b),
            _ => Value::Nil,
        };
    }

    if let (Value::Bool(a), Value::Bool(b)) = (left, right) {
        return match op {
            BinaryOp::Eq => Value::Bool(a == b),
            BinaryOp::Ne => Value::Bool(a != b),
            _ => Value::Nil,
        };
    }

    if let (Value::String(a), Value::String(b)) = (left, right) {
        return match op {
            BinaryOp::Eq => Value::Bool(a == b),
            BinaryOp::Ne => Value::Bool(a != b),
            BinaryOp::Lt => Value::Bool(a < b),
            BinaryOp::Le => Value::Bool(a <= b),
            BinaryOp::Gt => Value::Bool(a > b),
            BinaryOp::Ge => Value::Bool(a >= b),
            _ => Value::Nil,
        };
    }

    Value::Nil
}

pub(crate) fn literal_to_value(lit: &Literal) -> Value {
    match lit {
        Literal::Int(n) | Literal::IntSuffixed(n, _) => Value::Int(*n),
        Literal::Float(f) => Value::Float(*f),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Char(c) => Value::Char(*c),
    }
}

pub(crate) fn value_to_iterable(value: Value) -> Vec<Value> {
    match value {
        Value::Vec(items) => items,
        Value::String(s) => s.chars().map(Value::Char).collect(),
        _ => vec![],
    }
}

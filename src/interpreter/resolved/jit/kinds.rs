//! Static types of machine-code values and their bit encodings (`osr.rs`, `lower.rs`, `rt.rs`).
//!
//! An integer is 64 bits: the signed widths up to 64 bits, `u8`..`u32` and unsuffixed literals hold
//! their value sign-extended; `u64`/`usize` hold it as an unsigned pattern. Every result the walker
//! can produce in those widths fits, so machine code never needs a wider value (`i128`/`u128` and
//! `f32` stay in the closure engine). A float is an `f64` (an `f64`- or literal-typed one); a bool
//! is 0/1. Every value keeps the walker's width tag, statically.

use crate::interpreter::int::IntTy;
use crate::interpreter::value::{FloatTy, Value};
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ty {
    Int(IntTy),
    Flt(FloatTy),
    Bool,
}

/// What a local holds when a loop is entered (the speculation the machine code is built for).
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Class {
    Scalar(Ty),
    /// A `Vec` (possibly behind references), with what its first element looked like.
    Vec(Elem),
    /// A `HashMap` (possibly behind references), with what one of its values looked like (`lower_map.rs`).
    Map(Elem),
    /// A struct (possibly behind references): its type and the scalar fields it had.
    Struct(Shape),
    /// A struct held directly by the local (no reference) whose fields are all scalars: machine code
    /// keeps it as one variable per field (`lower_struct.rs`).
    Value(Shape),
    Opaque,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Elem {
    Scalar(Ty),
    Struct(Shape),
    Unknown,
}

/// A struct value's type name and, per field, the scalar type it held.
#[derive(Clone, Debug)]
pub(crate) struct Shape {
    pub(crate) name: Rc<str>,
    pub(crate) fields: Vec<(Rc<str>, Option<Ty>)>,
    pub(crate) copy: bool,
}

/// Two names are the same text; the values of one program share their name strings, so the pointer test
/// answers almost every comparison before the bytes are compared.
#[inline(always)]
pub(crate) fn same_name(a: &Rc<str>, b: &Rc<str>) -> bool {
    Rc::ptr_eq(a, b) || **a == **b
}

impl PartialEq for Shape {
    fn eq(&self, o: &Shape) -> bool {
        same_name(&self.name, &o.name)
            && self.copy == o.copy
            && self.fields.len() == o.fields.len()
            && self.fields.iter().zip(&o.fields).all(|((n, t), (m, u))| t == u && same_name(n, m))
    }
}

/// `shape(v) == Some(sh)`, without building the shape.
pub(crate) fn shape_is(v: &Value, sh: &Shape) -> bool {
    match v {
        Value::Struct(s) => {
            s.copy == sh.copy
                && s.fields.len() == sh.fields.len()
                && same_name(&s.name, &sh.name)
                && s.fields.iter().zip(&sh.fields).all(|((n, x), (m, t))| ty_of(x) == *t && same_name(n, m))
        }
        _ => false,
    }
}

/// `value_class(v).map(|k| k == *c)`, without building the class.
pub(crate) fn value_class_is(v: &Value, c: &Class) -> Option<bool> {
    match v {
        Value::Struct(s) if s.uid == 0 && s.fields.iter().all(|(_, x)| ty_of(x).is_some()) => Some(matches!(c, Class::Value(sh) if shape_is(v, sh))),
        _ => None,
    }
}

/// Is `c` what a local reaching `v` classifies as (`class_of`, with a scalar counted as `Opaque` as
/// `osr::classify` does)? Nothing is allocated.
pub(crate) fn class_of_is(v: &Value, c: &Class) -> bool {
    match v {
        Value::Map(m) if m.hashed => matches!(c, Class::Map(e) if *e == map_elem(m)),
        Value::Vec(items) => {
            let Class::Vec(e) = c else { return false };
            match items.get_value(0) {
                None => *e == Elem::Unknown,
                Some(x) => match (ty_of(&x), e) {
                    (Some(t), Elem::Scalar(u)) => t == *u,
                    (Some(_), _) => false,
                    (None, Elem::Struct(sh)) => shape_is(&x, sh),
                    (None, Elem::Unknown) => !matches!(x, Value::Struct(_)),
                    (None, Elem::Scalar(_)) => false,
                },
            }
        }
        v if ty_of(v).is_some() => *c == Class::Opaque,
        Value::Struct(_) => matches!(c, Class::Struct(sh) if shape_is(v, sh)),
        _ => *c == Class::Opaque,
    }
}

impl Shape {
    pub(crate) fn field(&self, name: &str) -> Option<(usize, Ty)> {
        let i = self.fields.iter().position(|(n, _)| &**n == name)?;
        Some((i, self.fields[i].1?))
    }
}

/// Integer widths machine code computes with.
pub(crate) fn int_ok(t: IntTy) -> bool {
    !matches!(t, IntTy::I128 | IntTy::U128)
}

/// `u64` / `usize`: held as an unsigned 64-bit pattern.
pub(crate) fn unsigned_repr(t: IntTy) -> bool {
    matches!(t, IntTy::U64 | IntTy::Usize)
}

/// Widths whose range is exactly `i64`'s (checked `i64` arithmetic is the walker's).
pub(crate) fn full_i64(t: IntTy) -> bool {
    matches!(t, IntTy::I64 | IntTy::Isize | IntTy::Lit)
}

const INTS: [IntTy; 13] = [
    IntTy::I8,
    IntTy::I16,
    IntTy::I32,
    IntTy::I64,
    IntTy::I128,
    IntTy::Isize,
    IntTy::U8,
    IntTy::U16,
    IntTy::U32,
    IntTy::U64,
    IntTy::U128,
    IntTy::Usize,
    IntTy::Lit,
];

/// A type as a small integer (passed to the runtime helpers).
pub(crate) fn code(t: Ty) -> i64 {
    match t {
        Ty::Int(i) => INTS.iter().position(|x| *x == i).expect("int width") as i64,
        Ty::Flt(FloatTy::F64) => 100,
        Ty::Flt(FloatTy::Lit) => 101,
        Ty::Flt(FloatTy::F32) => 102,
        Ty::Bool => 200,
    }
}

pub(crate) fn from_code(c: i64) -> Option<Ty> {
    Some(match c {
        100 => Ty::Flt(FloatTy::F64),
        101 => Ty::Flt(FloatTy::Lit),
        200 => Ty::Bool,
        c => Ty::Int(*INTS.get(usize::try_from(c).ok()?)?),
    })
}

/// The type of a scalar value machine code can hold, if it is one.
pub(crate) fn ty_of(v: &Value) -> Option<Ty> {
    match v {
        Value::Int(i, t) if int_ok(*t) && fits_repr(*i, *t) => Some(Ty::Int(*t)),
        Value::Float(_, t) if *t != FloatTy::F32 => Some(Ty::Flt(*t)),
        Value::Bool(_) => Some(Ty::Bool),
        _ => None,
    }
}

/// `t.is_none() || ty_of(v) == t`, without building the `Option<Ty>` (the row view check runs per field per build).
#[inline(always)]
pub(crate) fn ty_is(v: &Value, t: &Option<Ty>) -> bool {
    let Some(t) = t else { return true };
    let r = match (v, t) {
        (Value::Float(_, vt), Ty::Flt(t)) => vt == t && *t != FloatTy::F32,
        (Value::Int(i, vt), Ty::Int(t)) => vt == t && int_ok(*t) && fits_repr(*i, *t),
        (Value::Bool(_), Ty::Bool) => true,
        _ => false,
    };
    debug_assert_eq!(r, ty_of(v) == Some(*t));
    r
}

fn fits_repr(i: i128, t: IntTy) -> bool {
    if unsigned_repr(t) {
        (0..=u64::MAX as i128).contains(&i)
    } else {
        (i64::MIN as i128..=i64::MAX as i128).contains(&i)
    }
}

/// The bits of `v` if it is a scalar of type `t`.
pub(crate) fn bits(v: &Value, t: Ty) -> Option<i64> {
    match (v, t) {
        (Value::Int(i, vt), Ty::Int(t)) if *vt == t && fits_repr(*i, t) => Some(*i as i64),
        (Value::Float(f, vt), Ty::Flt(t)) if *vt == t => Some(f.to_bits() as i64),
        (Value::Bool(b), Ty::Bool) => Some(*b as i64),
        _ => None,
    }
}

/// The value `x` encodes at type `t`.
pub(crate) fn value(x: i64, t: Ty) -> Value {
    match t {
        Ty::Int(t) if unsigned_repr(t) => Value::Int(x as u64 as i128, t),
        Ty::Int(t) => Value::Int(x as i128, t),
        Ty::Flt(t) => Value::Float(f64::from_bits(x as u64), t),
        Ty::Bool => Value::Bool(x != 0),
    }
}

/// An integer as the walker holds it.
pub(crate) fn int_value(x: i64, t: IntTy) -> i128 {
    if unsigned_repr(t) {
        x as u64 as i128
    } else {
        x as i128
    }
}

/// The scalar type of one of a map's values (`Unknown`: empty, or not a scalar).
fn map_elem(m: &crate::interpreter::kmap::KMap) -> Elem {
    match m.any_value().and_then(ty_of) {
        Some(t) => Elem::Scalar(t),
        None => Elem::Unknown,
    }
}

fn shape(v: &Value) -> Option<Shape> {
    match v {
        Value::Struct(s) => Some(Shape { name: s.name.clone(), fields: s.fields.iter().map(|(n, v)| (n.clone(), ty_of(v))).collect(), copy: s.copy }),
        _ => None,
    }
}

/// A struct held directly, all of whose fields are scalars (and no destructor identity).
pub(crate) fn value_class(v: &Value) -> Option<Class> {
    match v {
        Value::Struct(s) if s.uid == 0 => {
            let sh = shape(v)?;
            sh.fields.iter().all(|(_, t)| t.is_some()).then_some(Class::Value(sh))
        }
        _ => None,
    }
}

/// The class of an aggregate reached from a local (references already followed).
pub(crate) fn class_of(v: &Value) -> Class {
    match v {
        Value::Map(m) if m.hashed => Class::Map(map_elem(m)),
        Value::Vec(items) => Class::Vec(match items.get_value(0) {
            None => Elem::Unknown,
            Some(x) => match (ty_of(&x), shape(&x)) {
                (Some(t), _) => Elem::Scalar(t),
                (_, Some(s)) => Elem::Struct(s),
                _ => Elem::Unknown,
            },
        }),
        v => match (ty_of(v), shape(v)) {
            (Some(t), _) => Class::Scalar(t),
            (_, Some(s)) => Class::Struct(s),
            _ => Class::Opaque,
        },
    }
}

/// Test-only mutants (debug builds, `WJ_JIT_MUTANT=<name>`): a check of the machine code is left
/// out so the differential tests can show they would catch its absence. Always off in release.
pub(crate) fn mutant(it: &crate::interpreter::engine::Interpreter, name: &str) -> bool {
    cfg!(debug_assertions) && it.jit_mutant.as_deref() == Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ty_is` is the row view's field check: `shape field type unset, or ty_of(value) == it`, for every pairing.
    #[test]
    fn ty_is_agrees_with_ty_of() {
        let values = [
            Value::Int(5, IntTy::I64),
            Value::Int(5, IntTy::I32),
            Value::Int(-1, IntTy::U64),
            Value::Int(i128::MAX, IntTy::I64),
            Value::Int(i128::from(u64::MAX) + 1, IntTy::U64),
            Value::Float(1.5, FloatTy::F64),
            Value::Float(f64::NAN, FloatTy::F64),
            Value::Float(1.5, FloatTy::F32),
            Value::Bool(true),
            Value::Unit,
        ];
        let tys = [None, Some(Ty::Int(IntTy::I64)), Some(Ty::Int(IntTy::I32)), Some(Ty::Int(IntTy::U64)), Some(Ty::Flt(FloatTy::F64)), Some(Ty::Flt(FloatTy::F32)), Some(Ty::Bool)];
        for v in &values {
            for t in &tys {
                assert_eq!(ty_is(v, t), t.is_none() || ty_of(v) == *t, "{v:?} against {t:?}");
            }
        }
    }
}
